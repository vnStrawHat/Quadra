module platform

import os

// replace_file atomically moves src over dst (rename(2) replaces an existing file).
pub fn replace_file(src string, dst string) ! {
	os.rename(src, dst)!
}
