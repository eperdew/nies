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

pub mod background;
pub mod oam;
pub mod palette;
pub mod registers;
pub mod state;
pub mod vram;

use crate::mapper::{MapperImpl, MapperKind};
use background::Background;
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
    pub bg: Background,
    #[serde(with = "BigArray")]
    pub framebuffer: [u8; 256 * 240],
    /// Internal: latched on a rising edge of the NMI line; drained by
    /// `take_nmi()`. Take-on-read edge semantics per design spec §3.1.
    nmi_pending_take: bool,
    /// Internal: previous sample of the NMI line, for edge detection.
    /// `nmi_line = (PPUCTRL bit 7) AND (PPUSTATUS bit 7)`.
    nmi_line_prev: bool,
    /// Internal: set when PPUSTATUS is read at the dot before vblank
    /// raise (scanline 241 dot 0). Blocks the dot-1 vblank set for the
    /// rest of the frame; cleared at scanline 261 dot 1.
    vblank_suppress: bool,
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            state: PpuState::default(),
            regs: Registers::default(),
            vram: Vram::default(),
            oam: Oam::default(),
            palette: Palette::default(),
            bg: Background::default(),
            framebuffer: [0; 256 * 240],
            nmi_pending_take: false,
            nmi_line_prev: false,
            vblank_suppress: false,
        }
    }
}

impl Ppu {
    pub fn new() -> Self {
        Self::default()
    }

    fn nt_addr(&self) -> u16 {
        0x2000 | (self.regs.v & 0x0FFF)
    }

    fn at_addr(&self) -> u16 {
        0x23C0 | (self.regs.v & 0x0C00) | ((self.regs.v >> 4) & 0x38) | ((self.regs.v >> 2) & 0x07)
    }

    fn pat_addr(&self, hi: bool) -> u16 {
        let base = self.regs.bg_pattern_table_base();
        let tile = self.bg.nt_byte as u16;
        let fine_y = (self.regs.v >> 12) & 7;
        base + (tile << 4) + fine_y + if hi { 8 } else { 0 }
    }

    fn ppu_bus_read(&mut self, mapper: &mut MapperKind, addr: u16) -> u8 {
        let mirroring = mapper.mirroring();
        if addr < 0x2000 {
            mapper.ppu_read(addr)
        } else if addr < 0x3F00 {
            self.vram.read(addr, mirroring)
        } else {
            self.palette.read(addr)
        }
    }

    fn increment_coarse_x(&mut self) {
        if (self.regs.v & 0x001F) == 31 {
            self.regs.v &= !0x001F;
            self.regs.v ^= 0x0400;
        } else {
            self.regs.v += 1;
        }
    }

    fn copy_horizontal_t_to_v(&mut self) {
        self.regs.v = (self.regs.v & !0x041F) | (self.regs.t & 0x041F);
    }

    fn copy_vertical_t_to_v(&mut self) {
        self.regs.v = (self.regs.v & !0x7BE0) | (self.regs.t & 0x7BE0);
    }

    fn increment_y(&mut self) {
        if (self.regs.v & 0x7000) != 0x7000 {
            self.regs.v += 0x1000;
        } else {
            self.regs.v &= !0x7000;
            let mut y = (self.regs.v & 0x03E0) >> 5;
            if y == 29 {
                y = 0;
                self.regs.v ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }
            self.regs.v = (self.regs.v & !0x03E0) | (y << 5);
        }
    }

    fn at_bits_for_tile(&self) -> (bool, bool) {
        let coarse_x_hi = (self.regs.v >> 1) & 1; // bit 1 of coarse X
        let coarse_y_hi = (self.regs.v >> 6) & 1; // bit 1 of coarse Y
        let shift = ((coarse_y_hi << 1) | coarse_x_hi) * 2;
        let bits = (self.bg.at_byte >> shift) & 0b11;
        (bits & 1 != 0, bits & 2 != 0)
    }

    /// Advance the PPU by one dot.
    pub fn step(&mut self, mapper: &mut MapperKind) {
        let rendering = self.regs.rendering_enabled();
        let scanline = self.state.scanline;
        let dot = self.state.dot;
        let visible_or_pre = scanline < 240 || scanline == 261;

        // Background pipeline runs during dots 1-256 and 321-336.
        if rendering && visible_or_pre && ((1..=256).contains(&dot) || (321..=336).contains(&dot)) {
            self.bg.shift();
            match dot % 8 {
                1 => {
                    let a = self.nt_addr();
                    self.bg.nt_byte = self.ppu_bus_read(mapper, a);
                }
                3 => {
                    let a = self.at_addr();
                    self.bg.at_byte = self.ppu_bus_read(mapper, a);
                }
                5 => {
                    let a = self.pat_addr(false);
                    self.bg.pat_lo_latch = self.ppu_bus_read(mapper, a);
                }
                7 => {
                    let a = self.pat_addr(true);
                    self.bg.pat_hi_latch = self.ppu_bus_read(mapper, a);
                }
                0 => {
                    let (al, ah) = self.at_bits_for_tile();
                    self.bg.at_latch_lo = al;
                    self.bg.at_latch_hi = ah;
                    self.bg.reload_shifters();
                    self.increment_coarse_x();
                }
                _ => {}
            }
        }

        if rendering && visible_or_pre && dot == 256 {
            self.increment_y();
        }

        if rendering && visible_or_pre && dot == 257 {
            self.copy_horizontal_t_to_v();
        }
        if rendering && scanline == 261 && (280..=304).contains(&dot) {
            self.copy_vertical_t_to_v();
        }

        self.state.advance_dot_with_rendering(rendering);

        if self.state.dot == 1 {
            match self.state.scanline {
                241 => {
                    if !self.vblank_suppress {
                        self.regs.status |= 0x80;
                    }
                }
                261 => {
                    self.regs.status &= 0x1F; // clear vblank, sprite-0 hit, overflow
                    self.vblank_suppress = false;
                }
                _ => {}
            }
        }
        self.update_nmi_line();
    }

    /// Drain the NMI rising-edge latch. Returns true at most once per
    /// rising edge of the NMI line.
    pub fn take_nmi(&mut self) -> bool {
        let v = self.nmi_pending_take;
        self.nmi_pending_take = false;
        v
    }

    /// Re-sample the NMI line and latch on a rising edge. NMI line is
    /// `(PPUCTRL bit 7) AND (PPUSTATUS bit 7)`.
    fn update_nmi_line(&mut self) {
        let line = self.regs.nmi_enabled() && (self.regs.status & 0x80) != 0;
        if line && !self.nmi_line_prev {
            self.nmi_pending_take = true;
        }
        self.nmi_line_prev = line;
    }

    /// CPU-side register read at $2000-$3FFF. The address is mirrored
    /// down to the 8 register bytes via `& 0x7`.
    pub fn cpu_read(&mut self, mapper: &mut MapperKind, addr: u16) -> u8 {
        match addr & 0x7 {
            0 | 1 | 3 | 5 | 6 => self.regs.open_bus, // write-only registers
            2 => {
                if self.state.scanline == 241 && self.state.dot == 0 {
                    self.vblank_suppress = true;
                }
                let v = self.regs.read_ppustatus();
                self.update_nmi_line(); // reading clears bit 7, which can drop the line
                v
            }
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
            0 => {
                self.regs.write_ppuctrl(val);
                self.update_nmi_line();
            }
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
    fn vblank_flag_set_at_scanline_241_dot_1() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        while !(ppu.state.scanline == 241 && ppu.state.dot == 0) {
            ppu.step(&mut mapper);
        }
        assert_eq!(ppu.regs.status & 0x80, 0);
        ppu.step(&mut mapper);
        assert_eq!(ppu.regs.status & 0x80, 0x80);
    }

    #[test]
    fn vblank_flag_cleared_at_scanline_261_dot_1() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.regs.status = 0x80;
        while !(ppu.state.scanline == 261 && ppu.state.dot == 0) {
            ppu.step(&mut mapper);
        }
        assert_eq!(ppu.regs.status & 0x80, 0x80);
        ppu.step(&mut mapper);
        assert_eq!(ppu.regs.status & 0x80, 0);
    }

    #[test]
    fn take_nmi_fires_on_vblank_when_ctrl_bit7_set() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.regs.write_ppuctrl(0x80);
        while !(ppu.state.scanline == 241 && ppu.state.dot == 0) {
            ppu.step(&mut mapper);
        }
        assert!(!ppu.take_nmi());
        ppu.step(&mut mapper);
        assert!(ppu.take_nmi());
    }

    #[test]
    fn take_nmi_does_not_fire_when_ctrl_bit7_clear() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        while ppu.state.scanline < 242 {
            ppu.step(&mut mapper);
        }
        assert!(!ppu.take_nmi());
    }

    #[test]
    fn take_nmi_returns_true_only_once_per_edge() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.regs.write_ppuctrl(0x80);
        while !(ppu.state.scanline == 241 && ppu.state.dot == 1) {
            ppu.step(&mut mapper);
        }
        assert!(ppu.take_nmi());
        assert!(!ppu.take_nmi());
    }

    #[test]
    fn enabling_nmi_while_vblank_set_fires_immediate_nmi() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        ppu.regs.status = 0x80;
        ppu.regs.ctrl = 0x00;
        ppu.nmi_line_prev = false;
        assert!(!ppu.take_nmi());
        ppu.cpu_write(&mut mapper, 0x2000, 0x80);
        assert!(ppu.take_nmi());
    }

    #[test]
    fn ppustatus_read_at_241_dot_0_suppresses_vblank_this_frame() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        while !(ppu.state.scanline == 241 && ppu.state.dot == 0) {
            ppu.step(&mut mapper);
        }
        let _ = ppu.cpu_read(&mut mapper, 0x2002);
        ppu.step(&mut mapper);
        assert_eq!(ppu.regs.status & 0x80, 0);
    }

    #[test]
    fn ppustatus_read_at_241_dot_2_does_not_suppress() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        while !(ppu.state.scanline == 241 && ppu.state.dot == 2) {
            ppu.step(&mut mapper);
        }
        let v = ppu.cpu_read(&mut mapper, 0x2002);
        assert_eq!(v & 0x80, 0x80);
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

    #[test]
    fn coarse_x_increment_basic() {
        let mut ppu = Ppu::new();
        ppu.regs.v = 0x2000;
        ppu.increment_coarse_x();
        assert_eq!(ppu.regs.v, 0x2001);
    }

    #[test]
    fn coarse_x_wraps_and_toggles_nametable_bit_10() {
        let mut ppu = Ppu::new();
        ppu.regs.v = 0x201F;
        ppu.increment_coarse_x();
        assert_eq!(ppu.regs.v & 0x001F, 0);
        assert_eq!(ppu.regs.v & 0x0400, 0x0400);
    }

    #[test]
    #[allow(clippy::unusual_byte_groupings)]
    fn y_increment_fine_y_wraps_and_increments_coarse_y() {
        let mut ppu = Ppu::new();
        ppu.regs.v = 0b111_00_00000_00000; // fine_y=7, coarse_y=0
        ppu.increment_y();
        assert_eq!((ppu.regs.v >> 12) & 7, 0);
        assert_eq!((ppu.regs.v >> 5) & 0x1F, 1);
    }

    #[test]
    #[allow(clippy::unusual_byte_groupings)]
    fn y_increment_coarse_y_29_to_0_toggles_v_bit_11() {
        let mut ppu = Ppu::new();
        ppu.regs.v = 0b111_00_11101_00000; // fine_y=7, coarse_y=29
        ppu.increment_y();
        assert_eq!((ppu.regs.v >> 5) & 0x1F, 0);
        assert_eq!(ppu.regs.v & 0x0800, 0x0800);
    }

    #[test]
    #[allow(clippy::unusual_byte_groupings)]
    fn y_increment_from_coarse_y_31_wraps_without_toggle() {
        let mut ppu = Ppu::new();
        ppu.regs.v = 0b111_00_11111_00000; // fine_y=7, coarse_y=31
        ppu.increment_y();
        assert_eq!((ppu.regs.v >> 5) & 0x1F, 0);
        assert_eq!(ppu.regs.v & 0x0800, 0);
    }

    #[test]
    #[allow(clippy::unusual_byte_groupings)]
    fn horizontal_copy_at_dot_257() {
        let mut ppu = Ppu::new();
        ppu.regs.t = 0b111_11_11111_10101;
        ppu.regs.v = 0;
        ppu.copy_horizontal_t_to_v();
        // bit 10 (horizontal NT select) | coarse_x bits 0-4
        assert_eq!(ppu.regs.v & 0x041F, 0x0400 | 0b10101);
    }

    #[test]
    #[allow(clippy::unusual_byte_groupings)]
    fn vertical_copy_at_pre_render() {
        let mut ppu = Ppu::new();
        ppu.regs.t = 0b111_11_11111_10101;
        ppu.regs.v = 0;
        ppu.copy_vertical_t_to_v();
        assert_eq!(ppu.regs.v & 0x7BE0, 0b111_10_11111_00000);
    }
}
