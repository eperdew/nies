//! `Nes` — top-level emulator driver (CPU + Bus), and the embedded demo ROM.
//!
//! Pure logic: honors the crate's no-I/O contract. `include_bytes!` is a
//! compile-time data embed, not runtime I/O.

/// Bytes of the bundled `nmi_sync/demo_ntsc.nes` test ROM. Single source
/// shared by both frontends and the golden-hash tests (spec §5.3).
pub fn demo_rom_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/roms/nmi_sync/demo_ntsc.nes"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;

    #[test]
    fn demo_rom_parses_as_cartridge() {
        let bytes = demo_rom_bytes();
        assert!(
            bytes.len() > 16,
            "demo ROM should be larger than an iNES header"
        );
        Cartridge::from_bytes(bytes).expect("demo ROM parses as a cartridge");
    }
}
