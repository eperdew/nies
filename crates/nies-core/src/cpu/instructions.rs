//! 6502 opcode dispatch table and instruction handlers.

use crate::bus::Bus;
use crate::cpu::Cpu;

/// Type of an instruction handler. Takes the CPU, the bus, and the
/// already-fetched opcode byte; runs the instruction to completion via
/// `bus.read` / `bus.write`. The CPU's PC has already been advanced past
/// the opcode byte by the dispatch loop.
pub type InstrFn = fn(&mut Cpu, &mut Bus);

/// Dispatch table indexed by opcode byte. Filled in incrementally per
/// opcode family. Entries default to `unimplemented_opcode`.
pub static OPCODES: [InstrFn; 256] = build_table();

const fn build_table() -> [InstrFn; 256] {
    let mut t: [InstrFn; 256] = [unimplemented_opcode; 256];
    // Real opcode wiring happens below via individual `t[0xNN] = handler`
    // assignments as each family lands.
    let _ = &mut t; // suppress "value assigned to is never read" lint when no entries are wired.
    t
}

fn unimplemented_opcode(cpu: &mut Cpu, _bus: &mut Bus) {
    // PC has been advanced past the opcode byte, so PC-1 points at it.
    let opcode = cpu.pc.wrapping_sub(1);
    panic!("CPU executed an unimplemented opcode at PC={opcode:04X}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_table_has_256_entries() {
        let _entry = OPCODES[0x00];
        let _entry = OPCODES[0xFF];
    }
}
