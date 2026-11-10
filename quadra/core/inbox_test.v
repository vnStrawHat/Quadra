module core

fn test_a_drained_buffer_is_kept_unless_a_burst_grew_it() {
	assert keeps_buffer(64 << 10, 10, false) // small: always kept
	assert keeps_buffer(2 << 20, 1 << 20, false) // still streaming
	assert !keeps_buffer(2 << 20, 100, false) // the burst is over
	assert !keeps_buffer(8 << 20, 8 << 20, false) // too big to keep
	assert !keeps_buffer(64 << 10, 10, true) // the producer exited
}
