//! Nestest "automated mode": run nestest.nes from PC=$C000 and compare
//! every per-instruction state against the Nintendulator log file.

use nies_core::bus::Bus;
use nies_core::cartridge::Cartridge;
use nies_core::cpu::Cpu;
use nies_core::mapper::MapperKind;

const ROM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/roms/nestest/nestest.nes"
);
const LOG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/roms/nestest/nestest.log"
);

#[derive(Debug)]
struct LogLine {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    sp: u8,
}

fn parse_log_line(line: &str) -> LogLine {
    fn hex2(s: &str, prefix: &str) -> u8 {
        let idx = s.find(prefix).expect("prefix in line");
        let start = idx + prefix.len();
        u8::from_str_radix(&s[start..start + 2], 16).expect("hex")
    }
    fn hex4(s: &str) -> u16 {
        u16::from_str_radix(&s[0..4], 16).expect("hex")
    }
    LogLine {
        pc: hex4(line),
        a: hex2(line, "A:"),
        x: hex2(line, "X:"),
        y: hex2(line, "Y:"),
        p: hex2(line, "P:"),
        sp: hex2(line, "SP:"),
    }
}

#[test]
fn nestest_matches_nintendulator_log() {
    let bytes = std::fs::read(ROM).expect("read nestest.nes");
    let cart = Cartridge::from_bytes(&bytes).expect("parse nestest.nes");
    let mapper = MapperKind::from_cartridge(&cart).expect("build mapper");
    let mut bus = Bus::new(mapper);
    let mut cpu = Cpu::new();
    // Nestest "automated mode" entry: skip the normal reset and jump
    // directly to $C000 with the canonical post-reset state nestest expects.
    cpu.pc = 0xC000;
    cpu.sp = 0xFD;
    cpu.p = 0x24;
    cpu.a = 0;
    cpu.x = 0;
    cpu.y = 0;

    let log = std::fs::read_to_string(LOG).expect("read nestest.log");
    let mut last_line_no = 0usize;
    for (idx, raw) in log.lines().enumerate() {
        let line_no = idx + 1;
        if raw.trim().is_empty() {
            continue;
        }
        last_line_no = line_no;
        let expected = parse_log_line(raw);

        if cpu.pc != expected.pc
            || cpu.a != expected.a
            || cpu.x != expected.x
            || cpu.y != expected.y
            || cpu.p != expected.p
            || cpu.sp != expected.sp
        {
            panic!(
                "nestest divergence at log line {line_no}:\n  expected PC={:04X} A={:02X} X={:02X} Y={:02X} P={:02X} SP={:02X}\n  actual   PC={:04X} A={:02X} X={:02X} Y={:02X} P={:02X} SP={:02X}",
                expected.pc,
                expected.a,
                expected.x,
                expected.y,
                expected.p,
                expected.sp,
                cpu.pc,
                cpu.a,
                cpu.x,
                cpu.y,
                cpu.p,
                cpu.sp
            );
        }
        cpu.step(&mut bus);
    }
    println!("nestest: {last_line_no} log lines matched");
}
