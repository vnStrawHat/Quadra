// The calls the browser makes on an SFTP session (specs/27). Implemented by ssh.Sftp and by
// the test fake; every method blocks, so the panel runs them on worker tasks.
module sftpui

import ssh

pub interface Client {
mut:
	read_dir(path string) ![]ssh.FileEntry
	stat(path string) !ssh.FileEntry
	realpath(path string) !string
	rename(from string, to string) !
	remove(path string) !
	remove_dir_all(path string) !
	mkdir(path string) !
	upload(id u64, local string, remote string, on_event ssh.OnTransfer) !
	download(id u64, remote string, local string, on_event ssh.OnTransfer) !
	cancel_transfer(id u64)
	alive() bool
}

// SftpClient adapts a real session. V rejects &ssh.Sftp as a Client directly: its check of the
// transfer callback's type fails across the module boundary.
pub struct SftpClient {
mut:
	s &ssh.Sftp = unsafe { nil }
}

pub fn (mut c SftpClient) read_dir(path string) ![]ssh.FileEntry {
	return c.s.read_dir(path)
}

pub fn (mut c SftpClient) stat(path string) !ssh.FileEntry {
	return c.s.stat(path)
}

pub fn (mut c SftpClient) realpath(path string) !string {
	return c.s.realpath(path)
}

pub fn (mut c SftpClient) rename(from string, to string) ! {
	c.s.rename(from, to)!
}

pub fn (mut c SftpClient) remove(path string) ! {
	c.s.remove(path)!
}

pub fn (mut c SftpClient) remove_dir_all(path string) ! {
	c.s.remove_dir_all(path)!
}

pub fn (mut c SftpClient) mkdir(path string) ! {
	c.s.mkdir(path)!
}

pub fn (mut c SftpClient) upload(id u64, local string, remote string, on_event ssh.OnTransfer) ! {
	c.s.upload(id, local, remote, on_event)!
}

pub fn (mut c SftpClient) download(id u64, remote string, local string, on_event ssh.OnTransfer) ! {
	c.s.download(id, remote, local, on_event)!
}

pub fn (mut c SftpClient) cancel_transfer(id u64) {
	c.s.cancel_transfer(id)
}

pub fn (mut c SftpClient) alive() bool {
	return c.s.alive()
}

// Job is a worker task's data: what the work function needs, passed in instead of captured
// (no closures in apps, specs/14). Each task fills the fields it uses.
struct Job {
	client   Client
	bus      &Bus = unsafe { nil }
	key      u64
	gen      u64
	id       u64
	path     string
	cwd      string
	name     string
	remote   string
	paths    []string
	entry    ssh.FileEntry
	rename   bool
	baseline ?i64
}
