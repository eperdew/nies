//! Integration test harness for blargg-style and nestest test ROMs.
//!
//! ROMs that follow the blargg "$6000 status protocol" use the
//! battery-backed PRG-RAM region $6000-$60FF as a status surface:
//!
//! - `$6000`: status code; `$80` = running, `$00` = success (once the
//!   magic-ready bytes are set), any other byte = failure
//! - `$6001-$6003`: magic-ready signature `$DE $B0 $61` — written by
//!   the ROM once it has reached its main test loop
//! - `$6004+`: null-terminated ASCII status message
//!
//! `run_test_rom` loads a ROM, builds a `Cartridge` + `Bus` + `Cpu`,
//! resets, and steps the CPU until either the ROM signals completion
//! via the status protocol or a cycle budget is exhausted.

use nies_core::bus::Bus;
use nies_core::cartridge::Cartridge;
use nies_core::cpu::Cpu;
use nies_core::mapper::MapperKind;

/// Outcome of running a test ROM.
#[derive(Debug)]
pub enum RomResult {
    Done { status: u8, message: String },
    Timeout,
}

/// Load a ROM file, run it through the CPU until completion or cycle
/// budget exhaustion. See module docs for the $6000 protocol.
pub fn run_test_rom(path: &str, max_cycles: u64) -> RomResult {
    let bytes = std::fs::read(path).expect("read rom");
    let cart = Cartridge::from_bytes(&bytes).expect("parse rom");
    let mapper = MapperKind::from_cartridge(&cart).expect("build mapper");
    let mut bus = Bus::new(mapper);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    // Wait for the ROM to set the magic ready handshake at $6001-$6003,
    // then watch $6000 for a non-running status code.
    let start_cycle = bus.cycle;
    while bus.cycle - start_cycle < max_cycles {
        cpu.step(&mut bus);
        let status = bus.peek(0x6000);
        if status != 0x80 && status != 0x00 {
            return read_result(&bus, status);
        }
        // Special case: status 0x00 *with* magic-ready bytes set means success.
        if status == 0x00
            && bus.peek(0x6001) == 0xDE
            && bus.peek(0x6002) == 0xB0
            && bus.peek(0x6003) == 0x61
        {
            return read_result(&bus, 0x00);
        }
    }
    RomResult::Timeout
}

fn read_result(bus: &Bus, status: u8) -> RomResult {
    let mut s = String::new();
    for offset in 0u16..0xFC {
        let b = bus.peek(0x6004 + offset);
        if b == 0 {
            break;
        }
        s.push(b as char);
    }
    RomResult::Done { status, message: s }
}
