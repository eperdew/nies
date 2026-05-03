//! 6502 addressing-mode resolution helpers.
//!
//! Each helper reads operand bytes from PC (advancing PC past them),
//! computes the effective address, and returns it. For modes where the
//! cycle count varies on page-cross, the helper also returns whether
//! a page boundary was crossed.
//!
//! References:
//! - <https://www.nesdev.org/wiki/CPU_addressing_modes> — the canonical
//!   per-mode summary, including the page-cross dummy-read timing rules
//!   and the indirect / zero-page-indexed page-wrap quirks implemented
//!   by `read_word_buggy` and `read_word_zp` below.
//! - <https://www.nesdev.org/wiki/CPU_unofficial_opcodes> — addressing
//!   variants of the unofficial opcodes that share these helpers.

use crate::bus::BusLike;
use crate::cpu::Cpu;

/// Read a 16-bit value from two consecutive bus addresses, low byte first.
pub fn read_word<B: BusLike>(bus: &mut B, addr: u16) -> u16 {
    let lo = bus.read(addr) as u16;
    let hi = bus.read(addr.wrapping_add(1)) as u16;
    (hi << 8) | lo
}

/// Read 16-bit pointer with the 6502's page-wrap bug: high byte read
/// from the same page as the low byte (i.e., low+1 wraps within the
/// page, not across pages). Used by JMP indirect.
pub fn read_word_buggy<B: BusLike>(bus: &mut B, addr: u16) -> u16 {
    let lo = bus.read(addr) as u16;
    // Buggy increment: only the low byte wraps.
    let hi_addr = (addr & 0xFF00) | ((addr.wrapping_add(1)) & 0x00FF);
    let hi = bus.read(hi_addr) as u16;
    (hi << 8) | lo
}

/// Read 16-bit pointer from zero-page address `zp`, with the page-wrap
/// bug intrinsic to zero-page indexing. Used by indirect-X / indirect-Y.
pub fn read_word_zp<B: BusLike>(bus: &mut B, zp: u8) -> u16 {
    let lo = bus.read(zp as u16) as u16;
    let hi = bus.read(zp.wrapping_add(1) as u16) as u16;
    (hi << 8) | lo
}

/// Fetch the next byte from PC and advance.
pub fn fetch_byte<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u8 {
    let v = bus.read(cpu.pc);
    cpu.pc = cpu.pc.wrapping_add(1);
    v
}

/// Fetch the next word from PC (LSB first) and advance PC by 2.
pub fn fetch_word<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let lo = fetch_byte(cpu, bus) as u16;
    let hi = fetch_byte(cpu, bus) as u16;
    (hi << 8) | lo
}

// --- Address resolvers ---

/// Zero-page: 1-byte operand is the effective address.
pub fn zp<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    fetch_byte(cpu, bus) as u16
}

/// Zero-page,X: 1-byte operand + X (8-bit wrap-around within zero page).
/// Includes the dummy read at the unindexed address per real 6502 timing.
pub fn zp_x<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let base = fetch_byte(cpu, bus);
    let _ = bus.read(base as u16); // dummy read
    base.wrapping_add(cpu.x) as u16
}

/// Zero-page,Y: same as zp,X but with Y.
pub fn zp_y<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let base = fetch_byte(cpu, bus);
    let _ = bus.read(base as u16); // dummy read
    base.wrapping_add(cpu.y) as u16
}

/// Absolute: 2-byte little-endian operand.
pub fn abs<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    fetch_word(cpu, bus)
}

/// Absolute,X (read variants). Returns (effective addr, page_crossed).
/// The cycle penalty for page-cross is encoded by the caller's choice
/// to either issue a dummy read or not.
pub fn abs_x_read<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let base = fetch_word(cpu, bus);
    let effective = base.wrapping_add(cpu.x as u16);
    if (base & 0xFF00) != (effective & 0xFF00) {
        // Page-crossed: dummy read at the wrong-page address.
        let _ = bus.read((base & 0xFF00) | (effective & 0x00FF));
    }
    effective
}

/// Absolute,X (read-modify-write or store variants). Always issues the
/// dummy read regardless of page-cross.
pub fn abs_x_rmw<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let base = fetch_word(cpu, bus);
    let effective = base.wrapping_add(cpu.x as u16);
    let _ = bus.read((base & 0xFF00) | (effective & 0x00FF));
    effective
}

/// Absolute,Y (read variants).
pub fn abs_y_read<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let base = fetch_word(cpu, bus);
    let effective = base.wrapping_add(cpu.y as u16);
    if (base & 0xFF00) != (effective & 0xFF00) {
        let _ = bus.read((base & 0xFF00) | (effective & 0x00FF));
    }
    effective
}

/// Absolute,Y (RMW/store variants).
pub fn abs_y_rmw<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let base = fetch_word(cpu, bus);
    let effective = base.wrapping_add(cpu.y as u16);
    let _ = bus.read((base & 0xFF00) | (effective & 0x00FF));
    effective
}

/// Indirect (JMP only): 2-byte operand → 2-byte pointer with page-wrap bug.
pub fn ind<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let ptr = fetch_word(cpu, bus);
    read_word_buggy(bus, ptr)
}

/// (Indirect,X) "preindexed indirect": (zp + X) wrap, then read 2-byte pointer.
pub fn ind_x<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let base = fetch_byte(cpu, bus);
    let _ = bus.read(base as u16); // dummy read
    let zp_addr = base.wrapping_add(cpu.x);
    read_word_zp(bus, zp_addr)
}

/// (Indirect),Y "postindexed indirect" (read variants).
pub fn ind_y_read<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let zp_addr = fetch_byte(cpu, bus);
    let base = read_word_zp(bus, zp_addr);
    let effective = base.wrapping_add(cpu.y as u16);
    if (base & 0xFF00) != (effective & 0xFF00) {
        let _ = bus.read((base & 0xFF00) | (effective & 0x00FF));
    }
    effective
}

/// (Indirect),Y RMW/store variants.
pub fn ind_y_rmw<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> u16 {
    let zp_addr = fetch_byte(cpu, bus);
    let base = read_word_zp(bus, zp_addr);
    let effective = base.wrapping_add(cpu.y as u16);
    let _ = bus.read((base & 0xFF00) | (effective & 0x00FF));
    effective
}

/// Relative (branch): 1-byte signed offset added to PC. Caller handles the
/// branch-taken / page-cross cycle penalties.
pub fn relative<B: BusLike>(cpu: &mut Cpu, bus: &mut B) -> i8 {
    fetch_byte(cpu, bus) as i8
}
