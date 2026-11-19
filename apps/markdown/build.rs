//! Links `assets/logo.ico` as the executable's icon (specs/16, "An executable's icon"): a
//! resource file of an icon group, resource 1, whose images are resources 1 to n, written
//! here from the icon file's headers, so no resource compiler is needed.

use std::path::PathBuf;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
/// Moveable, pure and discardable, as resource compilers mark icons.
const FLAGS: u16 = 0x1030;

fn le16(at: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([at[i], at[i + 1]])
}

fn le32(at: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([at[i], at[i + 1], at[i + 2], at[i + 3]])
}

/// One resource: its header (a numbered type and name) and its data, padded to four bytes.
fn resource(out: &mut Vec<u8>, kind: u16, name: u16, flags: u16, data: &[u8]) {
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(32u32.to_le_bytes());
    for v in [0xFFFF, kind, 0xFFFF, name] {
        out.extend(u16::to_le_bytes(v));
    }
    out.extend(0u32.to_le_bytes()); // data version
    out.extend(flags.to_le_bytes());
    out.extend(0u16.to_le_bytes()); // neutral language
    out.extend([0u8; 8]); // version, characteristics
    out.extend(data);
    out.resize(out.len().next_multiple_of(4), 0);
}

fn main() {
    println!("cargo:rerun-if-changed=assets/logo.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let ico = std::fs::read("assets/logo.ico").expect("assets/logo.ico");
    let count = le16(&ico, 4) as usize;
    let mut res = Vec::new();
    resource(&mut res, 0, 0, 0, &[]); // a resource file starts with an empty one
    let mut group = Vec::new();
    for v in [0u16, 1, count as u16] {
        group.extend(v.to_le_bytes());
    }
    for k in 0..count {
        let entry = &ico[6 + 16 * k..6 + 16 * (k + 1)];
        let (size, offset) = (le32(entry, 8) as usize, le32(entry, 12) as usize);
        resource(&mut res, RT_ICON, k as u16 + 1, FLAGS, &ico[offset..offset + size]);
        // The group's entry is the file's, with the resource's number for its offset.
        group.extend(&entry[..12]);
        group.extend((k as u16 + 1).to_le_bytes());
    }
    resource(&mut res, RT_GROUP_ICON, 1, FLAGS, &group);
    let file = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("icon.res");
    std::fs::write(&file, res).expect("the resource file");
    println!("cargo:rustc-link-arg-bins={}", file.display());
}
