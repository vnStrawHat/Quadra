// The FPS HUD's rows (specs/22).
module main

import quadra.native.sys

fn test_the_rows_are_fps_gpu_api_and_usage_in_that_order() {
	mut g := HudGpu{
		tried:   true
		probe:   &sys.GpuUsage{}
		percent: 1.84
	}
	rows := hud_rows(2, device_text('Intel(R) UHD Graphics 770', false), g.usage_text())
	assert rows.map('${it[0]} ${it[1]}') == ['FPS 2', 'GPU Intel(R) UHD Graphics 770',
		'API Direct3D 11', 'GPU usage 1.8%']
	assert hud_rows(-1, 'n/a', '-')[0] == ['FPS', '-'] // before the first whole second
	assert device_text('Microsoft Basic Render Driver', true) == 'Microsoft Basic Render Driver (software)'
	assert device_text('', false) == 'n/a'
	// Open but no sample yet, or still opening: `-`.
	g.percent = -1
	assert g.usage_text() == '-'
	assert HudGpu{
		tried: true
		busy:  true
	}.usage_text() == '-'
	// The query could not open: `n/a`.
	assert HudGpu{
		tried: true
	}.usage_text() == 'n/a'
	assert HudGpu{
		tried:   true
		probe:   &sys.GpuUsage{}
		percent: 100
	}.usage_text() == '100.0%'
}
