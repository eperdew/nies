//! Per-opcode integration tests driven by the SingleStepTests/65x02 corpus.

mod common;

use common::{TestCase, load_opcode_cases};
use nies_core::bus::BusLike;

pub struct FlatBus {
    pub mem: [u8; 0x10000],
    pub trace: Vec<(u16, u8, &'static str)>,
}

impl FlatBus {
    fn new() -> Self {
        FlatBus {
            mem: [0u8; 0x10000],
            trace: Vec::with_capacity(16),
        }
    }
}

impl BusLike for FlatBus {
    fn read(&mut self, addr: u16) -> u8 {
        let val = self.mem[addr as usize];
        self.trace.push((addr, val, "read"));
        val
    }
    fn write(&mut self, addr: u16, val: u8) {
        self.mem[addr as usize] = val;
        self.trace.push((addr, val, "write"));
    }
}

pub fn run_opcode_tests(opcode: u8) {
    let cases = load_opcode_cases(opcode);
    let mut failures = 0usize;
    let mut first_failure: Option<String> = None;

    for case in &cases {
        match run_single_case(case) {
            Ok(()) => {}
            Err(msg) => {
                failures += 1;
                if first_failure.is_none() {
                    first_failure = Some(format!("case '{}': {msg}", case.name));
                }
            }
        }
    }

    if failures > 0 {
        panic!(
            "opcode ${opcode:02X}: {failures}/{} cases failed.\nFirst failure: {}",
            cases.len(),
            first_failure.as_deref().unwrap_or("?")
        );
    }
}

fn run_single_case(case: &TestCase) -> Result<(), String> {
    let mut bus = FlatBus::new();
    for &(addr, val) in &case.initial.ram {
        bus.mem[addr as usize] = val;
    }
    let mut cpu = nies_core::cpu::Cpu::new();
    cpu.pc = case.initial.pc;
    cpu.sp = case.initial.s;
    cpu.a = case.initial.a;
    cpu.x = case.initial.x;
    cpu.y = case.initial.y;
    cpu.p = case.initial.p;

    cpu.step(&mut bus);

    if cpu.pc != case.r#final.pc {
        return Err(format!(
            "PC: expected {:04X}, got {:04X}",
            case.r#final.pc, cpu.pc
        ));
    }
    if cpu.sp != case.r#final.s {
        return Err(format!(
            "S: expected {:02X}, got {:02X}",
            case.r#final.s, cpu.sp
        ));
    }
    if cpu.a != case.r#final.a {
        return Err(format!(
            "A: expected {:02X}, got {:02X}",
            case.r#final.a, cpu.a
        ));
    }
    if cpu.x != case.r#final.x {
        return Err(format!(
            "X: expected {:02X}, got {:02X}",
            case.r#final.x, cpu.x
        ));
    }
    if cpu.y != case.r#final.y {
        return Err(format!(
            "Y: expected {:02X}, got {:02X}",
            case.r#final.y, cpu.y
        ));
    }
    if cpu.p != case.r#final.p {
        return Err(format!(
            "P: expected {:02X}, got {:02X}",
            case.r#final.p, cpu.p
        ));
    }
    for (addr, expected) in &case.r#final.ram {
        let got = bus.mem[*addr as usize];
        if got != *expected {
            return Err(format!(
                "ram[{addr:04X}]: expected {expected:02X}, got {got:02X}"
            ));
        }
    }
    if bus.trace.len() != case.cycles.len() {
        return Err(format!(
            "cycle count: expected {}, got {}",
            case.cycles.len(),
            bus.trace.len()
        ));
    }
    for (i, (expected, actual)) in case.cycles.iter().zip(bus.trace.iter()).enumerate() {
        if expected.0 != actual.0 || expected.1 != actual.1 || expected.2 != actual.2 {
            return Err(format!(
                "cycle {i}: expected ({:04X}, {:02X}, {}), got ({:04X}, {:02X}, {})",
                expected.0, expected.1, expected.2, actual.0, actual.1, actual.2
            ));
        }
    }
    Ok(())
}

#[test]
fn opcode_a9_lda_imm() {
    run_opcode_tests(0xA9);
}
