// The overwrite question: which files of a batch would land on something already there, and
// its wording. No UI and no I/O here, so it is plain to test.
module sftpui

import os
import ssh

pub struct Collision {
pub:
	upload          string
	existing        string
	existing_is_dir bool
}

fn (c Collision) differs_in_case() bool {
	return c.upload != c.existing
}

fn upload_name(path string) string {
	name := os.file_name(path.trim_right('/\\'))
	return if name == '' { 'uploaded' } else { name }
}

// collisions returns the entries of the batch an upload would write over, one per name. The
// match is case-insensitive: a client cannot know the server's rule, and asking too often
// beats truncating a file on a case-folding server.
pub fn collisions(paths []string, existing []ssh.FileEntry) []Collision {
	mut found := []Collision{}
	for path in paths {
		upload := upload_name(path)
		if found.any(it.upload == upload) {
			continue
		}
		mut hit := -1
		for i, e in existing {
			if e.name == upload {
				hit = i
				break
			}
		}
		if hit < 0 {
			lower := upload.to_lower()
			for i, e in existing {
				if e.name.to_lower() == lower {
					hit = i
					break
				}
			}
		}
		if hit >= 0 {
			found << Collision{upload, existing[hit].name, existing[hit].is_dir}
		}
	}
	return found
}

// keep_after is what is left of the batch after the answer.
pub fn keep_after(paths []string, colliding []Collision, replace bool) []string {
	if replace {
		return paths
	}
	return paths.filter(upload_name(it) !in colliding.map(it.upload))
}

pub struct Prompt {
pub:
	title   string
	body    string
	replace string
	keep    string
}

// collision_prompt words the question; `keep` says Cancel only when nothing else would go up.
pub fn collision_prompt(colliding []Collision, others_remain bool) Prompt {
	keep := if others_remain { 'Skip' } else { 'Cancel' }
	if colliding.len == 1 {
		only := colliding[0]
		if only.existing_is_dir {
			spelling := if only.differs_in_case() {
				' You are uploading "${only.upload}", which may be the same folder on this server.'
			} else {
				''
			}
			return Prompt{'Replace Remote Folder', '"${only.existing}" already exists in the remote folder. The upload merges into it and overwrites the files inside that collide.${spelling} Continue?', 'Merge', keep}
		}
		spelling := if only.differs_in_case() {
			' You are uploading "${only.upload}", which may be the same file on this server.'
		} else {
			''
		}
		return Prompt{'Replace Remote File', '"${only.existing}" already exists in the remote folder.${spelling} Replace it?', 'Replace', keep}
	}
	mut names := []string{}
	for c in colliding {
		if c.existing !in names {
			names << c.existing
		}
	}
	mut listed := names[..int_min(5, names.len)].map('"${it}"').join(', ')
	if names.len > 5 {
		listed += ', and ${names.len - 5} more'
	}
	note := if colliding.any(it.differs_in_case()) {
		' Names that differ only in case may be the same file on this server.'
	} else {
		''
	}
	folders := if colliding.any(it.existing_is_dir) {
		' An existing folder is merged into, not emptied first.'
	} else {
		''
	}
	return Prompt{'Replace Remote Files', '${names.len} files already exist in the remote folder: ${listed}. Replace them?${note}${folders}', 'Replace all', keep}
}
