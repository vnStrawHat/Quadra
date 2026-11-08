module win

fn test_guid_parses_sdk_format() {
	g := guid('54ec77fa-1377-44e6-8c32-88fd5f44c84c')
	assert g.d1 == 0x54ec77fa
	assert g.d2 == 0x1377
	assert g.d3 == 0x44e6
	assert g.d4 == [u8(0x8c), 0x32, 0x88, 0xfd, 0x5f, 0x44, 0xc8, 0x4c]!
	assert sizeof(Guid) == 16
}

fn test_guid_equals() {
	a := guid('00000000-0000-0000-c000-000000000046')
	assert a.equals(&iid_iunknown)
	assert !a.equals(&Guid{})
}
