// SFTP version 3 messages and attributes (draft-ietf-secsh-filexfer-02).
module ssh

const fx_init = u8(1)
const fx_version = u8(2)
const fx_open = u8(3)
const fx_close = u8(4)
const fx_read = u8(5)
const fx_write = u8(6)
const fx_lstat = u8(7)
const fx_setstat = u8(9)
const fx_opendir = u8(11)
const fx_readdir = u8(12)
const fx_remove = u8(13)
const fx_mkdir = u8(14)
const fx_rmdir = u8(15)
const fx_realpath = u8(16)
const fx_stat = u8(17)
const fx_rename = u8(18)
const fx_status = u8(101)
const fx_handle = u8(102)
const fx_data = u8(103)
const fx_name = u8(104)
const fx_attrs = u8(105)
const fx_extended = u8(200)
const fx_extended_reply = u8(201)

const fxf_read = u32(1)
const fxf_write = u32(2)
const fxf_creat = u32(8)
const fxf_trunc = u32(16)

const attr_size = u32(1)
const attr_uidgid = u32(2)
const attr_perm = u32(4)
const attr_times = u32(8)
const attr_extended = u32(0x80000000)

const mode_type_mask = u32(0o170000)
const mode_dir = u32(0o040000)
const mode_link = u32(0o120000)

// Attrs is an SFTP attribute block; flags say which fields are present.
struct Attrs {
mut:
	flags u32
	size  u64
	uid   u32
	gid   u32
	perms u32
	atime u32
	mtime u32
}

fn (a Attrs) has(flag u32) bool {
	return a.flags & flag != 0
}

fn (a Attrs) is_dir() bool {
	return a.has(attr_perm) && a.perms & mode_type_mask == mode_dir
}

fn (a Attrs) is_symlink() bool {
	return a.has(attr_perm) && a.perms & mode_type_mask == mode_link
}

fn parse_attrs(mut r Reader) !Attrs {
	mut a := Attrs{
		flags: r.get_u32()!
	}
	if a.has(attr_size) {
		a.size = r.get_u64()!
	}
	if a.has(attr_uidgid) {
		a.uid = r.get_u32()!
		a.gid = r.get_u32()!
	}
	if a.has(attr_perm) {
		a.perms = r.get_u32()!
	}
	if a.has(attr_times) {
		a.atime = r.get_u32()!
		a.mtime = r.get_u32()!
	}
	if a.has(attr_extended) {
		n := r.get_u32()!
		for _ in 0 .. n {
			r.get_bytes()!
			r.get_bytes()!
		}
	}
	return a
}

fn put_attrs(mut w Writer, a Attrs) {
	w.put_u32(a.flags)
	if a.has(attr_size) {
		w.put_u64(a.size)
	}
	if a.has(attr_uidgid) {
		w.put_u32(a.uid)
		w.put_u32(a.gid)
	}
	if a.has(attr_perm) {
		w.put_u32(a.perms)
	}
	if a.has(attr_times) {
		w.put_u32(a.atime)
		w.put_u32(a.mtime)
	}
}

// status_error maps an SSH_FXP_STATUS body to nothing (OK) or a typed error.
fn status_error(body []u8) ! {
	mut r := new_reader(body[5..])
	code := r.get_u32()!
	if code == 0 {
		return
	}
	message := r.get_str() or { '' }
	return SftpError{
		status:  match code {
			1 { SftpStatus.eof }
			2 { SftpStatus.no_such_file }
			3 { SftpStatus.permission_denied }
			4 { SftpStatus.failure }
			5 { SftpStatus.bad_message }
			6, 7 { SftpStatus.connection_lost }
			8 { SftpStatus.op_unsupported }
			else { SftpStatus.other }
		}
		message: message
	}
}
