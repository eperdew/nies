//! NROM / mapper 0.
//!
//! - PRG-ROM: 16 KiB or 32 KiB at $8000-$FFFF (16 KiB games mirror at $C000).
//! - PRG-RAM: 0 or 8 KiB at $6000-$7FFF.
//! - CHR: 8 KiB ROM (or RAM in some variants) at PPU $0000-$1FFF.
//! - No bank switching, no IRQ.

use crate::cartridge::{Cartridge, Mirroring};

#[derive(Debug, Clone)]
pub struct NromState {
    prg_rom: Vec<u8>,
    chr: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: Mirroring,
    /// True iff CHR was provided as ROM (read-only). Some homebrew NROM uses CHR-RAM.
    chr_is_rom: bool,
    #[cfg(test)]
    pub a12_log: std::cell::RefCell<Vec<bool>>,
}

impl NromState {
    pub fn new(cart: &Cartridge) -> Self {
        let prg_ram = vec![0u8; cart.prg_ram_size as usize];
        let chr_is_rom = !cart.chr_rom.is_empty();
        let chr = if chr_is_rom {
            cart.chr_rom.clone()
        } else {
            // No CHR-ROM provided → 8 KiB CHR-RAM.
            vec![0u8; 8 * 1024]
        };
        NromState {
            prg_rom: cart.prg_rom.clone(),
            chr,
            prg_ram,
            mirroring: cart.mirroring,
            chr_is_rom,
            #[cfg(test)]
            a12_log: std::cell::RefCell::new(Vec::new()),
        }
    }

    #[cfg(test)]
    pub fn notify_a12(&mut self, level: bool) {
        self.a12_log.borrow_mut().push(level);
    }

    #[cfg(not(test))]
    pub fn notify_a12(&mut self, _level: bool) {}

    pub fn cpu_read(&self, addr: u16) -> u8 {
        // NROM has no read side effects, so read == peek.
        self.cpu_peek(addr)
    }

    /// Side-effect-free CPU-bus read. For NROM this is identical to
    /// `cpu_read`; the distinction matters for stateful mappers.
    pub fn cpu_peek(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF if !self.prg_ram.is_empty() => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xFFFF => {
                let len = self.prg_rom.len();
                // 16 KiB PRG mirrors at $C000; 32 KiB occupies $8000-$FFFF directly.
                let idx = (addr as usize - 0x8000) % len;
                self.prg_rom[idx]
            }
            // Unmapped: returns 0 (open-bus modeled minimally at M1).
            _ => 0,
        }
    }

    pub fn cpu_write(&mut self, addr: u16, val: u8) {
        if (0x6000..=0x7FFF).contains(&addr) && !self.prg_ram.is_empty() {
            self.prg_ram[(addr - 0x6000) as usize] = val;
        }
        // Writes to $8000-$FFFF are no-ops on NROM (PRG is ROM).
        let _ = val;
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        // NROM has no read side effects on the PPU bus either.
        self.ppu_peek(addr)
    }

    /// Side-effect-free PPU-bus read. Same body as `ppu_read` for NROM.
    pub fn ppu_peek(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.chr[addr as usize],
            _ => 0,
        }
    }

    pub fn ppu_write(&mut self, addr: u16, val: u8) {
        if !self.chr_is_rom && addr < 0x2000 {
            self.chr[addr as usize] = val;
        }
        // CHR-ROM writes are no-ops.
        let _ = val;
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};

    fn fake_cart(prg_kib: usize, chr_kib: usize) -> Cartridge {
        Cartridge {
            format: NesFormat::INes,
            mapper_id: 0,
            submapper_id: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            has_trainer: false,
            prg_rom: (0..(prg_kib * 1024) as u32).map(|i| i as u8).collect(),
            chr_rom: vec![0xCD; chr_kib * 1024],
            prg_ram_size: 8 * 1024,
            chr_ram_size: 0,
        }
    }

    #[test]
    fn prg_rom_16k_mirrors_at_c000() {
        let cart = fake_cart(16, 8);
        let nrom = NromState::new(&cart);
        // PRG[0] should appear at both $8000 and $C000.
        assert_eq!(nrom.cpu_read(0x8000), nrom.cpu_read(0xC000));
        assert_eq!(nrom.cpu_read(0x8000), 0);
        assert_eq!(nrom.cpu_read(0x8001), 1);
        assert_eq!(nrom.cpu_read(0xC001), 1);
    }

    #[test]
    fn prg_rom_32k_does_not_mirror() {
        let cart = fake_cart(32, 8);
        let nrom = NromState::new(&cart);
        // The 32 KiB ROM occupies $8000-$FFFF linearly. The fake_cart fills
        // PRG with `(i as u8)` so each 256-byte block repeats 0x00..0xFF.
        // $8000 → PRG[0x0000] = 0; $C000 → PRG[0x4000] = 0 (mod 256).
        assert_eq!(nrom.cpu_read(0x8000), 0);
        assert_eq!(nrom.cpu_read(0xC000), 0); // halfway through 32 KiB
        // A non-aligned offset proves the read isn't mirrored from $8000.
        assert_eq!(nrom.cpu_read(0xC001), 1);
    }

    #[test]
    fn prg_ram_round_trip() {
        let cart = fake_cart(16, 8);
        let mut nrom = NromState::new(&cart);
        nrom.cpu_write(0x6000, 0xAB);
        nrom.cpu_write(0x7FFF, 0xCD);
        assert_eq!(nrom.cpu_read(0x6000), 0xAB);
        assert_eq!(nrom.cpu_read(0x7FFF), 0xCD);
    }

    #[test]
    fn prg_rom_writes_are_ignored() {
        let cart = fake_cart(16, 8);
        let mut nrom = NromState::new(&cart);
        let original = nrom.cpu_read(0x8000);
        nrom.cpu_write(0x8000, !original);
        assert_eq!(nrom.cpu_read(0x8000), original);
    }

    #[test]
    fn chr_rom_writes_are_ignored() {
        let cart = fake_cart(16, 8);
        let mut nrom = NromState::new(&cart);
        nrom.ppu_write(0x0000, 0x55);
        assert_eq!(nrom.ppu_read(0x0000), 0xCD); // unchanged
    }

    #[test]
    fn chr_ram_round_trip_when_no_chr_rom() {
        let mut cart = fake_cart(16, 0);
        cart.chr_rom = vec![]; // explicit no CHR-ROM → CHR-RAM mode
        let mut nrom = NromState::new(&cart);
        nrom.ppu_write(0x0500, 0x77);
        assert_eq!(nrom.ppu_read(0x0500), 0x77);
    }
}
