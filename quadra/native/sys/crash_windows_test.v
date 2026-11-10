module sys

import os
import rand

fn test_fixed_buffer_formats_signed_and_hex_values_without_allocation() {
	mut b := FixedBuffer{}
	b.push_i64(-42)
	b.push_str(' ')
	b.push_hex(0xc0000005)
	b.push_str(' ')
	b.push_u64(0)
	assert unsafe { tos(&b.bytes[0], b.len) } == '-42 0xC0000005 0'
}

fn test_fixed_buffer_is_bounded() {
	mut b := FixedBuffer{}
	for _ in 0 .. 200 {
		b.push_str('0123456789')
	}
	assert b.len == 512
}

fn test_simulated_native_exception_writes_a_staging_report() {
	dir := os.join_path(@VMODROOT, '.tmp', 'native-sys-tests', rand.hex(8))
	os.mkdir_all(dir)!
	path := os.join_path(dir, 'staging.tmp')
	prepare_native_state(open_crash_staging(path)!, 'Version: 1.2.3\n')!
	record := NativeExceptionRecord{
		code:    0xC0000005
		address: voidptr(0x1234)
	}
	pointers := NativeExceptionPointers{
		record: &record
	}
	assert native_filter(voidptr(&pointers)) == 0
	close_handle(native_state.handle)
	report := os.read_file(path)!
	assert report.contains('Version: 1.2.3')
	assert report.contains('Exception code: 0xC0000005')
	assert report.contains('Exception address: 0x1234')
}
