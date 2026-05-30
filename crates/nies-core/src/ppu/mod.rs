//! PPU — Picture Processing Unit (RP2C02 NTSC variant).
//!
//! Per-dot state machine called from `Bus::tick` 3 times per CPU cycle.
//! Module layout per the M2 design spec §2:
//! - state.rs: dot/scanline counters, frame parity
//! - registers.rs (Task 4+): PPUCTRL/MASK/STATUS/etc. + Loopy v/t/x/w
//! - vram.rs (Task 10): 2KB nametable RAM + mirroring
//! - oam.rs (Task 11): 256B primary OAM + 32B secondary OAM
//! - palette.rs (Task 12): 32-byte palette RAM with $3F1x mirrors
//! - background.rs (Task 26+): 8-cycle fetch pipeline
//! - sprite.rs (Task 39+): sprite eval, fetch, sprite-0 hit

pub mod oam;
pub mod palette;
pub mod registers;
pub mod state;
pub mod vram;

use crate::mapper::{MapperImpl, MapperKind};
use oam::Oam;
use palette::Palette;
use registers::Registers;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use state::PpuState;
use vram::Vram;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ppu {
    pub state: PpuState,
    pub regs: Registers,
    pub vram: Vram,
    pub oam: Oam,
    pub palette: Palette,
    #[serde(with = "BigArray")]
    pub framebuffer: [u8; 256 * 240],
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            state: PpuState::default(),
            regs: Registers::default(),
            vram: Vram::default(),
            oam: Oam::default(),
            palette: Palette::default(),
            framebuffer: [0; 256 * 240],
        }
    }
}

impl Ppu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the PPU by one dot. M2 unit 1: just advances the counter.
    /// `_mapper` will be used by later tasks for CHR access and notify_a12.
    pub fn step(&mut self, _mapper: &mut MapperKind) {
        let rendering_enabled = self.regs.rendering_enabled();
        self.state.advance_dot_with_rendering(rendering_enabled);
    }

    /// CPU-side register read at $2000-$3FFF. The address is mirrored
    /// down to the 8 register bytes via `& 0x7`.
    pub fn cpu_read(&mut self, mapper: &mut MapperKind, addr: u16) -> u8 {
        match addr & 0x7 {
            0 | 1 | 3 | 5 | 6 => self.regs.open_bus, // write-only registers
            2 => self.regs.read_ppustatus(),
            4 => self.oam.read(self.regs.oamaddr),
            7 => {
                let mirroring = mapper.mirroring();
                let addr_v = self.regs.v & 0x3FFF;
                let val = if addr_v < 0x3F00 {
                    let prev = self.regs.read_buffer;
                    self.regs.read_buffer = if addr_v < 0x2000 {
                        mapper.ppu_read(addr_v)
                    } else {
                        self.vram.read(addr_v, mirroring)
                    };
                    prev
                } else {
                    let mirror_addr = addr_v - 0x1000;
                    self.regs.read_buffer = if mirror_addr < 0x2000 {
                        mapper.ppu_read(mirror_addr)
                    } else {
                        self.vram.read(mirror_addr, mirroring)
                    };
                    self.palette.read(addr_v)
                };
                self.regs.v = self.regs.v.wrapping_add(self.regs.vram_increment()) & 0x7FFF;
                val
            }
            _ => unreachable!(),
        }
    }

    /// CPU-side register write at $2000-$3FFF.
    pub fn cpu_write(&mut self, mapper: &mut MapperKind, addr: u16, val: u8) {
        self.regs.open_bus = val;
        match addr & 0x7 {
            0 => self.regs.write_ppuctrl(val),
            1 => self.regs.write_ppumask(val),
            2 => {} // PPUSTATUS is read-only
            3 => self.regs.oamaddr = val,
            4 => {
                self.oam.write(self.regs.oamaddr, val);
                self.regs.oamaddr = self.regs.oamaddr.wrapping_add(1);
            }
            5 => self.regs.write_ppuscroll(val),
            6 => self.regs.write_ppuaddr(val),
            7 => {
                let mirroring = mapper.mirroring();
                let addr_v = self.regs.v & 0x3FFF;
                if addr_v < 0x2000 {
                    mapper.ppu_write(addr_v, val);
                } else if addr_v < 0x3F00 {
                    self.vram.write(addr_v, val, mirroring);
                } else {
                    self.palette.write(addr_v, val);
                }
                self.regs.v = self.regs.v.wrapping_add(self.regs.vram_increment()) & 0x7FFF;
            }
            _ => unreachable!(),
        }
    }

    /// Side-effect-free PPU register read for the debugger.
    pub fn cpu_peek(&self, _mapper: &MapperKind, addr: u16) -> u8 {
        match addr & 0x7 {
            0 | 1 | 3 | 5 | 6 => self.regs.open_bus,
            2 => (self.regs.status & 0xE0) | (self.regs.open_bus & 0x1F),
            4 => self.oam.read(self.regs.oamaddr),
            7 => self.regs.read_buffer, // approximation
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};
    use crate::mapper::MapperImpl;

    fn fake_mapper() -> MapperKind {
        let cart = Cartridge {
            format: NesFormat::INes,
            mapper_id: 0,
            submapper_id: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            has_trainer: false,
            prg_rom: vec![0; 16 * 1024],
            chr_rom: vec![0; 8 * 1024],
            prg_ram_size: 0,
            chr_ram_size: 0,
        };
        MapperKind::from_cartridge(&cart).unwrap()
    }

    #[test]
    fn step_advances_one_dot() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.step(&mut mapper);
        assert_eq!(ppu.state.dot, 1);
        assert_eq!(ppu.state.scanline, 0);
    }

    #[test]
    fn cpu_write_2000_latches_ppuctrl() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.cpu_write(&mut mapper, 0x2000, 0x80);
        assert_eq!(ppu.regs.ctrl, 0x80);
    }

    #[test]
    fn cpu_read_2002_returns_status_and_clears_w() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.regs.status = 0x80;
        ppu.regs.w = true;
        let v = ppu.cpu_read(&mut mapper, 0x2002);
        assert_eq!(v & 0x80, 0x80);
        assert_eq!(ppu.regs.status & 0x80, 0);
        assert!(!ppu.regs.w);
    }

    #[test]
    fn cpu_write_mirrors_within_2000_3fff() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.cpu_write(&mut mapper, 0x3FF8, 0x80); // mirrors $2000
        assert_eq!(ppu.regs.ctrl, 0x80);
    }

    #[test]
    fn cpu_write_2004_writes_oam_and_increments_oamaddr() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.regs.oamaddr = 0x10;
        ppu.cpu_write(&mut mapper, 0x2004, 0x42);
        assert_eq!(ppu.oam.read(0x10), 0x42);
        assert_eq!(ppu.regs.oamaddr, 0x11);
    }

    #[test]
    fn cpu_read_2007_buffered_for_vram_addresses() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.vram.write(0x2000, 0x55, mapper.mirroring());
        // Set v = $2000 via two PPUADDR writes.
        ppu.cpu_write(&mut mapper, 0x2006, 0x20);
        ppu.cpu_write(&mut mapper, 0x2006, 0x00);
        // First read returns the (zero) buffer; buffer refills with 0x55.
        let first = ppu.cpu_read(&mut mapper, 0x2007);
        assert_eq!(first, 0x00);
        // Second read returns 0x55 (buffer from the prior read).
        let second = ppu.cpu_read(&mut mapper, 0x2007);
        assert_eq!(second, 0x55);
    }
}
