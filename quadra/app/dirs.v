// Per-app directories for settings, data, cache, logs and crash dumps (specs/10).
module app

import os

pub struct Dirs {
pub:
	config  string
	data    string
	cache   string
	logs    string
	crashes string
}

// dirs returns the app's directories, creating them. With root set (tests, portable
// installs, development runs) everything lives under root; otherwise under the user's
// config and cache directories, in a folder named after the app id.
pub fn dirs(app_id string, root string) !Dirs {
	base := if root != '' { root } else { os.join_path(os.config_dir()!, app_id) }
	cache := if root != '' {
		os.join_path(root, 'cache')
	} else {
		os.join_path(os.cache_dir(), app_id)
	}
	d := Dirs{
		config:  os.join_path(base, 'config')
		data:    os.join_path(base, 'data')
		cache:   cache
		logs:    os.join_path(base, 'logs')
		crashes: os.join_path(base, 'crashes')
	}
	for p in [d.config, d.data, d.cache, d.logs, d.crashes] {
		os.mkdir_all(p)!
	}
	return d
}
