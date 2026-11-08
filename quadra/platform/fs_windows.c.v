module platform

fn C.MoveFileExW(src &u16, dst &u16, flags u32) int

const movefile_replace_existing = u32(0x1)
const movefile_write_through = u32(0x8)

// replace_file atomically moves src over dst (dst may exist): the last step of a safe save.
pub fn replace_file(src string, dst string) ! {
	ok := C.MoveFileExW(src.replace('/', r'\').to_wide(), dst.replace('/', r'\').to_wide(),
		movefile_replace_existing | movefile_write_through)
	if ok == 0 {
		return error('cannot replace ${dst} with ${src} (error ${C.GetLastError()})')
	}
}
