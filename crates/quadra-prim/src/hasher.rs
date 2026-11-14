//! A fast hasher for small keys the program makes itself (glyph keys, faces). The standard hasher
//! resists keys chosen by an attacker, which costs most of a glyph lookup; these keys need no
//! such thing.

use std::hash::{BuildHasherDefault, Hasher};

/// Multiply and rotate, a word at a time.
#[derive(Clone, Copy, Default)]
pub struct FastHasher(u64);

/// For `HashMap::with_hasher` and as the hasher parameter of a map type.
pub type FastBuild = BuildHasherDefault<FastHasher>;

impl FastHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

impl Hasher for FastHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.add(u64::from_le_bytes(word));
        }
    }

    fn write_u8(&mut self, v: u8) {
        self.add(v as u64);
    }

    fn write_u16(&mut self, v: u16) {
        self.add(v as u64);
    }

    fn write_u32(&mut self, v: u32) {
        self.add(v as u64);
    }

    fn write_u64(&mut self, v: u64) {
        self.add(v);
    }

    fn write_usize(&mut self, v: usize) {
        self.add(v as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{BuildHasher, Hash};

    #[test]
    fn close_keys_hash_apart() {
        let hash = |key: (u32, u16, u32, u8)| FastBuild::default().hash_one(key);
        let mut seen = std::collections::HashSet::new();
        for face in 0..4 {
            for glyph in 0..500 {
                for phase in 0..4 {
                    assert!(seen.insert(hash((face, glyph, 14.0f32.to_bits(), phase))));
                }
            }
        }
        let bytes = |s: &str| {
            let mut h = FastHasher::default();
            s.hash(&mut h);
            h.finish()
        };
        assert_ne!(bytes("row 1"), bytes("row 2"));
    }
}
