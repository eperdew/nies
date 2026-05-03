//! 6502 opcode dispatch and instruction handlers.
//!
//! Per spec §3.4 we use a function-table dispatch on opcode byte. To make
//! the production Cpu and the SingleStepTests harness share code, we
//! parameterize handlers on `B: BusLike`. Generic function pointers can't
//! live in a static table, so we dispatch via a `match` switch instead.
//! The match is monomorphized per concrete bus type at compile time.

use crate::bus::BusLike;
use crate::cpu::Cpu;
use crate::cpu::flags;

pub fn dispatch<B: BusLike>(opcode: u8, cpu: &mut Cpu, bus: &mut B) {
    match opcode {
        0xA9 => lda_imm(cpu, bus),
        _ => panic!(
            "CPU executed unimplemented opcode ${opcode:02X} at PC=${:04X}",
            cpu.pc.wrapping_sub(1)
        ),
    }
}

fn set_nz(cpu: &mut Cpu, val: u8) {
    cpu.p &= !(flags::FLAG_N | flags::FLAG_Z);
    if val == 0 {
        cpu.p |= flags::FLAG_Z;
    }
    if val & 0x80 != 0 {
        cpu.p |= flags::FLAG_N;
    }
}

fn lda_imm<B: BusLike>(cpu: &mut Cpu, bus: &mut B) {
    let val = bus.read(cpu.pc);
    cpu.pc = cpu.pc.wrapping_add(1);
    cpu.a = val;
    set_nz(cpu, val);
}
