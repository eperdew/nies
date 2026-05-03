//! iNES / NES 2.0 ROM file parser. See spec §3.2.

use std::error::Error;
use std::fmt;

/// Mirroring scheme as encoded in the iNES header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    /// Vertical mirroring (horizontal arrangement).
    Vertical,
    /// Horizontal mirroring (vertical arrangement).
    Horizontal,
    /// Four-screen VRAM provided by the cartridge.
    FourScreen,
    /// Single-screen mirroring (used by some mappers; lower or upper bank).
    SingleScreen,
}

/// Format of the loaded ROM file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NesFormat {
    INes,
    Nes2_0,
}

/// A parsed NES cartridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cartridge {
    pub format: NesFormat,
    pub mapper_id: u16,
    pub submapper_id: u8,
    pub mirroring: Mirroring,
    pub has_battery: bool,
    pub has_trainer: bool,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub prg_ram_size: u32,
    pub chr_ram_size: u32,
}

/// Errors produced while parsing an iNES / NES 2.0 ROM file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CartridgeError {
    TooShort,
    BadMagic,
    PrgRomSizeOverflow,
    ChrRomSizeOverflow,
    UnsupportedMapper(u16),
    Truncated { expected: usize, got: usize },
}

impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "ROM file shorter than the 16-byte iNES header"),
            Self::BadMagic => write!(f, "ROM does not start with the iNES magic bytes 'NES\\x1A'"),
            Self::PrgRomSizeOverflow => write!(f, "PRG-ROM size exceeds u32::MAX bytes"),
            Self::ChrRomSizeOverflow => write!(f, "CHR-ROM size exceeds u32::MAX bytes"),
            Self::UnsupportedMapper(id) => write!(
                f,
                "mapper {id} is not supported (M1 only ships NROM / mapper 0)"
            ),
            Self::Truncated { expected, got } => write!(
                f,
                "ROM file truncated: header declares {expected} bytes of PRG/CHR data, got {got}"
            ),
        }
    }
}

impl Error for CartridgeError {}

impl Cartridge {
    pub const INES_MAGIC: [u8; 4] = *b"NES\x1A";

    /// Parse a ROM file from raw bytes. Accepts both iNES (legacy) and NES 2.0 headers.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CartridgeError> {
        if bytes.len() < 16 {
            return Err(CartridgeError::TooShort);
        }
        if bytes[0..4] != Self::INES_MAGIC {
            return Err(CartridgeError::BadMagic);
        }

        // iNES header layout (per https://www.nesdev.org/wiki/INES):
        //   0..4   magic
        //   4      PRG-ROM size in 16 KiB units (low byte; high in NES 2.0)
        //   5      CHR-ROM size in 8 KiB units (low byte; high in NES 2.0)
        //   6      Flags 6:  mapper-low, mirroring, battery, trainer, four-screen
        //   7      Flags 7:  mapper-mid, NES 2.0 indicator, vs/playchoice
        //   8..16  varies by format
        let flags6 = bytes[6];
        let flags7 = bytes[7];

        let format = if (flags7 & 0x0C) == 0x08 {
            NesFormat::Nes2_0
        } else {
            NesFormat::INes
        };

        let prg_units_low = bytes[4] as u16;
        let chr_units_low = bytes[5] as u16;

        let (prg_units, chr_units, mapper_id, submapper_id, prg_ram_size, chr_ram_size) =
            match format {
                NesFormat::INes => {
                    let mapper_id = ((flags7 & 0xF0) as u16) | ((flags6 >> 4) as u16);
                    let prg_ram_size = if bytes[8] == 0 {
                        8192
                    } else {
                        bytes[8] as u32 * 8192
                    };
                    (prg_units_low, chr_units_low, mapper_id, 0, prg_ram_size, 0)
                }
                NesFormat::Nes2_0 => {
                    // NES 2.0 augments header bytes 9-15.
                    let prg_high = (bytes[9] & 0x0F) as u16;
                    let chr_high = ((bytes[9] >> 4) & 0x0F) as u16;
                    let mapper_id = ((bytes[8] & 0x0F) as u16) << 8
                        | ((flags7 & 0xF0) as u16)
                        | ((flags6 >> 4) as u16);
                    let submapper_id = (bytes[8] >> 4) & 0x0F;
                    // Section "PRG-RAM (volatile/non-volatile) size":
                    let prg_ram_size = nes2_size_unit(bytes[10] & 0x0F);
                    let chr_ram_size = nes2_size_unit(bytes[11] & 0x0F);
                    (
                        (prg_high << 8) | prg_units_low,
                        (chr_high << 8) | chr_units_low,
                        mapper_id,
                        submapper_id,
                        prg_ram_size,
                        chr_ram_size,
                    )
                }
            };

        let prg_rom_bytes = (prg_units as usize)
            .checked_mul(16 * 1024)
            .ok_or(CartridgeError::PrgRomSizeOverflow)?;
        let chr_rom_bytes = (chr_units as usize)
            .checked_mul(8 * 1024)
            .ok_or(CartridgeError::ChrRomSizeOverflow)?;

        let has_trainer = (flags6 & 0x04) != 0;
        let has_battery = (flags6 & 0x02) != 0;

        let four_screen = (flags6 & 0x08) != 0;
        let vertical = (flags6 & 0x01) != 0;
        let mirroring = if four_screen {
            Mirroring::FourScreen
        } else if vertical {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let mut offset = 16usize;
        if has_trainer {
            offset += 512; // 512-byte trainer immediately after header
        }

        let need = offset + prg_rom_bytes + chr_rom_bytes;
        if bytes.len() < need {
            return Err(CartridgeError::Truncated {
                expected: need,
                got: bytes.len(),
            });
        }

        let prg_rom = bytes[offset..offset + prg_rom_bytes].to_vec();
        offset += prg_rom_bytes;
        let chr_rom = bytes[offset..offset + chr_rom_bytes].to_vec();

        Ok(Cartridge {
            format,
            mapper_id,
            submapper_id,
            mirroring,
            has_battery,
            has_trainer,
            prg_rom,
            chr_rom,
            prg_ram_size,
            chr_ram_size,
        })
    }

    /// SHA-256 hash of the cartridge's PRG-ROM concatenated with CHR-ROM. Used as
    /// a save-state key in M7; defined here because the cartridge owns the data.
    pub fn rom_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&self.prg_rom);
        hasher.update(&self.chr_rom);
        hasher.finalize().into()
    }
}

/// Decode an NES 2.0 size unit (4-bit nibble) into bytes. Per the spec, value 0 means
/// "no PRG-RAM present"; values 1..=14 encode `64 << v` bytes; value 15 is reserved.
fn nes2_size_unit(nibble: u8) -> u32 {
    match nibble {
        0 => 0,
        v if v <= 14 => 64u32 << v,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_minimal_ines(prg_units: u8, chr_units: u8, flags6: u8, flags7: u8) -> Vec<u8> {
        let mut buf = vec![0u8; 16];
        buf[0..4].copy_from_slice(&Cartridge::INES_MAGIC);
        buf[4] = prg_units;
        buf[5] = chr_units;
        buf[6] = flags6;
        buf[7] = flags7;
        buf.resize(
            16 + prg_units as usize * 16 * 1024 + chr_units as usize * 8 * 1024,
            0,
        );
        buf
    }

    #[test]
    fn rejects_short_rom() {
        let result = Cartridge::from_bytes(&[0u8; 8]);
        assert_eq!(result, Err(CartridgeError::TooShort));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = vec![0u8; 32];
        bytes[0..4].copy_from_slice(b"BORK");
        let result = Cartridge::from_bytes(&bytes);
        assert_eq!(result, Err(CartridgeError::BadMagic));
    }

    #[test]
    fn parses_minimal_nrom_ines() {
        let bytes = build_minimal_ines(2, 1, 0x00, 0x00); // 32 KiB PRG, 8 KiB CHR, mapper 0
        let cart = Cartridge::from_bytes(&bytes).unwrap();
        assert_eq!(cart.format, NesFormat::INes);
        assert_eq!(cart.mapper_id, 0);
        assert_eq!(cart.prg_rom.len(), 32 * 1024);
        assert_eq!(cart.chr_rom.len(), 8 * 1024);
        assert_eq!(cart.mirroring, Mirroring::Horizontal);
        assert!(!cart.has_battery);
        assert!(!cart.has_trainer);
    }

    #[test]
    fn vertical_mirroring_flag_round_trip() {
        let bytes = build_minimal_ines(1, 1, 0x01, 0x00);
        let cart = Cartridge::from_bytes(&bytes).unwrap();
        assert_eq!(cart.mirroring, Mirroring::Vertical);
    }

    #[test]
    fn battery_flag_round_trip() {
        let bytes = build_minimal_ines(1, 1, 0x02, 0x00);
        let cart = Cartridge::from_bytes(&bytes).unwrap();
        assert!(cart.has_battery);
    }

    #[test]
    fn truncated_rom_is_rejected() {
        // Header claims 1 unit of PRG (16 KiB) but supplies only the header.
        let mut bytes = vec![0u8; 16];
        bytes[0..4].copy_from_slice(&Cartridge::INES_MAGIC);
        bytes[4] = 1;
        let err = Cartridge::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CartridgeError::Truncated { .. }));
    }

    #[test]
    fn rom_hash_is_deterministic() {
        let bytes = build_minimal_ines(1, 1, 0x00, 0x00);
        let a = Cartridge::from_bytes(&bytes).unwrap().rom_hash();
        let b = Cartridge::from_bytes(&bytes).unwrap().rom_hash();
        assert_eq!(a, b);
    }
}
