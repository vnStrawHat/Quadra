// Key material shared by every OS: RSA magnitudes and the NIST curve table.
module cng

// RsaParts are the magnitudes of an RSA key; d, p and q are empty for a public key.
pub struct RsaParts {
pub mut:
	n []u8
	e []u8
	p []u8
	q []u8
}

// Curve describes one NIST curve: its CNG names, coordinate size and blob magics.
pub struct Curve {
pub:
	ssh_id    string // nistp256
	ecdsa_alg string
	ecdh_alg  string
	size      int
	bits      u32
	ecs1      u32 // ECDSA public
	ecs2      u32 // ECDSA private
	eck1      u32 // ECDH public
	eck2      u32 // ECDH private
}

const curves = [
	Curve{'nistp256', 'ECDSA_P256', 'ECDH_P256', 32, 256, 0x31534345, 0x32534345, 0x314B4345, 0x324B4345},
	Curve{'nistp384', 'ECDSA_P384', 'ECDH_P384', 48, 384, 0x33534345, 0x34534345, 0x334B4345, 0x344B4345},
	Curve{'nistp521', 'ECDSA_P521', 'ECDH_P521', 66, 521, 0x35534345, 0x36534345, 0x354B4345, 0x364B4345},
]

pub fn curve_by_name(id string) !Curve {
	for c in curves {
		if c.ssh_id == id {
			return c
		}
	}
	return error('unsupported curve ${id}')
}

// strip_zeros drops the leading zero bytes of a big-endian magnitude.
pub fn strip_zeros(b []u8) []u8 {
	mut i := 0
	for i < b.len && b[i] == 0 {
		i++
	}
	return b[i..]
}
