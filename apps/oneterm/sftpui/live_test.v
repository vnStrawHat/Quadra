// One round trip through the real engine against sshd on 127.0.0.1 (specs/27).
// Runs only under apps/oneterm/ssh/interop.sh, which starts the server inside .tmp.
module sftpui

import os
import ssh
import quadra.core
import quadra.kit
import quadra.testing
import quadra.ui

@[heap; markused]
pub struct LiveRoot {
	panel core.View[SftpPanel]
}

pub fn (mut r LiveRoot) render(mut cx core.ViewCx[LiveRoot]) core.Element {
	return ui.div().size_full().child(core.view_element(r.panel.any()))
}

fn wait_done(mut h testing.Harness) ! {
	for _ in 0 .. 30 {
		h.wait_tasks() or { continue }
		return
	}
	return error('still running')
}

fn known_hosts_for(dir string, port int) !string {
	path := os.join_path(dir, 'known_sftpui')
	mut text := ''
	for k in ['ed25519', 'ecdsa', 'rsa'] {
		f := os.read_file(os.join_path(dir, 'hk_${k}.pub'))!.fields()
		text += '[127.0.0.1]:${port} ${f[0]} ${f[1]}\n'
	}
	os.write_file(path, text)!
	return path
}

fn test_round_trip_against_sshd() {
	if os.getenv('QUADRA_SSHD_TESTS') != '1' {
		eprintln('skipped: run under apps/oneterm/ssh/interop.sh')
		return
	}
	dir := os.getenv('SSHD_DIR')
	port := os.getenv('SSHD_PORT').int()
	scratch := os.join_path(dir, 'sftpui_live')
	os.rmdir_all(scratch) or {}
	os.mkdir_all(os.join_path(scratch, 'remote'))!
	os.mkdir_all(os.join_path(scratch, 'local'))!
	local := os.join_path(scratch, 'local', 'hello.txt')
	os.write_file(local, 'hello over sftp')!
	mut sess := ssh.connect(ssh.Config{
		target:           ssh.Hop{
			host:     '127.0.0.1'
			port:     port
			username: os.getenv('USERNAME')
			auth:     ssh.AuthMethod(ssh.KeyAuth{
				key_path: os.join_path(dir, 'id_ed25519')
			})
		}
		known_hosts_path: known_hosts_for(dir, port)!
		agent_pipe:       r'\\.\pipe\no-such-agent-sftpui'
	}) or {
		eprintln('skipped: sshd unavailable: ${err}')
		return
	}
	defer {
		sess.close()
	}
	sftp := sess.sftp() or { panic('no sftp channel') }
	mut h := testing.new_harness(width: 900, height: 600)!
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	kit.install(mut h.app)!
	v := new_panel(mut cx,
		edit_dir:  os.join_path(scratch, 'edit')
		local_dir: os.join_path(scratch, 'local')
	)
	h.open(LiveRoot{ panel: v },
		width:  900
		height: 600
	)!
	set_session(mut cx, v, sftp)
	wait_done(mut h)!
	p := cx.peek(v.model())
	assert p.active != unsafe { nil }
	assert p.active.cwd.starts_with('/') // the home folder resolved through realpath
	// The remote side of the round trip stays inside the project's scratch folder.
	win := os.join_path(scratch, 'remote').replace('\\', '/')
	remote_root := '/${win[0..1].to_lower()}${win[2..]}'
	cx.update_view(v, fn [remote_root] (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.goto_path(remote_root, mut cx)
	})
	wait_done(mut h)!
	assert cx.peek(v.model()).active.cwd == remote_root
	cx.update_view(v, fn [local] (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.upload_paths([local], mut cx)
	})
	wait_done(mut h)!
	wait_done(mut h)!
	assert h.has_text('hello.txt')
	assert h.has_text('Done')
	uploaded := os.join_path(scratch, 'remote', 'hello.txt')
	assert os.read_file(uploaded)! == 'hello over sftp'
	// Download through the panel, into a local file.
	back := os.join_path(scratch, 'local', 'back.txt')
	entry := ssh.FileEntry{
		name: 'hello.txt'
		path: remote_root + '/hello.txt'
	}
	cx.update_view(v, fn [entry, back] (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.download_to(entry, back, mut cx)
	})
	wait_done(mut h)!
	assert os.read_file(back)! == 'hello over sftp'
	// Delete from the panel.
	mut panel := cx.peek(v.model())
	panel.active.selected = 0
	cx.update_view(v, fn (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.do_delete(mut cx)
	})
	h.frame()
	h.click('ask-yes')!
	wait_done(mut h)!
	wait_done(mut h)!
	assert !os.exists(uploaded)
	assert cx.peek(v.model()).active.entries.len == 0
}
