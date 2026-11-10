// More live tests against sshd: SFTP, forwards, jump hosts, the agent and keepalive.
module ssh

import net
import os
import rand
import time

fn test_sftp_round_trip() {
	dir, port := gate() or { return }
	mut sess, mut c := sftp_for(dir, port, 'sftp') or { panic(err) }
	defer {
		sess.close()
	}
	root := remote_area('rt')
	c.remove_dir_all(root) or {}
	c.mkdir(root) or { panic(err) }
	mut data := rand.bytes(3 * 1024 * 1024 + 123) or { panic(err) }
	data << 'tail'.bytes()
	local_up := os.join_path(dir, 'up.bin')
	local_down := os.join_path(dir, 'down.bin')
	os.write_file_array(local_up, data) or { panic(err) }
	mut log := &EventLog{}
	c.upload(1, local_up, '${root}/a.bin', OnTransfer{
		ctx: log
		f:   fn (ctx voidptr, e TransferEvent) {
			mut log := unsafe { &EventLog(ctx) }
			log.events << e
		}
	}) or { panic(err) }
	assert log.events.last().fraction == 1.0
	for i in 1 .. log.events.len {
		assert log.events[i].fraction >= log.events[i - 1].fraction
	}
	st := c.stat('${root}/a.bin') or { panic(err) }
	assert st.size == u64(data.len) && !st.is_dir
	c.download(2, '${root}/a.bin', local_down, OnTransfer{}) or { panic(err) }
	downloaded := os.read_bytes(local_down) or { panic(err) }
	assert downloaded == data
	// overwrite through the backup path, and rename
	c.upload(3, local_up, '${root}/a.bin', OnTransfer{}) or { panic(err) }
	c.rename('${root}/a.bin', '${root}/b.bin') or { panic(err) }
	c.mkdir('${root}/Sub') or { panic(err) }
	names := c.read_dir(root) or { panic(err) }
	assert names.len == 2
	assert names[0].name == 'Sub' && names[0].is_dir
	assert names[1].name == 'b.bin' && names[1].size == u64(data.len)
	real := c.realpath(root) or { panic(err) }
	assert real.ends_with('/sftp/rt')
	// a missing file is typed
	c.stat('${root}/missing') or {
		assert err is SftpError && (err as SftpError).status == .no_such_file, err.msg()
		c.remove('${root}/b.bin') or { panic(err) }
		c.remove_dir_all(root) or { panic(err) }
		c.stat(root) or { return }
		assert false, 'tree must be gone'
		return
	}
	assert false
}

fn test_sftp_large_transfer_across_rekeys_and_folders() {
	dir, port := gate() or { return }
	mut sess, mut c := sftp_for(dir, port, 'bulk') or { panic(err) }
	defer {
		sess.close()
	}
	root := remote_area('bulk')
	c.remove_dir_all(root) or {}
	// 20 MiB crosses the server's RekeyLimit (8 MiB) in both directions
	data := rand.bytes(20 * 1024 * 1024) or { panic(err) }
	tree := os.join_path(dir, 'tree')
	os.rmdir_all(tree) or {}
	os.mkdir_all(os.join_path(tree, 'nest', 'deep')) or { panic(err) }
	os.write_file_array(os.join_path(tree, 'nest', 'deep', 'big.bin'), data) or { panic(err) }
	os.write_file(os.join_path(tree, 'small.txt'), 'hi') or { panic(err) }
	mut seen := &EventLog{}
	c.upload(10, tree, root, OnTransfer{}) or { panic(err) }
	back := os.join_path(dir, 'tree_back')
	os.rmdir_all(back) or {}
	c.download(11, root, back, OnTransfer{
		ctx: seen
		f:   fn (ctx voidptr, e TransferEvent) {
			mut seen := unsafe { &EventLog(ctx) }
			if e.kind == .discovering {
				seen.files = e.files
			}
		}
	}) or { panic(err) }
	assert seen.files == 2
	big_back := os.read_bytes(os.join_path(back, 'nest', 'deep', 'big.bin')) or { panic(err) }
	assert big_back == data
	assert (os.read_file(os.join_path(back, 'small.txt')) or { panic(err) }) == 'hi'
	c.remove_dir_all(root) or { panic(err) }
}

fn test_sftp_cancel_and_duplicate_id() {
	dir, port := gate() or { return }
	mut sess, mut c := sftp_for(dir, port, 'cancel') or { panic(err) }
	defer {
		sess.close()
	}
	root := remote_area('cx')
	c.remove_dir_all(root) or {}
	c.mkdir(root) or { panic(err) }
	big := os.join_path(dir, 'big_cancel.bin')
	os.write_file_array(big, rand.bytes(16 * 1024 * 1024) or { panic(err) }) or { panic(err) }
	th := spawn upload_expect_cancel(mut c, big, '${root}/x.bin')
	time.sleep(30 * time.millisecond)
	c.upload(77, big, '${root}/y.bin', OnTransfer{}) or {
		assert err.msg().contains('duplicate'), err.msg()
	}
	c.cancel_transfer(77)
	assert th.wait()
	// no partial file left behind
	entries := c.read_dir(root) or { panic(err) }
	assert entries.len == 0, '${entries}'
	c.remove_dir_all(root) or { panic(err) }
}

fn upload_expect_cancel(mut c Sftp, local string, remote string) bool {
	c.upload(77, local, remote, OnTransfer{}) or { return err is Cancelled }
	return false
}

fn echo_server() (&net.TcpListener, int) {
	mut l := net.listen_tcp(.ip, '127.0.0.1:0') or { panic(err) }
	addr := l.addr() or { panic(err) }
	spawn echo_loop(mut l)
	return l, int(addr.port() or { 0 })
}

fn echo_loop(mut l net.TcpListener) {
	for {
		mut c := l.accept() or { return }
		spawn fn [mut c] () {
			c.set_read_timeout(5 * time.second)
			mut buf := []u8{len: 4096}
			for {
				n := c.read(mut buf) or { break }
				c.write(buf[..n]) or { break }
			}
			c.close() or {}
		}()
	}
}

fn expect_echo(mut c net.TcpConn) {
	c.set_read_timeout(5 * time.second)
	c.write('forwarded!'.bytes()) or { panic(err) }
	mut buf := []u8{len: 64}
	mut got := []u8{}
	for got.len < 10 {
		n := c.read(mut buf) or { panic(err) }
		got << buf[..n]
	}
	assert got.bytestr() == 'forwarded!'
	c.close() or {}
}

fn test_forwards_local_dynamic_remote() {
	dir, port := gate() or { return }
	mut l, echo_port := echo_server()
	defer {
		l.close() or {}
	}
	mut cfg := base_config(dir, port, key_auth(dir, 'id_ed25519', ''), 'fwd')
	cfg.port_forwards = [
		PortForward(LocalForward{
			bind_port:   0
			target_host: '127.0.0.1'
			target_port: echo_port
		}),
		PortForward(DynamicForward{
			bind_port: 0
		}),
		PortForward(RemoteForward{
			bind_host:   '127.0.0.1'
			bind_port:   0
			target_host: '127.0.0.1'
			target_port: echo_port
		}),
	]
	mut warned := &EventLog{}
	cfg.warning_ctx = warned
	cfg.on_warning = fn (ctx voidptr, m string) {
		mut warned := unsafe { &EventLog(ctx) }
		warned.warnings << m
	}
	mut s := connect(cfg) or { panic(err) }
	defer {
		s.close()
	}
	assert warned.warnings.len == 0, '${warned.warnings}'
	ports := s.bound_ports()
	assert ports.len == 2
	mut local := net.dial_tcp('127.0.0.1:${ports[0]}') or { panic(err) }
	expect_echo(mut local)
	// SOCKS5 CONNECT to the echo server, resolved on the far side
	mut socks := net.dial_tcp('127.0.0.1:${ports[1]}') or { panic(err) }
	socks.set_read_timeout(5 * time.second)
	socks.write([u8(5), 1, 0]) or { panic(err) }
	mut buf := []u8{len: 16}
	greeting := socks.read(mut buf) or { panic(err) }
	assert greeting == 2 && buf[1] == 0
	mut req := [u8(5), 1, 0, 3, 9]
	req << '127.0.0.1'.bytes()
	req << u8(echo_port >> 8)
	req << u8(echo_port)
	socks.write(req) or { panic(err) }
	mut reply := []u8{len: 10}
	mut got := 0
	for got < 10 {
		n := socks.read(mut reply[got..]) or { panic(err) }
		got += n
	}
	assert reply[1] == 0
	expect_echo(mut socks)
	// the remote forward: the server picked a port, reach it from the server side
	mut remote_port := 0
	{
		mut f := s.forwards
		f.mu.@lock()
		for key, _ in f.remote {
			remote_port = key.all_after_last(':').int()
		}
		f.mu.unlock()
	}
	assert remote_port > 0
	mut peer_ch := s.conn.open_direct_tcpip('127.0.0.1', remote_port, '127.0.0.1', 0) or {
		panic(err)
	}
	peer_ch.write('forwarded!'.bytes()) or { panic(err) }
	mut peer_buf := []u8{len: 10}
	peer_ch.read_full(mut peer_buf) or { panic(err) }
	assert peer_buf.bytestr() == 'forwarded!'
	peer_ch.close()
}

fn test_jump_host_route() {
	dir, port := gate() or { return }
	mut cfg := base_config(dir, port, key_auth(dir, 'id_ed25519', ''), 'jump')
	cfg.jump_hops = [hop_for(dir, port, key_auth(dir, 'id_ed25519', ''))]
	mut s := connect(cfg) or { panic(err) }
	out, _ := run_cmd(mut s.conn, 'echo through-the-jump') or { panic(err) }
	assert out.contains('through-the-jump')
	s.close()
}

fn test_keepalive_reaches_sshd() {
	dir, port := gate() or { return }
	mut cfg := base_config(dir, port, key_auth(dir, 'id_ed25519', ''), 'ka')
	cfg.keepalive = Keepalive{
		enabled: true
		seconds: 5
		max:     3
	}
	mut s := connect(cfg) or { panic(err) }
	time.sleep(11 * time.second) // the idle check fires on the second tick
	s.close()
	time.sleep(200 * time.millisecond)
	log := os.read_file(os.join_path(dir, 'sshd.log')) or { panic(err) }
	assert log.contains('rtype keepalive@openssh.com')
}
