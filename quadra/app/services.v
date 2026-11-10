// App services entry point: directories, log, crash dumps, i18n, single instance.
module app

import quadra.core

pub const already_running = 3001 // error code of init when another instance took over

pub struct AppInfo {
pub:
	id              string            // folder and instance name: 'quadra-notes'
	name            string            // display name
	root            string            // keep every file under this folder (tests, dev runs, portable)
	locales         map[string]string // locale code -> JSON string table
	locale          string = 'en'
	fallback        string = 'en'
	single_instance bool
	args            []string // forwarded to the running instance when single_instance
	log_level       Level = .info
}

@[heap]
pub struct Services {
pub:
	info AppInfo
	dirs Dirs
pub mut:
	log    &Logger
	assets Assets
}

pub struct ServicesRef {
	s &Services
}

// init prepares the services and stores them in the app (read them with services(cx)).
// With single_instance, a second launch forwards its args and init fails with
// already_running: the caller should exit quietly.
pub fn init(mut a core.App, info AppInfo) !&Services {
	if info.single_instance && !a.plat.claim_single_instance(info.id, info.args) {
		return error_with_code('${info.id} is already running', already_running)
	}
	d := dirs(info.id, info.root)!
	mut log := new_logger(d.logs, info.id, info.log_level)
	// Made once per process (the crash handler lives as long as the process): no leak.
	a.plat.install_crash_handler(d.crashes, fn [mut log] () string {
		return log.recent(100).map(it.str()).join('\n')
	})
	mut cx := core.Cx{
		app: a
	}
	if info.locales.len > 0 {
		install_i18n(mut cx, info.locales, info.locale, info.fallback, log)!
	}
	s := &Services{
		info: info
		dirs: d
		log:  log
	}
	cx.set_global(ServicesRef{s})
	log.info('app', 'started ${info.id}')
	return s
}

// services returns what init prepared; it panics when init was not called (a setup bug).
pub fn services(cx &core.Cx) &Services {
	r := cx.peek_global[ServicesRef]() or { panic('quadra.app: call app.init first') }
	return r.s
}
