// Quadra Cluster: a cluster dashboard on fake data, the reference app of the developer-tool
// kit (specs/16).
module main

import os
import quadra.core
import quadra.kit
import quadra.native
import quadra.platform

fn main() {
	mut app := core.new_app(native.backends(force_warp: '--warp' in os.args)!, core.AppOptions{})!
	kit.install(mut app)!
	kit.follow_system(mut app, kit.light(), kit.dark())
	app.open_window(platform.WindowOptions{
		title:           'Quadra Cluster'
		custom_titlebar: true
		width:           1280
		height:          800
		min_width:       800
		min_height:      500
	}, Cluster{})!
	app.run()
}
