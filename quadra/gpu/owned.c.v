// Owned uploads (Upload.owned): pixels in the C heap that the upload takes over.
module gpu

// free_owned gives an owned upload's pixels back to the C heap once the device has copied
// them; other uploads are left alone. Call it once per upload, after Device.upload.
pub fn free_owned(u Upload) {
	if u.owned && u.pixels.len > 0 {
		unsafe { C.free(u.pixels.data) }
	}
}
