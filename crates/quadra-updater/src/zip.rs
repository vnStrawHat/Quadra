//! A zip unpacked (specs/22, "Fetching"): its files, stored or deflated, written under one
//! folder. Read from memory by its central directory; inflating is `miniz_oxide`'s (specs/01,
//! R15), the rest is a few headers. What would land outside the folder, a link, an encrypted
//! entry or one whose bytes are not what its checksum says is refused, and nothing is trusted
//! to be as long as it says.

use std::io;
use std::path::{Component, Path, PathBuf};

/// No package unpacks to more than this.
const MOST: u64 = 2 * 1024 * 1024 * 1024;

fn no(why: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("not a zip this reads: {why}"))
}

/// A little-endian number of `n` bytes at `at`.
fn number(zip: &[u8], at: usize, n: usize) -> io::Result<u64> {
    let bytes = at.checked_add(n).and_then(|end| zip.get(at..end)).ok_or_else(|| no("cut off"))?;
    Ok(bytes.iter().rev().fold(0, |all, b| all << 8 | u64::from(*b)))
}

/// The CRC-32 a zip keeps of each file.
pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        *slot = (0..8).fold(n as u32, |c, _| if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 });
    }
    !bytes.iter().fold(!0u32, |c, b| table[((c ^ u32::from(*b)) & 0xFF) as usize] ^ (c >> 8))
}

/// Where an entry's name puts it under `into`: every part a plain name.
fn place(into: &Path, name: &str) -> io::Result<PathBuf> {
    let path = Path::new(name);
    let plain = path.components().all(|it| matches!(it, Component::Normal(_)));
    match plain && !name.is_empty() && !name.contains([':', '\\']) {
        true => Ok(into.join(path)),
        false => Err(no(&format!("the entry {name:?} would not land in the folder"))),
    }
}

/// Unpacks `zip` under `into`, which exists.
/// ponytail: archives of 4 GiB and more, or of 65,535 files and more (zip64), are refused;
/// read when a package is ever that large.
pub(crate) fn unpack(zip: &[u8], into: &Path) -> io::Result<()> {
    // The end record: the last 22 bytes, or before a comment.
    let ends = (0..=zip.len().saturating_sub(22)).rev().take(66_000);
    let end =
        ends.into_iter().find(|at| zip[*at..].starts_with(b"PK\x05\x06")).ok_or_else(|| no("no end"))?;
    let (count, mut at) = (number(zip, end + 10, 2)?, number(zip, end + 16, 4)? as usize);
    if count == 0xFFFF || at == 0xFFFF_FFFF {
        return Err(no("too large"));
    }
    let mut left = MOST;
    for _ in 0..count {
        if number(zip, at, 4)? != 0x0201_4B50 {
            return Err(no("its list is broken"));
        }
        let (flags, method, crc) =
            (number(zip, at + 8, 2)?, number(zip, at + 10, 2)?, number(zip, at + 16, 4)?);
        let (packed, size) = (number(zip, at + 20, 4)? as usize, number(zip, at + 24, 4)?);
        let lengths = [number(zip, at + 28, 2)?, number(zip, at + 30, 2)?, number(zip, at + 32, 2)?];
        let [named, extra, comment] = lengths.map(|it| it as usize);
        let (mode, local) = (number(zip, at + 38, 4)? >> 16, number(zip, at + 42, 4)? as usize);
        let name = zip.get(at + 46..at + 46 + named).and_then(|it| std::str::from_utf8(it).ok());
        let name = name.ok_or_else(|| no("a name that is not text"))?;
        at += 46 + named + extra + comment;
        let to = place(into, name.trim_end_matches('/'))?;
        if name.ends_with('/') {
            std::fs::create_dir_all(&to)?;
            continue;
        }
        if flags & 1 == 1 || mode & 0o170_000 == 0o120_000 {
            return Err(no(&format!("{name} is encrypted or a link")));
        }
        left = left.checked_sub(size).ok_or_else(|| no("it unpacks to too much"))?;
        // The file's own header says where its bytes start.
        let start = local + 30 + number(zip, local + 26, 2)? as usize + number(zip, local + 28, 2)? as usize;
        let bytes =
            start.checked_add(packed).and_then(|end| zip.get(start..end)).ok_or_else(|| no("cut off"))?;
        let plain = match method {
            0 => bytes.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(bytes, size as usize)
                .map_err(|_| no(&format!("{name} does not inflate")))?,
            other => return Err(no(&format!("{name} is packed by method {other}"))),
        };
        if plain.len() as u64 != size || u64::from(crc32(&plain)) != crc {
            return Err(no(&format!("{name} is not what its checksum says")));
        }
        to.parent().map_or(Ok(()), std::fs::create_dir_all)?;
        std::fs::write(&to, plain)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::zip;

    #[test]
    fn a_zip_unpacks_and_what_is_not_one_is_refused() {
        let root = std::path::absolute(".tmp/update/zip").unwrap();
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        let long = "a line of text\n".repeat(5000);
        let made = zip(&[
            ("app/app.exe", b"MZ\x00\x01\xFF"),
            ("app/x64/notes.txt", long.as_bytes()),
            ("app/empty", b""),
        ]);
        assert!(made.len() < long.len() / 10, "deflated");
        unpack(&made, &root).unwrap();
        assert_eq!(std::fs::read(root.join("app/app.exe")).unwrap(), b"MZ\x00\x01\xFF");
        assert_eq!(std::fs::read_to_string(root.join("app/x64/notes.txt")).unwrap(), long);
        assert_eq!(std::fs::read(root.join("app/empty")).unwrap(), b"");

        for name in ["../out.txt", "a/../../out.txt", "/abs.txt", "C:/abs.txt", "a\\b.txt", ""] {
            let refused = unpack(&zip(&[(name, b"x")]), &root).unwrap_err().to_string();
            assert!(refused.contains("would not land"), "{name}: {refused}");
        }
        assert!(!root.parent().unwrap().join("out.txt").exists());
        // A byte of a file changed, the archive cut short, and what is no zip at all.
        let mut changed = zip(&[("a.txt", b"stored as it is, too short to deflate")]);
        let at = changed.windows(6).position(|it| it == b"stored").unwrap();
        changed[at] = b'S';
        assert!(unpack(&changed, &root).unwrap_err().to_string().contains("checksum"));
        assert!(unpack(&made[..made.len() - 30], &root).is_err());
        assert!(unpack(b"<html>not found</html>", &root).is_err() && unpack(b"", &root).is_err());
    }
}
