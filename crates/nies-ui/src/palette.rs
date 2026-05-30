//! FBX Smooth NES palette: 64 RGB entries, the default M3 LUT.

/// Raw "Smooth (FBX)" palette: 64 entries × (R, G, B). Pinned as data so
/// the rendered colors are reproducible; the M3 golden hash is over
/// palette *indices*, so these exact bytes are cosmetic, not gated.
const SMOOTH_FBX: &[u8] = include_bytes!("../assets/smooth_fbx.pal");

/// The 64-entry palette as RGB triplets.
pub fn fbx_smooth() -> [[u8; 3]; 64] {
    assert_eq!(SMOOTH_FBX.len(), 64 * 3, "palette must be 192 bytes");
    let mut out = [[0u8; 3]; 64];
    for (i, chunk) in SMOOTH_FBX.chunks_exact(3).enumerate() {
        out[i] = [chunk[0], chunk[1], chunk[2]];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_has_64_entries() {
        let p = fbx_smooth();
        assert_eq!(p.len(), 64);
    }

    #[test]
    fn raw_palette_is_192_bytes() {
        assert_eq!(SMOOTH_FBX.len(), 192);
    }
}
