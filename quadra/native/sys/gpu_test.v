module sys

fn test_gpu_usage_is_this_pids_busiest_single_engine() {
	names := ['pid_42_luid_0x00000000_0x0000BEEF_phys_0_eng_0_engtype_3D',
		'pid_42_luid_0x00000000_0x0000BEEF_phys_0_eng_1_engtype_3D',
		'pid_42_luid_0x00000000_0x0000BEEF_phys_0_eng_2_engtype_Copy',
		'pid_421_luid_0x00000000_0x0000BEEF_phys_0_eng_0_engtype_3D',
		'pid_7_luid_0x00000000_0x0000BEEF_phys_0_eng_0_engtype_VideoDecode']
	values := [1.25, 0.5, 1.0, 90.0, 50.0]
	// The busiest engine, not the sum of an engine type (1.75) nor of everything (2.75);
	// pid 421 shares the leading digits and is not counted.
	assert busiest_engine(42, names, values)? == f32(1.25)
	// A process with no engines has no reading, not zero.
	assert busiest_engine(9, names, values) == none
	two := ['pid_1_luid_0x0_0x1_phys_0_eng_3_engtype_VideoDecode',
		'pid_1_luid_0x0_0x1_phys_0_eng_11_engtype_VideoDecode']
	assert busiest_engine(1, two, [40.0, 30.0])? == f32(40)
	// A counter overshoot is clamped to 100 %.
	assert busiest_engine(1, ['pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D'], [130.0])? == f32(100)
}

fn test_open_gpu_usage_samples_or_says_none() {
	mut g := open_gpu_usage() or { return } // no counter: other OSes, a locked-down PDH
	defer {
		g.close()
	}
	if v := g.sample() {
		assert v >= 0 && v <= 100
	}
	g.close()
	assert g.sample() == none
}
