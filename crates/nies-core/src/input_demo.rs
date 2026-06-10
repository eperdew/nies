//! Test-support for the M4 input determinism gate: a hand-assembled
//! micro-ROM that polls both controller ports every NMI into a RAM ring
//! buffer, plus the scripted input sequence and hash the native and wasm
//! gates share. Lives in the lib (like `demo_rom_bytes`) so
//! `crates/nies-core/tests/input_determinism.rs` and
//! `crates/nies-web/tests/determinism_wasm.rs` use identical bytes.
//! See the M4 design spec §6.3.

use crate::input::Buttons;
use crate::nes::Nes;
use std::hash::{DefaultHasher, Hasher};

/// Frames the gate runs. The reset stub burns ~3 frames waiting out PPU
/// warm-up before the first NMI poll; the script's first press is at
/// frame 6 so a released-state poll is always observed first.
pub const DEMO_FRAMES: u64 = 24;

/// One scripted step: before running frame `before_frame`, set `port`
/// to `buttons`.
pub struct ScriptStep {
    pub before_frame: u64,
    pub port: u8,
    pub buttons: Buttons,
}

/// The gate's input script. Every held state lasts ≥ 2 frames (so each
/// is observed by at least one NMI poll) except the final SELECT
/// press+release pair, which lands within one poll interval on purpose:
/// it must appear in the journal but never in the ring buffer.
pub fn script() -> Vec<ScriptStep> {
    fn step(before_frame: u64, buttons: Buttons) -> ScriptStep {
        ScriptStep {
            before_frame,
            port: 0,
            buttons,
        }
    }
    vec![
        step(6, Buttons::A),
        step(8, Buttons::A | Buttons::RIGHT),
        step(10, Buttons::default()),
        step(12, Buttons::START),
        step(14, Buttons::default()),
        step(16, Buttons::DOWN), // held 3 frames
        step(19, Buttons::default()),
        step(20, Buttons::SELECT),    // press + release within one frame:
        step(20, Buttons::default()), // journaled, never polled
    ]
}

/// Build the micro-ROM: iNES header, 16 KiB PRG (code below, vectors at
/// the top), 8 KiB zero CHR. Assembly listing in the M4 plan, Task 5.
pub fn input_demo_rom() -> Vec<u8> {
    #[rustfmt::skip]
    const CODE: &[u8] = &[
        // reset @ $8000
        0x78,                   // SEI
        0xD8,                   // CLD
        0xA2, 0xFF,             // LDX #$FF
        0x9A,                   // TXS
        0x2C, 0x02, 0x20,       // BIT $2002    ; wait vblank 1
        0x10, 0xFB,             // BPL *-5
        0x2C, 0x02, 0x20,       // BIT $2002    ; wait vblank 2
        0x10, 0xFB,             // BPL *-5
        0xA9, 0x80,             // LDA #$80
        0x8D, 0x00, 0x20,       // STA $2000    ; enable NMI
        0x4C, 0x14, 0x80,       // JMP $8014    ; idle
        // nmi @ $8017
        0xA9, 0x01,             // LDA #$01
        0x8D, 0x16, 0x40,       // STA $4016    ; strobe on
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x16, 0x40,       // STA $4016    ; strobe off
        0xA2, 0x08,             // LDX #$08
        0xAD, 0x16, 0x40,       // LDA $4016    ; port-0 loop
        0x4A,                   // LSR A
        0x26, 0x00,             // ROL $00
        0xCA,                   // DEX
        0xD0, 0xF7,             // BNE *-9
        0xA2, 0x08,             // LDX #$08
        0xAD, 0x17, 0x40,       // LDA $4017    ; port-1 loop
        0x4A,                   // LSR A
        0x26, 0x01,             // ROL $01
        0xCA,                   // DEX
        0xD0, 0xF7,             // BNE *-9
        0xA6, 0x02,             // LDX $02      ; ring index
        0xA5, 0x00,             // LDA $00
        0x9D, 0x00, 0x03,       // STA $0300,X
        0xE8,                   // INX
        0xA5, 0x01,             // LDA $01
        0x9D, 0x00, 0x03,       // STA $0300,X
        0xE8,                   // INX
        0x86, 0x02,             // STX $02
        0x40,                   // RTI
    ];
    let mut prg = vec![0u8; 16 * 1024];
    prg[..CODE.len()].copy_from_slice(CODE);
    // Vectors at $FFFA-$FFFF (PRG offset 0x3FFA, NROM-128 mirror).
    prg[0x3FFA] = 0x17; // NMI   -> $8017
    prg[0x3FFB] = 0x80;
    prg[0x3FFC] = 0x00; // RESET -> $8000
    prg[0x3FFD] = 0x80;
    prg[0x3FFE] = 0x14; // IRQ   -> $8014 (idle; never fires)
    prg[0x3FFF] = 0x80;

    let mut rom = Vec::with_capacity(16 + prg.len() + 8 * 1024);
    // iNES header: "NES\x1A", 1×16K PRG, 1×8K CHR, flags all zero
    // (mapper 0, horizontal mirroring).
    rom.extend_from_slice(&[
        0x4E, 0x45, 0x53, 0x1A, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ]);
    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&[0u8; 8 * 1024]);
    rom
}

/// Run the scripted sequence on a fresh `Nes` and return it.
pub fn run_input_demo() -> Nes {
    let mut nes = Nes::from_rom_bytes(&input_demo_rom()).expect("input demo ROM builds");
    let steps = script();
    for frame in 0..DEMO_FRAMES {
        for s in steps.iter().filter(|s| s.before_frame == frame) {
            nes.set_buttons(s.port, s.buttons);
        }
        nes.run_frame();
    }
    nes
}

/// The gate value: hash of CPU RAM + the index framebuffer after the
/// scripted run. `Hasher::write` over raw bytes — never `.hash()`, whose
/// usize length prefix differs between native (8 bytes) and wasm32
/// (4 bytes). See crates/nies-core/tests/ppu_determinism.rs.
pub fn run_and_hash() -> u64 {
    let nes = run_input_demo();
    let mut h = DefaultHasher::new();
    h.write(nes.ram());
    h.write(nes.frame());
    h.finish()
}
