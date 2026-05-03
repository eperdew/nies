# M1 — CPU + Cartridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the 6502 CPU (all 256 opcodes including illegals), the iNES/NES2.0 cartridge parser, the NROM mapper (CPU side), and the `Bus` skeleton with stub PPU/APU and tick discipline. Vendor the SingleStepTests/65x02 corpus and the blargg/nestest test ROMs and wire them up as integration tests.

**Architecture:** Bus-tick architecture per spec §3.3: `Cpu::step(&mut self, bus: &mut Bus)` executes one instruction; every memory access goes through `Bus::read` / `Bus::write` which tick the rest of the system one CPU cycle (3 PPU dots placeholder, 1 APU step placeholder, cycle counter increment, DMC fetch service hook). The CPU is built test-first against the SingleStepTests/65x02 corpus, then validated end-to-end against the blargg cpu_* test ROMs and `nestest.nes`.

**Tech Stack:** Rust 2024 edition, serde + postcard for snapshots (basic derives only at M1; full snapshot machinery lands later), the SingleStepTests/65x02 corpus (vendored compressed), blargg test ROMs and nestest (vendored). No new external crate dependencies beyond what M0 introduced.

---

## Reference: spec section ↔ task mapping

| Spec § | Requirement | Tasks |
|---|---|---|
| §3.2 | `nies-core` module layout (cpu/, bus.rs, mapper/, cartridge.rs, etc.) | 1, 2, 3, 4, 5 |
| §3.3 | `Nes` struct + bus tick discipline | 6, 7, 8, 9 |
| §3.4 | CPU: 256-entry dispatch, all official + illegal opcodes, IRQ/NMI sampling, dummy reads, no BCD | 12, 13–37, 38–43, 44–46 |
| §3.7 | `MapperImpl` trait + `MapperKind` enum dispatch (NROM only at M1) | 4, 5 |
| §4.2 | Power-on state (Mesen pattern), CPU reset state | 6, 44 |
| §7.1 | SingleStepTests/65x02 unit tests | 11, 12, 47 |
| §7.2 | Test ROM integration tests with $6000 status protocol | 38–46, 48 |
| §8 (M1 gate) | nestest, blargg cpu_instrs/instr_misc/instr_timing/cpu_dummy_reads green | 46, 48 |

---

## Notes for the implementer

- **Working directory** is `/Users/eperdew/Software/nies`. M0 is merged to `master`. Create a feature branch `m1-cpu-cartridge` before starting.
- **Existing state from M0:** four-crate Cargo workspace (`nies-core`, `nies-ui`, `nies-app`, `nies-web`); native binary opens a wgpu window with a sentinel color; WASM build via Trunk; CI on macOS/Linux/Windows + WASM. `nies-core/src/lib.rs` currently has only a placeholder `tests::workspace_smoke` test.
- **TDD discipline.** Every opcode is implemented test-first against the SingleStepTests/65x02 per-opcode test corpus. The pattern for every opcode task is: (1) extract the opcode's JSON file from the vendored tarball, (2) write a test that runs each test case in that file and asserts final state + bus access list match, (3) confirm the test fails (opcode unimplemented), (4) implement the opcode, (5) re-run and confirm pass.
- **Commit cadence.** One commit per task. Each opcode-family task may bundle ~3-8 closely-related opcodes that share addressing-mode machinery (e.g., AND/ORA/EOR are all "load operand, do bitwise op, write A, set N+Z").
- **Avoid hidden plumbing.** New helper functions used in later tasks must be defined in or before the task that first uses them. If you find yourself thinking "I'll add this in the next task," stop and add it as a dedicated step.
- **No PPU, APU, or mapper IRQ behavior at M1.** The PPU/APU stubs in this milestone exist only so the bus tick has somewhere to call. Real PPU/APU behavior lands in M2/M5.
- **No save state machinery yet.** `serde::Serialize` / `Deserialize` derives can be added on `Cpu`, `Bus`, etc., but the actual snapshot/restore logic (with header, ROM hash, postcard wire format) is M7. At M1 we only need round-trip via `bincode`-equivalent for unit testing.
- **SingleStepTests/65x02 vendoring:** the corpus is ~50 MB raw JSON. Vendor as a single zstd-compressed tarball under `crates/nies-core/tests/data/65x02.tar.zst` (~6-10 MB). The test setup decompresses to `target/test-cache/65x02/` on first run; a directory-existence check skips re-extraction.
- **Test ROM licensing:** All test ROMs vendored at M1 are public domain or explicitly redistributable. Update `LICENSES.md` accordingly in Task 50.
- **Performance expectation:** at M1 the CPU + bus stack should run at well over 50× real-time on a modern machine when running headless integration tests; SingleStepTests and blargg ROMs should complete in under 30 seconds combined. If anything regresses below ~10× real-time, profile.
- **`cargo fmt` and `cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings`** must remain clean after every task. CI gates these.
- **Working tree after every task:** clean (`git status` shows nothing). Cargo.lock updates land with the task that introduces the new dependency.

---

## File map

New files this milestone introduces (full set; created across multiple tasks):

```
crates/nies-core/
├── src/
│   ├── lib.rs                       (modified — re-export structure)
│   ├── nes.rs                       (top-level Nes struct, frame/step driver — minimal at M1)
│   ├── bus.rs                       (Bus struct: RAM, mapper, cycle counter, tick discipline, DMC fetch hook)
│   ├── cartridge.rs                 (iNES + NES2.0 parser, Cartridge struct)
│   ├── input.rs                     (Controller stub — bare-bones at M1, just for $4016/$4017 stub reads)
│   ├── mapper/
│   │   ├── mod.rs                   (MapperImpl trait, MapperKind enum)
│   │   └── nrom.rs                  (NROM / mapper 0)
│   ├── cpu/
│   │   ├── mod.rs                   (Cpu struct, registers, flags, public step interface)
│   │   ├── addressing.rs            (addressing-mode resolution helpers)
│   │   ├── instructions.rs          (256-entry dispatch table, instruction implementations)
│   │   └── flags.rs                 (status flag constants and helpers)
│   ├── ppu/
│   │   └── mod.rs                   (Ppu stub — has step() that no-ops)
│   ├── apu/
│   │   ├── mod.rs                   (Apu stub — has step() that no-ops; DMC fetch hooks)
│   │   └── dmc.rs                   (DMC pending-fetch state struct, no actual sample generation yet)
│   └── snapshot.rs                  (placeholder module — actual format in M7)
└── tests/
    ├── data/
    │   └── 65x02.tar.zst            (vendored SingleStepTests corpus, ~6-10 MB compressed)
    ├── roms/
    │   ├── nestest/
    │   │   ├── nestest.nes          (vendored, ~24 KB)
    │   │   └── nestest.log          (Nintendulator reference log, ~8 MB)
    │   ├── blargg/
    │   │   ├── cpu_instrs/
    │   │   │   ├── cpu_instrs.nes
    │   │   │   ├── 01-basics.nes
    │   │   │   ├── 02-implied.nes
    │   │   │   ├── 03-immediate.nes
    │   │   │   ├── 04-zero_page.nes
    │   │   │   ├── 05-zp_xy.nes
    │   │   │   ├── 06-absolute.nes
    │   │   │   ├── 07-abs_xy.nes
    │   │   │   ├── 08-ind_x.nes
    │   │   │   ├── 09-ind_y.nes
    │   │   │   ├── 10-branches.nes
    │   │   │   ├── 11-stack.nes
    │   │   │   ├── 12-jmp_jsr.nes
    │   │   │   ├── 13-rts.nes
    │   │   │   ├── 14-rti.nes
    │   │   │   ├── 15-brk.nes
    │   │   │   └── 16-special.nes
    │   │   ├── instr_misc.nes
    │   │   ├── instr_timing/
    │   │   │   ├── instr_timing.nes
    │   │   │   ├── 1-instr_timing.nes
    │   │   │   └── 2-branch_timing.nes
    │   │   └── cpu_dummy_reads.nes
    │   └── manifest.toml             (per-ROM expected outcome + frame budget)
    ├── singlestep_tests.rs           (integration test runner for vendored 65x02 corpus)
    ├── test_roms.rs                  (integration test runner for blargg + nestest)
    └── nestest_compare.rs            (nestest-vs-Nintendulator-log byte-comparison test)
```

Files modified:

- `crates/nies-core/src/lib.rs` — re-exports of public types
- `crates/nies-core/Cargo.toml` — adds `serde`, `bincode`/`postcard`, `zstd`, `sha2` dev-dependencies as needed
- `Cargo.toml` (workspace) — adds `serde`, `bincode`/`postcard` to `[workspace.dependencies]`
- `LICENSES.md` — adds test ROM provenance section
- `Trunk.toml`, `crates/nies-app`, `crates/nies-web`, `crates/nies-ui` — untouched

---

## Phase A — Foundation (Tasks 1–10)

Build the workspace's data structures, bus, and stubs in dependency order. Nothing executes yet.

### Task 1: Create m1 feature branch and the new module skeleton

**Files:**
- New (empty): `crates/nies-core/src/bus.rs`, `cartridge.rs`, `input.rs`, `snapshot.rs`
- New (empty modules with `mod.rs`): `crates/nies-core/src/cpu/`, `crates/nies-core/src/mapper/`, `crates/nies-core/src/ppu/`, `crates/nies-core/src/apu/`
- Modify: `crates/nies-core/src/lib.rs`

- [ ] **Step 1: Branch off master**

```bash
git checkout master
git checkout -b m1-cpu-cartridge
git status   # working tree clean
```

- [ ] **Step 2: Create the directory skeleton**

```bash
mkdir -p crates/nies-core/src/cpu crates/nies-core/src/mapper crates/nies-core/src/ppu crates/nies-core/src/apu
```

- [ ] **Step 3: Write empty module files**

Write `crates/nies-core/src/bus.rs`:

```rust
//! CPU bus. Exposes `Bus::read` and `Bus::write`, both of which tick the
//! rest of the system (PPU/APU/mapper) one CPU cycle on every access.
//! See spec §3.3.
```

Write `crates/nies-core/src/cartridge.rs`:

```rust
//! iNES / NES 2.0 ROM file parser. See spec §3.2.
```

Write `crates/nies-core/src/input.rs`:

```rust
//! Controller state. Polled by CPU $4016/$4017 reads. See spec §5.5.
```

Write `crates/nies-core/src/snapshot.rs`:

```rust
//! Save state serialization. Full implementation lands in M7 (spec §4.3).
//! At M1 this module exists only so the type tree can derive serde traits
//! that the future snapshot logic will consume.
```

Write `crates/nies-core/src/cpu/mod.rs`:

```rust
//! 6502 CPU implementation. See spec §3.4.

pub mod addressing;
pub mod flags;
pub mod instructions;
```

Write `crates/nies-core/src/cpu/addressing.rs`, `flags.rs`, `instructions.rs` as empty (each containing only a leading `//! ...` doc comment).

Write `crates/nies-core/src/mapper/mod.rs`:

```rust
//! Mapper trait + variant enum. NROM only at M1 (spec §3.7).

pub mod nrom;
```

Write `crates/nies-core/src/mapper/nrom.rs` as empty (`//! NROM / mapper 0.`).

Write `crates/nies-core/src/ppu/mod.rs`:

```rust
//! PPU stub. Real implementation lands in M2 (spec §3.5).
```

Write `crates/nies-core/src/apu/mod.rs`:

```rust
//! APU stub. Real implementation lands in M5 (spec §3.6).

pub mod dmc;
```

Write `crates/nies-core/src/apu/dmc.rs` as empty (`//! DMC channel state. Real implementation lands in M5.`).

- [ ] **Step 4: Wire up `lib.rs`**

Replace `crates/nies-core/src/lib.rs` contents with:

```rust
//! `nies-core` — NES emulator backend (CPU, PPU, APU, mappers, save state, debugger).
//!
//! No I/O dependencies: this crate must remain free of `std::fs`, `std::time::SystemTime`,
//! audio/video device access, and threading. The deterministic emulator core lives here.

pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod input;
pub mod mapper;
pub mod ppu;
pub mod snapshot;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_smoke() {
        assert_eq!(2 + 2, 4);
    }
}
```

- [ ] **Step 5: Verify the workspace still builds**

```bash
cargo build --workspace --exclude nies-web
cargo test -p nies-core
```

Expected: builds cleanly, the `workspace_smoke` test passes.

- [ ] **Step 6: Commit**

```bash
git add crates/nies-core/src
git commit -m "$(cat <<'EOF'
feat(core): add module skeleton for M1

Empty placeholder modules for cpu, bus, cartridge, mapper, ppu, apu,
input, and snapshot. Each carries a top-level doc comment describing
its role per the design spec; concrete implementations land in
subsequent M1 tasks.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Cartridge struct + iNES header parser

The cartridge file format: 16-byte header followed by PRG-ROM, CHR-ROM, and (optionally) trainer/playchoice data. iNES vs NES 2.0 is determined by header byte 7's bits 2-3. We parse both.

**Files:**
- Modify: `crates/nies-core/src/cartridge.rs`

- [ ] **Step 1: Write the Cartridge type and a unit test for header rejection**

Append to `crates/nies-core/src/cartridge.rs`:

```rust
use std::error::Error;
use std::fmt;

/// Mirroring scheme as encoded in the iNES header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    /// Vertical mirroring (horizontal arrangement).
    Vertical,
    /// Horizontal mirroring (vertical arrangement).
    Horizontal,
    /// Four-screen VRAM provided by the cartridge.
    FourScreen,
    /// Single-screen mirroring (used by some mappers; lower or upper bank).
    SingleScreen,
}

/// Format of the loaded ROM file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NesFormat {
    INes,
    Nes2_0,
}

/// A parsed NES cartridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cartridge {
    pub format: NesFormat,
    pub mapper_id: u16,
    pub submapper_id: u8,
    pub mirroring: Mirroring,
    pub has_battery: bool,
    pub has_trainer: bool,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub prg_ram_size: u32,
    pub chr_ram_size: u32,
}

/// Errors produced while parsing an iNES / NES 2.0 ROM file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CartridgeError {
    TooShort,
    BadMagic,
    PrgRomSizeOverflow,
    ChrRomSizeOverflow,
    UnsupportedMapper(u16),
    Truncated { expected: usize, got: usize },
}

impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "ROM file shorter than the 16-byte iNES header"),
            Self::BadMagic => write!(f, "ROM does not start with the iNES magic bytes 'NES\\x1A'"),
            Self::PrgRomSizeOverflow => write!(f, "PRG-ROM size exceeds u32::MAX bytes"),
            Self::ChrRomSizeOverflow => write!(f, "CHR-ROM size exceeds u32::MAX bytes"),
            Self::UnsupportedMapper(id) => write!(f, "mapper {id} is not supported (M1 only ships NROM / mapper 0)"),
            Self::Truncated { expected, got } => write!(
                f,
                "ROM file truncated: header declares {expected} bytes of PRG/CHR data, got {got}"
            ),
        }
    }
}

impl Error for CartridgeError {}

impl Cartridge {
    pub const INES_MAGIC: [u8; 4] = *b"NES\x1A";

    /// Parse a ROM file from raw bytes. Accepts both iNES (legacy) and NES 2.0 headers.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CartridgeError> {
        if bytes.len() < 16 {
            return Err(CartridgeError::TooShort);
        }
        if bytes[0..4] != Self::INES_MAGIC {
            return Err(CartridgeError::BadMagic);
        }

        // iNES header layout (per https://www.nesdev.org/wiki/INES):
        //   0..4   magic
        //   4      PRG-ROM size in 16 KiB units (low byte; high in NES 2.0)
        //   5      CHR-ROM size in 8 KiB units (low byte; high in NES 2.0)
        //   6      Flags 6:  mapper-low, mirroring, battery, trainer, four-screen
        //   7      Flags 7:  mapper-mid, NES 2.0 indicator, vs/playchoice
        //   8..16  varies by format
        let flags6 = bytes[6];
        let flags7 = bytes[7];

        let format = if (flags7 & 0x0C) == 0x08 {
            NesFormat::Nes2_0
        } else {
            NesFormat::INes
        };

        let prg_units_low = bytes[4] as u16;
        let chr_units_low = bytes[5] as u16;

        let (prg_units, chr_units, mapper_id, submapper_id, prg_ram_size, chr_ram_size) =
            match format {
                NesFormat::INes => {
                    let mapper_id = ((flags7 & 0xF0) as u16) | ((flags6 >> 4) as u16);
                    let prg_ram_size = if bytes[8] == 0 { 8192 } else { bytes[8] as u32 * 8192 };
                    (prg_units_low, chr_units_low, mapper_id, 0, prg_ram_size, 0)
                }
                NesFormat::Nes2_0 => {
                    // NES 2.0 augments header bytes 9-15.
                    let prg_high = (bytes[9] & 0x0F) as u16;
                    let chr_high = ((bytes[9] >> 4) & 0x0F) as u16;
                    let mapper_id =
                        ((bytes[8] & 0x0F) as u16) << 8 | ((flags7 & 0xF0) as u16) | ((flags6 >> 4) as u16);
                    let submapper_id = (bytes[8] >> 4) & 0x0F;
                    // Section "PRG-RAM (volatile/non-volatile) size":
                    let prg_ram_size = nes2_size_unit(bytes[10] & 0x0F);
                    let chr_ram_size = nes2_size_unit(bytes[11] & 0x0F);
                    (
                        (prg_high << 8) | prg_units_low,
                        (chr_high << 8) | chr_units_low,
                        mapper_id,
                        submapper_id,
                        prg_ram_size,
                        chr_ram_size,
                    )
                }
            };

        let prg_rom_bytes = (prg_units as usize)
            .checked_mul(16 * 1024)
            .ok_or(CartridgeError::PrgRomSizeOverflow)?;
        let chr_rom_bytes = (chr_units as usize)
            .checked_mul(8 * 1024)
            .ok_or(CartridgeError::ChrRomSizeOverflow)?;

        let has_trainer = (flags6 & 0x04) != 0;
        let has_battery = (flags6 & 0x02) != 0;

        let four_screen = (flags6 & 0x08) != 0;
        let vertical = (flags6 & 0x01) != 0;
        let mirroring = if four_screen {
            Mirroring::FourScreen
        } else if vertical {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let mut offset = 16usize;
        if has_trainer {
            offset += 512; // 512-byte trainer immediately after header
        }

        let need = offset + prg_rom_bytes + chr_rom_bytes;
        if bytes.len() < need {
            return Err(CartridgeError::Truncated {
                expected: need,
                got: bytes.len(),
            });
        }

        let prg_rom = bytes[offset..offset + prg_rom_bytes].to_vec();
        offset += prg_rom_bytes;
        let chr_rom = bytes[offset..offset + chr_rom_bytes].to_vec();

        Ok(Cartridge {
            format,
            mapper_id,
            submapper_id,
            mirroring,
            has_battery,
            has_trainer,
            prg_rom,
            chr_rom,
            prg_ram_size,
            chr_ram_size,
        })
    }

    /// SHA-256 hash of the cartridge's PRG-ROM concatenated with CHR-ROM. Used as
    /// a save-state key in M7; defined here because the cartridge owns the data.
    pub fn rom_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&self.prg_rom);
        hasher.update(&self.chr_rom);
        hasher.finalize().into()
    }
}

/// Decode an NES 2.0 size unit (4-bit nibble) into bytes. Per the spec, value 0 means
/// "no PRG-RAM present"; values 1..=14 encode `64 << v` bytes; value 15 is reserved.
fn nes2_size_unit(nibble: u8) -> u32 {
    match nibble {
        0 => 0,
        v if v <= 14 => 64u32 << v,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_minimal_ines(prg_units: u8, chr_units: u8, flags6: u8, flags7: u8) -> Vec<u8> {
        let mut buf = vec![0u8; 16];
        buf[0..4].copy_from_slice(&Cartridge::INES_MAGIC);
        buf[4] = prg_units;
        buf[5] = chr_units;
        buf[6] = flags6;
        buf[7] = flags7;
        buf.resize(16 + prg_units as usize * 16 * 1024 + chr_units as usize * 8 * 1024, 0);
        buf
    }

    #[test]
    fn rejects_short_rom() {
        let result = Cartridge::from_bytes(&[0u8; 8]);
        assert_eq!(result, Err(CartridgeError::TooShort));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = vec![0u8; 32];
        bytes[0..4].copy_from_slice(b"BORK");
        let result = Cartridge::from_bytes(&bytes);
        assert_eq!(result, Err(CartridgeError::BadMagic));
    }

    #[test]
    fn parses_minimal_nrom_ines() {
        let bytes = build_minimal_ines(2, 1, 0x00, 0x00); // 32 KiB PRG, 8 KiB CHR, mapper 0
        let cart = Cartridge::from_bytes(&bytes).unwrap();
        assert_eq!(cart.format, NesFormat::INes);
        assert_eq!(cart.mapper_id, 0);
        assert_eq!(cart.prg_rom.len(), 32 * 1024);
        assert_eq!(cart.chr_rom.len(), 8 * 1024);
        assert_eq!(cart.mirroring, Mirroring::Horizontal);
        assert!(!cart.has_battery);
        assert!(!cart.has_trainer);
    }

    #[test]
    fn vertical_mirroring_flag_round_trip() {
        let bytes = build_minimal_ines(1, 1, 0x01, 0x00);
        let cart = Cartridge::from_bytes(&bytes).unwrap();
        assert_eq!(cart.mirroring, Mirroring::Vertical);
    }

    #[test]
    fn battery_flag_round_trip() {
        let bytes = build_minimal_ines(1, 1, 0x02, 0x00);
        let cart = Cartridge::from_bytes(&bytes).unwrap();
        assert!(cart.has_battery);
    }

    #[test]
    fn truncated_rom_is_rejected() {
        // Header claims 1 unit of PRG (16 KiB) but supplies only the header.
        let mut bytes = vec![0u8; 16];
        bytes[0..4].copy_from_slice(&Cartridge::INES_MAGIC);
        bytes[4] = 1;
        let err = Cartridge::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CartridgeError::Truncated { .. }));
    }

    #[test]
    fn rom_hash_is_deterministic() {
        let bytes = build_minimal_ines(1, 1, 0x00, 0x00);
        let a = Cartridge::from_bytes(&bytes).unwrap().rom_hash();
        let b = Cartridge::from_bytes(&bytes).unwrap().rom_hash();
        assert_eq!(a, b);
    }
}
```

- [ ] **Step 2: Add the `sha2` dependency**

Edit `crates/nies-core/Cargo.toml`. Update `[dependencies]`:

```toml
[dependencies]
log.workspace = true
sha2 = "0.10"
```

- [ ] **Step 3: Run the tests**

```bash
cargo test -p nies-core --lib cartridge::
```

Expected: all 6 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/nies-core/src/cartridge.rs crates/nies-core/Cargo.toml Cargo.lock
git commit -m "$(cat <<'EOF'
feat(core): iNES + NES 2.0 cartridge parser

Parses the 16-byte header, decodes PRG/CHR ROM sizes (incl. NES 2.0's
high nibbles), mapper id, mirroring, and battery/trainer flags.
Validates magic bytes and total file length. Adds a SHA-256 rom_hash
helper that M7 will use as a save-state key.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: NROM mapper (CPU side)

NROM (mapper 0) is the simplest mapper: PRG-ROM is mapped to $8000-$FFFF (16 KiB games are mirrored at $8000 and $C000; 32 KiB games occupy the full range). PRG-RAM (when present) is mapped to $6000-$7FFF. CHR-ROM is mapped to PPU $0000-$1FFF. Writes to PRG-ROM are no-ops; writes to PRG-RAM go to a backing array.

**Files:**
- Modify: `crates/nies-core/src/mapper/nrom.rs`

- [ ] **Step 1: Write the NromState struct and basic CPU-side mapping tests**

Replace `crates/nies-core/src/mapper/nrom.rs` with:

```rust
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
        }
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF if !self.prg_ram.is_empty() => {
                self.prg_ram[(addr - 0x6000) as usize]
            }
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
        // The 32 KiB ROM occupies $8000-$FFFF linearly.
        assert_eq!(nrom.cpu_read(0x8000), 0);
        assert_eq!(nrom.cpu_read(0xC000), 0x80); // halfway through 32 KiB
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
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p nies-core --lib mapper::nrom::
```

Expected: all 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/nies-core/src/mapper/nrom.rs
git commit -m "$(cat <<'EOF'
feat(core): NROM mapper (CPU+PPU sides)

Implements iNES mapper 0:
- PRG-ROM at $8000-$FFFF, with 16 KiB games mirrored at $C000.
- 8 KiB PRG-RAM at $6000-$7FFF (read/write).
- CHR at PPU $0000-$1FFF (ROM read-only or RAM read/write depending
  on whether the cartridge supplied CHR-ROM).
- No bank switching, no IRQ.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: MapperImpl trait + MapperKind enum

The trait defines the mapper interface; the enum dispatches at compile time without virtual calls. At M1 the only variant is `Nrom(NromState)`; future mappers (MMC1, UxROM, CNROM, MMC3) add variants in their own milestones.

**Files:**
- Modify: `crates/nies-core/src/mapper/mod.rs`

- [ ] **Step 1: Define the trait + enum + dispatch**

Replace `crates/nies-core/src/mapper/mod.rs` with:

```rust
//! Mapper trait + variant enum. NROM only at M1 (spec §3.7).
//!
//! Mappers are dispatched through the `MapperKind` enum rather than
//! `Box<dyn>`. This avoids vtable lookups, serializes cleanly via serde
//! (no `typetag` dependency), and makes the closed set of supported
//! mappers explicit at the type level.

pub mod nrom;

use crate::cartridge::{Cartridge, CartridgeError, Mirroring};
use nrom::NromState;

/// Per-mapper interface. Spec §3.7.
pub trait MapperImpl {
    fn cpu_read(&mut self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, val: u8);
    fn ppu_read(&mut self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, val: u8);
    /// Per-cycle PPU A12 line level. Used by MMC3-class mappers; default
    /// no-op for mappers that don't track A12 transitions.
    fn notify_a12(&mut self, _level: bool) {}
    /// True if the mapper has an asserted IRQ line (e.g., MMC3 scanline counter).
    fn irq_pending(&self) -> bool { false }
    /// Current mirroring mode (some mappers can change this dynamically).
    fn mirroring(&self) -> Mirroring;
    /// Debug introspection. Default: empty list. Each mapper can return its
    /// internal register state for the debugger UI (M9).
    fn debug_dump(&self) -> Vec<(&'static str, u32)> { vec![] }
}

/// Closed set of supported mappers. Adding a new mapper means adding a
/// variant here and updating every `match` site in this file.
#[derive(Debug, Clone)]
pub enum MapperKind {
    Nrom(NromState),
}

impl MapperKind {
    /// Build a `MapperKind` from a parsed cartridge. Returns
    /// `CartridgeError::UnsupportedMapper` for any mapper id not yet implemented.
    pub fn from_cartridge(cart: &Cartridge) -> Result<Self, CartridgeError> {
        match cart.mapper_id {
            0 => Ok(MapperKind::Nrom(NromState::new(cart))),
            id => Err(CartridgeError::UnsupportedMapper(id)),
        }
    }
}

impl MapperImpl for MapperKind {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match self {
            MapperKind::Nrom(s) => s.cpu_read(addr),
        }
    }

    fn cpu_write(&mut self, addr: u16, val: u8) {
        match self {
            MapperKind::Nrom(s) => s.cpu_write(addr, val),
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        match self {
            MapperKind::Nrom(s) => s.ppu_read(addr),
        }
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        match self {
            MapperKind::Nrom(s) => s.ppu_write(addr, val),
        }
    }

    fn mirroring(&self) -> Mirroring {
        match self {
            MapperKind::Nrom(s) => s.mirroring(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, NesFormat};

    fn fake_nrom_cart() -> Cartridge {
        Cartridge {
            format: NesFormat::INes,
            mapper_id: 0,
            submapper_id: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            has_trainer: false,
            prg_rom: vec![0xAA; 16 * 1024],
            chr_rom: vec![0xBB; 8 * 1024],
            prg_ram_size: 8 * 1024,
            chr_ram_size: 0,
        }
    }

    #[test]
    fn dispatches_to_nrom() {
        let cart = fake_nrom_cart();
        let mut mapper = MapperKind::from_cartridge(&cart).unwrap();
        assert_eq!(mapper.cpu_read(0x8000), 0xAA);
        assert_eq!(mapper.ppu_read(0x0000), 0xBB);
        assert_eq!(mapper.mirroring(), Mirroring::Horizontal);
        assert!(!mapper.irq_pending());
    }

    #[test]
    fn rejects_unsupported_mapper() {
        let mut cart = fake_nrom_cart();
        cart.mapper_id = 4; // MMC3 — not implemented at M1
        let err = MapperKind::from_cartridge(&cart).unwrap_err();
        assert_eq!(err, CartridgeError::UnsupportedMapper(4));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p nies-core --lib mapper::
```

Expected: all 8 tests pass (6 from nrom + 2 from mod).

- [ ] **Step 3: Commit**

```bash
git add crates/nies-core/src/mapper
git commit -m "$(cat <<'EOF'
feat(core): MapperImpl trait + MapperKind enum dispatch

NROM is the only variant at M1; the enum architecture is set up so
adding MMC1/UxROM/CNROM/MMC3 in later milestones requires only a
new variant + match-arm updates, with no `Box<dyn>` runtime cost
and no `typetag` complexity.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: PPU/APU/DMC stubs

The PPU and APU don't exist yet, but `Bus::tick` must be able to call into them on every CPU cycle. Stubs accept `step()` and do nothing (until M2/M5).

**Files:**
- Modify: `crates/nies-core/src/ppu/mod.rs`, `crates/nies-core/src/apu/mod.rs`, `crates/nies-core/src/apu/dmc.rs`

- [ ] **Step 1: PPU stub**

Replace `crates/nies-core/src/ppu/mod.rs` with:

```rust
//! PPU stub. Real implementation lands in M2 (spec §3.5).
//!
//! At M1 this is just a counter that records the number of dots advanced,
//! so the bus can call `Ppu::step(&mut self, mapper)` from inside its
//! tick loop without panicking.

use crate::mapper::MapperKind;

#[derive(Debug, Clone, Default)]
pub struct Ppu {
    /// PPU dots elapsed since power-on. M2 will replace this with full
    /// state: scanline, dot, register file, OAM, etc.
    pub dots: u64,
}

impl Ppu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the PPU by one dot. M1 stub: just increments the counter.
    /// `_mapper` is unused at M1 but reserved for M2's A12 hook.
    pub fn step(&mut self, _mapper: &mut MapperKind) {
        self.dots = self.dots.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};
    use crate::mapper::MapperKind;

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
    fn step_increments_dot_counter() {
        let mut ppu = Ppu::new();
        let mut mapper = fake_mapper();
        for _ in 0..100 {
            ppu.step(&mut mapper);
        }
        assert_eq!(ppu.dots, 100);
    }
}
```

- [ ] **Step 2: APU + DMC stubs**

Replace `crates/nies-core/src/apu/dmc.rs` with:

```rust
//! DMC channel state. Real implementation lands in M5.
//!
//! At M1 this exists only so the bus tick has a `pending_fetch` slot
//! to check (always None) and a no-op `take_pending_fetch` method.

#[derive(Debug, Clone, Default)]
pub struct DmcChannel {
    pending_fetch: Option<u16>,
    /// Number of stall cycles to consume on the next fetch service.
    /// M5 will populate this; M1 always returns 0.
    stall_cycles: u32,
}

impl DmcChannel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the address of a pending CPU-bus sample fetch, if any, and
    /// clears it. The bus services the fetch from inside `Bus::tick`.
    pub fn take_pending_fetch(&mut self) -> Option<u16> {
        self.pending_fetch.take()
    }

    pub fn deliver_sample(&mut self, _val: u8) {
        // M1 stub: no sample buffer yet.
    }

    pub fn stall_cycles(&self) -> u32 {
        self.stall_cycles
    }
}
```

Replace `crates/nies-core/src/apu/mod.rs` with:

```rust
//! APU stub. Real implementation lands in M5 (spec §3.6).
//!
//! At M1 this is a counter that records CPU-cycle steps. The DMC
//! sub-module exposes a no-op `pending_fetch` API that `Bus::tick`
//! calls into; the bus is the same shape it will be at M5.

pub mod dmc;

use crate::mapper::MapperKind;
use dmc::DmcChannel;

#[derive(Debug, Clone, Default)]
pub struct Apu {
    /// CPU cycles elapsed since power-on. M5 will replace this with full
    /// state: pulse/triangle/noise/DMC channels, frame counter, mixer.
    pub cycles: u64,
    pub dmc: DmcChannel,
}

impl Apu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self, _mapper: &mut MapperKind) {
        self.cycles = self.cycles.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};

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
    fn step_increments_cycle_counter() {
        let mut apu = Apu::new();
        let mut mapper = fake_mapper();
        for _ in 0..50 {
            apu.step(&mut mapper);
        }
        assert_eq!(apu.cycles, 50);
        assert!(apu.dmc.take_pending_fetch().is_none());
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p nies-core --lib ppu:: apu::
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/nies-core/src/ppu crates/nies-core/src/apu
git commit -m "$(cat <<'EOF'
feat(core): PPU/APU/DMC stubs for M1

Both stubs accept the bus's tick calls and increment a counter so the
bus tick discipline can be exercised end-to-end without the real PPU
or APU. The DMC channel exposes a no-op pending-fetch API matching
the M5 shape so M1's bus implementation doesn't have to change.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Controller stub for $4016/$4017

The CPU bus dispatches reads from $4016/$4017 to controller state. At M1 the controllers are stubs that return 0; M4 fleshes them out.

**Files:**
- Modify: `crates/nies-core/src/input.rs`

- [ ] **Step 1: Implement the stub**

Replace `crates/nies-core/src/input.rs` with:

```rust
//! Controller state. Polled by CPU $4016/$4017 reads. See spec §5.5.
//!
//! At M1 this is a stub: $4016/$4017 always read 0 and writes are
//! ignored. M4 implements the strobe latch and 8-bit shift register.

#[derive(Debug, Clone, Default)]
pub struct Controller {
    /// Latched button state. Real bit assignments are A/B/Select/Start/
    /// Up/Down/Left/Right (LSB first); M4 fills this in.
    pub buttons: u8,
}

impl Controller {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the next bit out of the shift register. M1 stub: always 0.
    pub fn read(&mut self) -> u8 {
        0
    }

    /// Write to $4016 (strobe). M1 stub: no-op.
    pub fn write_strobe(&mut self, _val: u8) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_read_returns_zero() {
        let mut c = Controller::new();
        assert_eq!(c.read(), 0);
        c.write_strobe(1);
        assert_eq!(c.read(), 0);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p nies-core --lib input::
```

Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add crates/nies-core/src/input.rs
git commit -m "$(cat <<'EOF'
feat(core): Controller stub for $4016/$4017

M1 controllers always read 0 and ignore strobe writes. The shape
(read/write_strobe interface, buttons byte) is what M4 will fill in
when keyboard/gamepad mappings land.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Bus struct (no tick yet)

The bus owns RAM, mapper, controllers, and references to PPU/APU. At this task we wire up the field layout and the address-decoder logic for `read`/`write` *without* yet calling `tick()`. That's the next task.

**Files:**
- Modify: `crates/nies-core/src/bus.rs`

- [ ] **Step 1: Write the struct + read/write address decoder**

Replace `crates/nies-core/src/bus.rs` with:

```rust
//! CPU bus. Exposes `Bus::read` and `Bus::write`, both of which tick the
//! rest of the system (PPU/APU/mapper) one CPU cycle on every access.
//! See spec §3.3.

use crate::apu::Apu;
use crate::input::Controller;
use crate::mapper::{MapperImpl, MapperKind};
use crate::ppu::Ppu;

/// 2 KiB of CPU RAM, mirrored across $0000-$1FFF.
pub const CPU_RAM_BYTES: usize = 2048;

#[derive(Debug, Clone)]
pub struct Bus {
    pub ram: [u8; CPU_RAM_BYTES],
    pub ppu: Ppu,
    pub apu: Apu,
    pub mapper: MapperKind,
    pub controllers: [Controller; 2],
    /// Master CPU cycle counter since power-on.
    pub cycle: u64,
    /// Open-bus latch: last value transferred on the CPU data bus.
    /// Used as the read result for unmapped addresses (a real-hardware
    /// quirk that some test ROMs depend on).
    pub open_bus: u8,
}

impl Bus {
    pub fn new(mapper: MapperKind) -> Self {
        // Power-on RAM pattern: spec §4.2 says we use the Mesen pattern
        // ($00 except $0008/$0009/$000A/$000F set to $F7/$EF/$DF/$BF).
        let mut ram = [0u8; CPU_RAM_BYTES];
        ram[0x0008] = 0xF7;
        ram[0x0009] = 0xEF;
        ram[0x000A] = 0xDF;
        ram[0x000F] = 0xBF;
        Bus {
            ram,
            ppu: Ppu::new(),
            apu: Apu::new(),
            mapper,
            controllers: [Controller::new(), Controller::new()],
            cycle: 0,
            open_bus: 0,
        }
    }

    /// Read without ticking. For debugger inspection only — bypasses any
    /// side effects on register-mapped addresses.
    pub fn peek(&self, addr: u16) -> u8 {
        self.read_no_tick(addr)
    }

    /// Internal: address-decoder read, no tick. Used by both `read` (which
    /// adds the tick) and `peek` (which doesn't).
    pub(crate) fn read_no_tick(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => {
                // PPU register space: $2000-$2007 mirrored every 8 bytes.
                // Real PPU read effects (PPUSTATUS clear, PPUDATA buffer)
                // land in M2; M1 returns open-bus.
                self.open_bus
            }
            0x4000..=0x4015 => self.open_bus, // APU registers: M5
            0x4016 => 0,                      // Controller 1: M4 will fill in
            0x4017 => 0,                      // Controller 2 / APU frame counter: M4/M5
            0x4018..=0x401F => self.open_bus, // CPU test mode (unused on retail NES)
            0x4020..=0xFFFF => {
                // Cartridge-mapped: PRG-RAM, expansion ROM, PRG-ROM.
                // We need a non-mut mapper read; do an unchecked clone-elision
                // by going through the read_no_tick interface on the mapper.
                // (Mapper trait requires &mut self for cpu_read because some
                // mappers update internal state on read; but we can take a
                // clone-of-state copy here for peek. For M1 NROM doesn't have
                // read side effects, so we use a direct const access.)
                //
                // To avoid cloning the whole mapper, we accept that peek
                // might not be fully side-effect-free for stateful mappers
                // (post-M1). At M1 we just call cpu_read on a casted mut
                // reference (safe because we hold &self exclusively).
                #[allow(unsafe_code)]
                {
                    let mapper_mut: *const MapperKind = &self.mapper;
                    let mapper_mut = mapper_mut as *mut MapperKind;
                    unsafe { (*mapper_mut).cpu_read(addr) }
                }
            }
        }
    }

    /// Internal: write the data bus and update the address-decoder side
    /// without ticking. Used by `write` (adds tick) and any future debugger
    /// "force write" functionality.
    pub(crate) fn write_no_tick(&mut self, addr: u16, val: u8) {
        self.open_bus = val;
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize] = val,
            0x2000..=0x3FFF => {
                // PPU register write: M2.
                let _ = val;
            }
            0x4000..=0x4013 | 0x4015 => {
                // APU register write: M5.
                let _ = val;
            }
            0x4014 => {
                // OAMDMA: M2/M3 will implement the 256-byte transfer.
                let _ = val;
            }
            0x4016 => {
                self.controllers[0].write_strobe(val);
                self.controllers[1].write_strobe(val);
            }
            0x4017 => {
                // APU frame counter: M5.
                let _ = val;
            }
            0x4018..=0x401F => {} // CPU test mode (unused)
            0x4020..=0xFFFF => self.mapper.cpu_write(addr, val),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};

    fn fake_bus() -> Bus {
        let cart = Cartridge {
            format: NesFormat::INes,
            mapper_id: 0,
            submapper_id: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            has_trainer: false,
            prg_rom: (0..(32 * 1024) as u32).map(|i| i as u8).collect(),
            chr_rom: vec![0; 8 * 1024],
            prg_ram_size: 8 * 1024,
            chr_ram_size: 0,
        };
        Bus::new(MapperKind::from_cartridge(&cart).unwrap())
    }

    #[test]
    fn ram_mirroring() {
        let mut bus = fake_bus();
        bus.write_no_tick(0x0000, 0x42);
        assert_eq!(bus.read_no_tick(0x0800), 0x42); // $0000 mirrors at $0800
        assert_eq!(bus.read_no_tick(0x1000), 0x42); // ...and at $1000
        assert_eq!(bus.read_no_tick(0x1800), 0x42); // ...and at $1800
    }

    #[test]
    fn power_on_ram_uses_mesen_pattern() {
        let bus = fake_bus();
        assert_eq!(bus.read_no_tick(0x0000), 0x00);
        assert_eq!(bus.read_no_tick(0x0008), 0xF7);
        assert_eq!(bus.read_no_tick(0x0009), 0xEF);
        assert_eq!(bus.read_no_tick(0x000A), 0xDF);
        assert_eq!(bus.read_no_tick(0x000F), 0xBF);
    }

    #[test]
    fn prg_rom_visible_through_bus() {
        let bus = fake_bus();
        // PRG byte 0 is at $8000; a 32 KiB ROM fills $8000-$FFFF.
        assert_eq!(bus.read_no_tick(0x8000), 0);
        assert_eq!(bus.read_no_tick(0x8001), 1);
        assert_eq!(bus.read_no_tick(0xC000), 0x80);
    }

    #[test]
    fn unmapped_apu_read_returns_open_bus() {
        let mut bus = fake_bus();
        bus.write_no_tick(0x0000, 0xAB); // sets open_bus = 0xAB
        assert_eq!(bus.read_no_tick(0x4015), 0xAB);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p nies-core --lib bus::
```

Expected: 4 tests pass.

> **Implementer note:** the `unsafe` block in `read_no_tick` for cartridge-space reads is intentional and scoped: it's the only way to call `MapperImpl::cpu_read(&mut self)` from a `&self` context, and at M1 NROM has no read side effects so it's safe. M2 will revisit (likely by adding a `peek` method to `MapperImpl` that takes `&self`).

- [ ] **Step 3: Commit**

```bash
git add crates/nies-core/src/bus.rs
git commit -m "$(cat <<'EOF'
feat(core): Bus address decoder (read_no_tick / write_no_tick)

Implements the CPU address map per spec §3.3:
- $0000-$1FFF: 2 KiB CPU RAM mirrored every $0800.
- $2000-$3FFF: PPU register space (open-bus stub at M1).
- $4000-$4017: APU + I/O registers (mostly stubs at M1).
- $4020-$FFFF: cartridge-mapped (NROM at M1).

Power-on RAM uses the Mesen pattern from spec §4.2. Open-bus latch
records the last data-bus value so unmapped reads return what real
hardware does. Tick discipline is added in the next task.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Bus tick discipline + `read`/`write` public methods

Now we add `tick()` and the public ticking variants. Every `read`/`write` calls `tick()` once: 3 PPU dots, 1 APU step, 1 cycle counter increment, then the DMC fetch service.

**Files:**
- Modify: `crates/nies-core/src/bus.rs`

- [ ] **Step 1: Add tick + public read/write**

Append to `crates/nies-core/src/bus.rs` (inside the `impl Bus` block, before `read_no_tick`):

```rust
    /// Tick the rest of the system one CPU cycle. Called from every public
    /// `read`/`write`. See spec §3.3.
    fn tick(&mut self) {
        // 3 PPU dots per CPU cycle (NTSC).
        for _ in 0..3 {
            self.ppu.step(&mut self.mapper);
        }
        // 1 APU step per CPU cycle.
        self.apu.step(&mut self.mapper);
        self.cycle = self.cycle.wrapping_add(1);
        // Service any pending DMC fetch. The fetch performs a no-tick read
        // from CPU memory, delivers the sample to DMC, and adds the
        // configured stall by recursively ticking. M1 DMC is always idle
        // (pending_fetch is None), so the body is unreachable at M1; this
        // is here so M5's DMC code can land without bus changes.
        if let Some(addr) = self.apu.dmc.take_pending_fetch() {
            let val = self.read_no_tick(addr);
            self.apu.dmc.deliver_sample(val);
            self.stall(self.apu.dmc.stall_cycles());
        }
    }

    fn stall(&mut self, cycles: u32) {
        for _ in 0..cycles {
            for _ in 0..3 {
                self.ppu.step(&mut self.mapper);
            }
            self.apu.step(&mut self.mapper);
            self.cycle = self.cycle.wrapping_add(1);
        }
    }

    /// Read a byte from the CPU bus. Ticks the system one CPU cycle.
    pub fn read(&mut self, addr: u16) -> u8 {
        self.tick();
        let val = self.read_no_tick(addr);
        self.open_bus = val;
        val
    }

    /// Write a byte to the CPU bus. Ticks the system one CPU cycle.
    pub fn write(&mut self, addr: u16, val: u8) {
        self.tick();
        self.write_no_tick(addr, val);
    }
```

- [ ] **Step 2: Add tick-discipline tests**

Append to the `tests` mod in `crates/nies-core/src/bus.rs`:

```rust
    #[test]
    fn read_advances_cycle_counter() {
        let mut bus = fake_bus();
        let cycle_before = bus.cycle;
        let _ = bus.read(0x0000);
        assert_eq!(bus.cycle, cycle_before + 1);
    }

    #[test]
    fn write_advances_cycle_counter() {
        let mut bus = fake_bus();
        let cycle_before = bus.cycle;
        bus.write(0x0000, 0x42);
        assert_eq!(bus.cycle, cycle_before + 1);
    }

    #[test]
    fn read_advances_ppu_three_dots() {
        let mut bus = fake_bus();
        let dots_before = bus.ppu.dots;
        let _ = bus.read(0x0000);
        assert_eq!(bus.ppu.dots, dots_before + 3);
    }

    #[test]
    fn read_advances_apu_one_cycle() {
        let mut bus = fake_bus();
        let apu_cycles_before = bus.apu.cycles;
        let _ = bus.read(0x0000);
        assert_eq!(bus.apu.cycles, apu_cycles_before + 1);
    }

    #[test]
    fn peek_does_not_tick() {
        let bus = fake_bus();
        let cycle_before = bus.cycle;
        let _ = bus.peek(0x0000);
        // peek takes &self; cycle_before still valid from before
        assert_eq!(bus.cycle, cycle_before);
    }
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p nies-core --lib bus::
```

Expected: 9 tests pass (4 prior + 5 new).

- [ ] **Step 4: Commit**

```bash
git add crates/nies-core/src/bus.rs
git commit -m "$(cat <<'EOF'
feat(core): Bus tick discipline + public read/write

Bus::read and Bus::write are the only public memory access methods,
and both unconditionally tick: 3 PPU dots, 1 APU step, 1 cycle
counter increment, then DMC fetch service. The DMC fetch path is
wired up but inert at M1 because the DMC stub never sets a pending
fetch; this lets M5's DMC code land without bus changes.

Bus::peek (introduced in the previous commit) is the only no-tick
read path; intended for debugger inspection.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: CPU registers, status flags, reset state

Define the `Cpu` struct and the `StatusFlags` bit constants. Implement `reset()` per spec §3.4 (P=0x34, S=0xFD, PC=read_word(reset_vector)).

**Files:**
- Modify: `crates/nies-core/src/cpu/flags.rs`, `crates/nies-core/src/cpu/mod.rs`

- [ ] **Step 1: Status flags**

Replace `crates/nies-core/src/cpu/flags.rs` with:

```rust
//! 6502 status register bit positions.

pub const FLAG_C: u8 = 0b0000_0001; // Carry
pub const FLAG_Z: u8 = 0b0000_0010; // Zero
pub const FLAG_I: u8 = 0b0000_0100; // Interrupt-disable
pub const FLAG_D: u8 = 0b0000_1000; // Decimal (settable; BCD not implemented on NES)
pub const FLAG_B: u8 = 0b0001_0000; // Break (only set in pushed P from BRK/PHP)
pub const FLAG_U: u8 = 0b0010_0000; // Unused (always reads as 1 in pushed P)
pub const FLAG_V: u8 = 0b0100_0000; // Overflow
pub const FLAG_N: u8 = 0b1000_0000; // Negative
```

- [ ] **Step 2: CPU struct and reset**

Replace `crates/nies-core/src/cpu/mod.rs` with:

```rust
//! 6502 CPU implementation. See spec §3.4.

pub mod addressing;
pub mod flags;
pub mod instructions;

use crate::bus::Bus;

/// 6502 CPU state.
#[derive(Debug, Clone, Copy)]
pub struct Cpu {
    pub a: u8,    // accumulator
    pub x: u8,    // X index
    pub y: u8,    // Y index
    pub pc: u16,  // program counter
    pub sp: u8,   // stack pointer
    pub p: u8,    // status flags
    /// True when the CPU is halted by a JAM/KIL/HLT illegal opcode.
    pub jammed: bool,
    /// Pending NMI: latched when the NMI line is pulled low; serviced
    /// at the next instruction boundary. Set by the PPU; cleared after
    /// the NMI handler entry.
    pub nmi_pending: bool,
    /// Pending IRQ: level-sensitive (asserted while any IRQ source holds
    /// the line low). Sampled at instruction boundaries when I flag is clear.
    pub irq_pending: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0xFD,
            p: 0x34, // I=1, U=1, B=1 (the "B flag" bit in P is always set when read directly)
            jammed: false,
            nmi_pending: false,
            irq_pending: false,
        }
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize CPU state per spec: A=X=Y=0, S=$FD, P=$34,
    /// PC=read_word(reset_vector $FFFC). Each of the two reset reads ticks
    /// the bus.
    pub fn reset(&mut self, bus: &mut Bus) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.p = 0x34;
        self.jammed = false;
        self.nmi_pending = false;
        self.irq_pending = false;
        let lo = bus.read(0xFFFC) as u16;
        let hi = bus.read(0xFFFD) as u16;
        self.pc = (hi << 8) | lo;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::cartridge::{Cartridge, Mirroring, NesFormat};
    use crate::mapper::MapperKind;

    fn bus_with_reset_vector(vector: u16) -> Bus {
        let mut prg = vec![0u8; 32 * 1024];
        // Reset vector lives at $FFFC-$FFFD; with 32 KiB PRG mapped to
        // $8000-$FFFF that's prg[0x7FFC..0x7FFE].
        prg[0x7FFC] = (vector & 0xFF) as u8;
        prg[0x7FFD] = (vector >> 8) as u8;
        let cart = Cartridge {
            format: NesFormat::INes,
            mapper_id: 0,
            submapper_id: 0,
            mirroring: Mirroring::Horizontal,
            has_battery: false,
            has_trainer: false,
            prg_rom: prg,
            chr_rom: vec![0; 8 * 1024],
            prg_ram_size: 0,
            chr_ram_size: 0,
        };
        Bus::new(MapperKind::from_cartridge(&cart).unwrap())
    }

    #[test]
    fn reset_loads_pc_from_vector() {
        let mut bus = bus_with_reset_vector(0xC000);
        let mut cpu = Cpu::new();
        cpu.reset(&mut bus);
        assert_eq!(cpu.pc, 0xC000);
        assert_eq!(cpu.sp, 0xFD);
        assert_eq!(cpu.p, 0x34);
        assert_eq!(cpu.a, 0);
        assert_eq!(cpu.x, 0);
        assert_eq!(cpu.y, 0);
    }

    #[test]
    fn reset_consumes_two_bus_cycles() {
        let mut bus = bus_with_reset_vector(0x8000);
        let mut cpu = Cpu::new();
        let cycle_before = bus.cycle;
        cpu.reset(&mut bus);
        assert_eq!(bus.cycle, cycle_before + 2);
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p nies-core --lib cpu::
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/nies-core/src/cpu
git commit -m "$(cat <<'EOF'
feat(core): CPU registers, status flags, reset

Cpu struct holds A/X/Y/PC/SP/P plus jammed/nmi_pending/irq_pending
flags. Cpu::reset reads $FFFC/$FFFD through the bus to set PC,
zeros A/X/Y, sets SP=$FD and P=$34 per the 6502 reset spec.

Status flag bit constants live in cpu::flags. The D flag is settable
but BCD arithmetic is intentionally not implemented (NES 6502
omits it).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: `Cpu::step` skeleton + opcode dispatch table

Wire up the structure that will hold the 256-entry dispatch table. At this task the table is empty — every entry maps to a "not yet implemented" handler that panics with the opcode byte. We'll fill it in incrementally as we implement each opcode family.

**Files:**
- Modify: `crates/nies-core/src/cpu/instructions.rs`, `crates/nies-core/src/cpu/mod.rs`

- [ ] **Step 1: Write the dispatch table scaffolding**

Replace `crates/nies-core/src/cpu/instructions.rs` with:

```rust
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
```

> **Implementer note:** the const `build_table()` approach won't compile if individual entries need non-const initialization. We'll switch to a `LazyLock` or a `OnceLock` once we start writing per-opcode handlers if `const` becomes the bottleneck. For the M1 dispatch we can build the table at module load time inside a `LazyLock` if needed. Don't over-engineer at this step; revisit when the first real opcode is added in Task 13.

- [ ] **Step 2: Add `Cpu::step` to `cpu/mod.rs`**

Append to `crates/nies-core/src/cpu/mod.rs` (inside `impl Cpu`):

```rust
    /// Execute one CPU instruction. Handles pending NMI/IRQ at the
    /// instruction boundary before fetching the next opcode.
    pub fn step(&mut self, bus: &mut Bus) {
        if self.jammed {
            // KIL/JAM/HLT halts the CPU until reset; just keep ticking
            // the bus so PPU/APU continue running.
            let _ = bus.read(self.pc);
            return;
        }

        // Interrupt servicing happens at instruction boundaries.
        if self.nmi_pending {
            self.service_nmi(bus);
            return;
        }
        if self.irq_pending && (self.p & flags::FLAG_I) == 0 {
            self.service_irq(bus);
            return;
        }

        let opcode = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        let handler = instructions::OPCODES[opcode as usize];
        handler(self, bus);
    }

    fn service_nmi(&mut self, _bus: &mut Bus) {
        // Filled in by Task 38.
        unimplemented!("NMI service in Task 38");
    }

    fn service_irq(&mut self, _bus: &mut Bus) {
        // Filled in by Task 39.
        unimplemented!("IRQ service in Task 39");
    }
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p nies-core --lib cpu::
```

Expected: existing 2 cpu tests still pass + 1 new dispatch table test = 3 total.

- [ ] **Step 4: Commit**

```bash
git add crates/nies-core/src/cpu
git commit -m "$(cat <<'EOF'
feat(core): CPU step skeleton and 256-entry dispatch table

Cpu::step handles JAM, NMI, and IRQ checks at the instruction boundary
(NMI/IRQ servicing is stubbed; filled in by Tasks 38-39). The opcode
dispatch table is built statically with all entries pointing at a
panic stub; per-opcode handlers are wired in by subsequent tasks.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase B — Test Harness (Tasks 11–12)

Get SingleStepTests/65x02 vendored and a per-opcode test runner working before we implement opcodes.

### Task 11: Vendor SingleStepTests/65x02 corpus

The corpus is at https://github.com/SingleStepTests/65x02 (~50 MB raw JSON). We compress it with zstd to ~6-10 MB, vendor, and decompress on first test run.

**Files:**
- Create: `crates/nies-core/tests/data/65x02.tar.zst`
- Modify: `crates/nies-core/Cargo.toml` (add `zstd`, `serde_json` dev-dependencies)

- [ ] **Step 1: Download and re-package the corpus**

```bash
cd /tmp
git clone --depth 1 https://github.com/SingleStepTests/65x02.git
cd 65x02

# The corpus may live under nes6502/v1/*.json or 6502/v1/*.json depending on
# the repo layout. Check both.
ls
```

If the JSON files are under `nes6502/v1/`, package those:

```bash
tar --create --file=- nes6502/v1/*.json | zstd -19 --long -o /Users/eperdew/Software/nies/crates/nies-core/tests/data/65x02.tar.zst
```

Otherwise package whatever directory contains the per-opcode JSON files. Verify the resulting tarball is between 5-12 MB:

```bash
ls -lh /Users/eperdew/Software/nies/crates/nies-core/tests/data/65x02.tar.zst
```

- [ ] **Step 2: Add the dev-dependencies**

Edit `crates/nies-core/Cargo.toml`:

```toml
[dev-dependencies]
zstd = "0.13"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tar = "0.4"
```

> **Note:** the original plan also listed `once_cell = "1"`, but the actual harness in Task 12 uses `std::sync::OnceLock` (stdlib) and never imports `once_cell`. The dep was dropped in a follow-up commit.

- [ ] **Step 3: Verify the tarball decompresses + parses**

Write a one-off scratch verifier as a unit test in `crates/nies-core/tests/data_smoke.rs`:

```rust
//! Smoke test: confirm the vendored 65x02 corpus tarball decompresses
//! and contains valid JSON for at least one opcode (LDA #imm, $A9).

use std::io::Read;

#[test]
fn corpus_tarball_contains_a9_lda_imm() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/65x02.tar.zst");
    let f = std::fs::File::open(path).expect("open tarball");
    let dec = zstd::Decoder::new(f).expect("zstd decoder");
    let mut archive = tar::Archive::new(dec);

    for entry in archive.entries().expect("read entries") {
        let mut entry = entry.expect("entry");
        let path = entry.path().expect("path").to_path_buf();
        if path.file_name().is_some_and(|n| n == "a9.json") {
            let mut buf = String::new();
            entry.read_to_string(&mut buf).expect("read a9.json");
            // Parse as untyped JSON; just confirm it's an array with > 0 cases.
            let parsed: serde_json::Value =
                serde_json::from_str(&buf).expect("parse a9.json");
            let cases = parsed.as_array().expect("array");
            assert!(cases.len() > 100, "expected lots of test cases, got {}", cases.len());
            return;
        }
    }
    panic!("a9.json not found in corpus tarball");
}
```

- [ ] **Step 4: Run the smoke test**

```bash
cargo test -p nies-core --test data_smoke
```

Expected: passes; confirms the tarball is loadable and contains the expected per-opcode JSON files.

- [ ] **Step 5: Commit**

```bash
git add crates/nies-core/tests/data/65x02.tar.zst crates/nies-core/tests/data_smoke.rs crates/nies-core/Cargo.toml Cargo.lock
git commit -m "$(cat <<'EOF'
test(core): vendor SingleStepTests/65x02 corpus

Compressed (zstd -19) tarball of the per-opcode JSON test cases from
https://github.com/SingleStepTests/65x02. ~6-10 MB at rest;
decompressed and consumed by the singlestep_tests integration test
in the next task. A smoke test confirms the tarball is loadable and
contains expected entries.

Per spec §7.1 + design discussion: vendored rather than downloaded
at test time, to keep CI offline-clean.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 12: SingleStepTests test harness

Build the integration test runner that decompresses the corpus on first run, parses each opcode's JSON file, and provides a `run_opcode_tests(opcode_byte)` function that opcode-implementation tasks can call.

**Files:**
- Create: `crates/nies-core/tests/singlestep_tests.rs`
- Create: `crates/nies-core/tests/common/mod.rs` (shared test utilities)

- [ ] **Step 1: Common utilities + test case types**

Write `crates/nies-core/tests/common/mod.rs`:

```rust
//! Shared utilities for the SingleStepTests/65x02 integration tests.

use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub initial: TestState,
    pub r#final: TestState,
    pub cycles: Vec<(u16, u8, String)>,
}

#[derive(Debug, Deserialize)]
pub struct TestState {
    pub pc: u16,
    pub s: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub ram: Vec<(u16, u8)>,
}

/// On first call, decompress the corpus tarball into
/// `target/test-cache/65x02/` and return that path. Subsequent calls
/// return the same path without re-extraction (idempotent via
/// directory-existence check).
pub fn corpus_root() -> &'static Path {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| {
        let target_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target")
            .join("test-cache")
            .join("65x02");
        if !target_dir.exists() {
            extract_corpus(&target_dir);
        }
        target_dir
    })
}

fn extract_corpus(dest: &Path) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/65x02.tar.zst");
    let f = std::fs::File::open(path).expect("open vendored corpus tarball");
    let dec = zstd::Decoder::new(f).expect("zstd decoder");
    let mut archive = tar::Archive::new(dec);
    std::fs::create_dir_all(dest).expect("create dest dir");
    archive.unpack(dest).expect("extract tarball");
}

/// Load all test cases for a specific opcode (e.g., 0xA9 for LDA #imm).
/// Returns the parsed test cases. Panics if the JSON file is missing.
pub fn load_opcode_cases(opcode: u8) -> Vec<TestCase> {
    let root = corpus_root();
    let filename = format!("{:02x}.json", opcode);
    // Search for the file under any subdirectory of root.
    let path = find_json(root, &filename)
        .unwrap_or_else(|| panic!("opcode {opcode:02X}: {filename} not found in corpus"));
    let mut buf = String::new();
    std::fs::File::open(&path)
        .unwrap_or_else(|e| panic!("open {path:?}: {e}"))
        .read_to_string(&mut buf)
        .expect("read corpus json");
    serde_json::from_str(&buf).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

fn find_json(root: &Path, filename: &str) -> Option<PathBuf> {
    fn recurse(dir: &Path, target: &str) -> Option<PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.file_name().is_some_and(|n| n == target) {
                return Some(p);
            }
            if p.is_dir() {
                if let Some(found) = recurse(&p, target) {
                    return Some(found);
                }
            }
        }
        None
    }
    recurse(root, filename)
}
```

- [ ] **Step 2: Tracing bus + per-opcode runner**

Write `crates/nies-core/tests/singlestep_tests.rs`:

```rust
//! Per-opcode integration tests driven by the SingleStepTests/65x02
//! corpus. Each opcode-implementation task adds a `#[test]` here that
//! calls `run_opcode_tests(0xNN)`.
//!
//! The corpus assumes a 64 KiB flat memory model (no PPU/APU sub-mapping),
//! so this test runner builds a `FlatBus` (declared below) that mirrors
//! `Bus`'s public read/write interface but stores all addresses in a
//! single 64 KiB `Vec<u8>` plus a recorded access list.

mod common;

use common::{load_opcode_cases, TestCase};
use nies_core::cpu::flags;

/// 64 KiB flat memory + cycle-by-cycle access trace.
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

/// Mini-Cpu compatible enough with `nies_core::cpu::Cpu` to drive the
/// dispatch table. Built so we can substitute the FlatBus without
/// changing the Cpu API.
///
/// We deliberately reuse the production opcode handlers; the SingleStepTests
/// corpus's purpose is to validate that production code is correct against
/// every documented case.
pub struct TestHarness {
    pub cpu: nies_core::cpu::Cpu,
    pub bus: FlatBus,
}

impl TestHarness {
    fn from_initial(state: &common::TestState) -> Self {
        let mut bus = FlatBus::new();
        for &(addr, val) in &state.ram {
            bus.mem[addr as usize] = val;
        }
        let mut cpu = nies_core::cpu::Cpu::new();
        cpu.pc = state.pc;
        cpu.sp = state.s;
        cpu.a = state.a;
        cpu.x = state.x;
        cpu.y = state.y;
        cpu.p = state.p;
        TestHarness { cpu, bus }
    }
}

/// Run all test cases for a specific opcode and assert state + cycle trace
/// match. Reports the first failing case if any.
pub fn run_opcode_tests(opcode: u8) {
    let cases = load_opcode_cases(opcode);
    let mut failures = 0usize;
    let mut first_failure: Option<String> = None;

    for case in &cases {
        match run_single_case(opcode, case) {
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
            "opcode {opcode:02X}: {failures}/{} cases failed. First failure: {}",
            cases.len(),
            first_failure.as_deref().unwrap_or("?")
        );
    }
}

fn run_single_case(_opcode: u8, case: &TestCase) -> Result<(), String> {
    let mut harness = TestHarness::from_initial(&case.initial);

    // Step one instruction. Production CPU expects a real `Bus`; our
    // FlatBus needs a thin adapter. The simplest approach for this
    // milestone: re-implement step against FlatBus by calling the
    // dispatch table directly with a bus-shaped trait. That requires
    // a `Bus`-like trait the production Cpu can consume. For M1 we
    // accept a small duplication: the test harness mirrors the bus's
    // public surface but operates on FlatBus.

    // ... (full step-against-FlatBus logic implemented in Task 13
    // as part of the LDA #imm bring-up. At Task 12 the runner exists
    // but cannot yet drive the CPU through opcodes — Task 13 introduces
    // a `BusLike` trait that both production Bus and FlatBus implement,
    // allowing the dispatch table to be polymorphic.)
    //
    // For Task 12 the runner is dead-coded.

    let _ = (case, &harness, flags::FLAG_C);
    Err("step-against-FlatBus not yet implemented; see Task 13".to_string())
}
```

> **Implementer note:** Task 12 stops at "test runner exists but the step-against-FlatBus glue lands in Task 13." This is intentional — completing the polymorphic-bus design *and* the LDA #imm opcode in the same task gives us a clean "first opcode green" milestone.

- [ ] **Step 3: Verify it compiles**

```bash
cargo test -p nies-core --test singlestep_tests --no-run
```

Expected: builds successfully (no test executions yet because Task 12 only sets up the harness).

- [ ] **Step 4: Commit**

```bash
git add crates/nies-core/tests/common crates/nies-core/tests/singlestep_tests.rs
git commit -m "$(cat <<'EOF'
test(core): SingleStepTests/65x02 harness scaffolding

Decompress-on-first-run corpus loader (target/test-cache/65x02/),
JSON parsing, FlatBus 64 KiB-flat-memory test bus, and the
run_opcode_tests entry point. Step-against-FlatBus glue lands in
Task 13 alongside the first opcode (LDA #imm).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase C — Bus Polymorphism + First Opcode (Task 13)

### Task 13: `BusLike` trait + LDA #imm (opcode $A9)

To run SingleStepTests, the production CPU needs to be parameterized over the bus type. Introduce a `BusLike` trait that both production `Bus` and the test harness's `FlatBus` implement.

**Files:**
- Modify: `crates/nies-core/src/bus.rs`, `crates/nies-core/src/cpu/mod.rs`, `crates/nies-core/src/cpu/instructions.rs`, `crates/nies-core/src/cpu/addressing.rs`, `crates/nies-core/tests/singlestep_tests.rs`

- [ ] **Step 1: Define `BusLike` trait**

Append to `crates/nies-core/src/bus.rs`:

```rust
/// Bus interface required by the CPU's dispatch table. Production code
/// uses `Bus`; tests substitute a flat-memory implementation. The trait
/// is the *only* thing the CPU sees; concrete bus types add extra state
/// (PPU/APU/mapper) on the side.
pub trait BusLike {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    /// True when the mapper has asserted IRQ. Production: forwards to
    /// `mapper.irq_pending()`. Tests: always false.
    fn mapper_irq_pending(&self) -> bool { false }
}

impl BusLike for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        Bus::read(self, addr)
    }
    fn write(&mut self, addr: u16, val: u8) {
        Bus::write(self, addr, val)
    }
    fn mapper_irq_pending(&self) -> bool {
        use crate::mapper::MapperImpl;
        self.mapper.irq_pending()
    }
}
```

- [ ] **Step 2: Generalize `Cpu::step` and the dispatch table over `BusLike`**

Modify `crates/nies-core/src/cpu/mod.rs`:

Change `pub fn step(&mut self, bus: &mut Bus)` to:

```rust
    pub fn step<B: crate::bus::BusLike>(&mut self, bus: &mut B) {
        if self.jammed {
            let _ = bus.read(self.pc);
            return;
        }
        if self.nmi_pending {
            self.service_nmi(bus);
            return;
        }
        if self.irq_pending && (self.p & flags::FLAG_I) == 0 {
            self.service_irq(bus);
            return;
        }
        let opcode = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        instructions::dispatch(opcode, self, bus);
    }

    fn service_nmi<B: crate::bus::BusLike>(&mut self, _bus: &mut B) {
        unimplemented!("NMI service in Task 38");
    }

    fn service_irq<B: crate::bus::BusLike>(&mut self, _bus: &mut B) {
        unimplemented!("IRQ service in Task 39");
    }
```

Change `Cpu::reset(&mut self, bus: &mut Bus)` to take `<B: BusLike>` similarly.

- [ ] **Step 3: Replace the static dispatch table with a `match` switch**

Replace `crates/nies-core/src/cpu/instructions.rs` with:

```rust
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
```

- [ ] **Step 4: Implement `BusLike` for `FlatBus` and finish the test runner**

Replace the `singlestep_tests.rs` file's `run_single_case` fn (and add `BusLike` impl):

```rust
//! Per-opcode integration tests driven by the SingleStepTests/65x02 corpus.

mod common;

use common::{load_opcode_cases, TestCase};
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
        return Err(format!("PC: expected {:04X}, got {:04X}", case.r#final.pc, cpu.pc));
    }
    if cpu.sp != case.r#final.s {
        return Err(format!("S: expected {:02X}, got {:02X}", case.r#final.s, cpu.sp));
    }
    if cpu.a != case.r#final.a {
        return Err(format!("A: expected {:02X}, got {:02X}", case.r#final.a, cpu.a));
    }
    if cpu.x != case.r#final.x {
        return Err(format!("X: expected {:02X}, got {:02X}", case.r#final.x, cpu.x));
    }
    if cpu.y != case.r#final.y {
        return Err(format!("Y: expected {:02X}, got {:02X}", case.r#final.y, cpu.y));
    }
    if cpu.p != case.r#final.p {
        return Err(format!("P: expected {:02X}, got {:02X}", case.r#final.p, cpu.p));
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
                expected.0, expected.1, expected.2,
                actual.0, actual.1, actual.2
            ));
        }
    }
    Ok(())
}

#[test]
fn opcode_a9_lda_imm() {
    run_opcode_tests(0xA9);
}
```

- [ ] **Step 5: Run the test**

```bash
cargo test -p nies-core --test singlestep_tests opcode_a9_lda_imm
```

Expected: passes, all ~10K test cases for $A9 succeed.

- [ ] **Step 6: Commit**

```bash
git add crates/nies-core/src crates/nies-core/tests/singlestep_tests.rs
git commit -m "$(cat <<'EOF'
feat(core): BusLike trait + first opcode (LDA #imm, $A9)

The CPU is now generic over BusLike, with concrete impls for the
production Bus and the FlatBus used by SingleStepTests. The dispatch
table is a `match opcode { ... }` (generics can't live in a static
function table) that monomorphizes per bus type at compile time.

LDA #immediate ($A9) is the first opcode implemented; all ~10K
SingleStepTests cases for $A9 pass.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase D — Addressing Modes (Task 14)

### Task 14: All addressing modes via shared helpers

The 6502 has 13 addressing modes; we factor each into a helper that returns the effective address (and whether a page-cross occurred, where relevant for timing). Subsequent opcode tasks compose these helpers with the operation logic.

**Files:**
- Modify: `crates/nies-core/src/cpu/addressing.rs`

This task only adds helper functions; it does not yet wire them up to opcodes (that's Tasks 15+). We test the helpers indirectly by implementing one opcode per addressing mode in Task 15 and watching the SingleStepTests pass.

- [ ] **Step 1: Implement all 13 addressing-mode helpers**

Replace `crates/nies-core/src/cpu/addressing.rs` with:

```rust
//! 6502 addressing-mode resolution helpers.
//!
//! Each helper reads operand bytes from PC (advancing PC past them),
//! computes the effective address, and returns it. For modes where the
//! cycle count varies on page-cross, the helper also returns whether
//! a page boundary was crossed.

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
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build -p nies-core
```

Expected: builds without warnings (the helpers are unused at this task; clippy will warn — gate the warnings with `#[allow(dead_code)]` if needed, or accept that Task 15 will exercise them).

If clippy complains about unused helpers, add `#![allow(dead_code)]` to the top of `addressing.rs` *temporarily* (Task 15 removes it).

- [ ] **Step 3: Commit**

```bash
git add crates/nies-core/src/cpu/addressing.rs
git commit -m "$(cat <<'EOF'
feat(core): all 6502 addressing-mode resolution helpers

Implements zp, zp_x, zp_y, abs, abs_x_read, abs_x_rmw, abs_y_read,
abs_y_rmw, ind (with page-wrap bug for JMP indirect), ind_x, ind_y_read,
ind_y_rmw, and relative. Each emits the canonical dummy reads for
its cycle profile per the SingleStepTests/65x02 expected access lists.

Helpers are exercised by Task 15+ when opcodes start binding them.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase E — Official Opcodes (Tasks 15–28)

Each task in this phase implements an opcode family across all its addressing-mode variants. The pattern: write the SingleStepTests `#[test]` for each opcode byte first (it'll fail with the unimplemented panic), implement the opcodes, run the tests, commit.

### Task 15: Load/Store family (LDA, LDX, LDY, STA, STX, STY)

**Opcodes covered:**
- LDA: $A9 imm, $A5 zp, $B5 zp,X, $AD abs, $BD abs,X, $B9 abs,Y, $A1 (ind,X), $B1 (ind),Y
- LDX: $A2 imm, $A6 zp, $B6 zp,Y, $AE abs, $BE abs,Y
- LDY: $A0 imm, $A4 zp, $B4 zp,X, $AC abs, $BC abs,X
- STA: $85 zp, $95 zp,X, $8D abs, $9D abs,X, $99 abs,Y, $81 (ind,X), $91 (ind),Y
- STX: $86 zp, $96 zp,Y, $8E abs
- STY: $84 zp, $94 zp,X, $8C abs

**Files:**
- Modify: `crates/nies-core/src/cpu/instructions.rs`
- Modify: `crates/nies-core/tests/singlestep_tests.rs`

- [ ] **Step 1: Add `#[test]` entries for every opcode in this family**

Append to `crates/nies-core/tests/singlestep_tests.rs`:

```rust
// Load/Store family
#[test] fn opcode_a5_lda_zp() { run_opcode_tests(0xA5); }
#[test] fn opcode_b5_lda_zpx() { run_opcode_tests(0xB5); }
#[test] fn opcode_ad_lda_abs() { run_opcode_tests(0xAD); }
#[test] fn opcode_bd_lda_absx() { run_opcode_tests(0xBD); }
#[test] fn opcode_b9_lda_absy() { run_opcode_tests(0xB9); }
#[test] fn opcode_a1_lda_indx() { run_opcode_tests(0xA1); }
#[test] fn opcode_b1_lda_indy() { run_opcode_tests(0xB1); }

#[test] fn opcode_a2_ldx_imm() { run_opcode_tests(0xA2); }
#[test] fn opcode_a6_ldx_zp() { run_opcode_tests(0xA6); }
#[test] fn opcode_b6_ldx_zpy() { run_opcode_tests(0xB6); }
#[test] fn opcode_ae_ldx_abs() { run_opcode_tests(0xAE); }
#[test] fn opcode_be_ldx_absy() { run_opcode_tests(0xBE); }

#[test] fn opcode_a0_ldy_imm() { run_opcode_tests(0xA0); }
#[test] fn opcode_a4_ldy_zp() { run_opcode_tests(0xA4); }
#[test] fn opcode_b4_ldy_zpx() { run_opcode_tests(0xB4); }
#[test] fn opcode_ac_ldy_abs() { run_opcode_tests(0xAC); }
#[test] fn opcode_bc_ldy_absx() { run_opcode_tests(0xBC); }

#[test] fn opcode_85_sta_zp() { run_opcode_tests(0x85); }
#[test] fn opcode_95_sta_zpx() { run_opcode_tests(0x95); }
#[test] fn opcode_8d_sta_abs() { run_opcode_tests(0x8D); }
#[test] fn opcode_9d_sta_absx() { run_opcode_tests(0x9D); }
#[test] fn opcode_99_sta_absy() { run_opcode_tests(0x99); }
#[test] fn opcode_81_sta_indx() { run_opcode_tests(0x81); }
#[test] fn opcode_91_sta_indy() { run_opcode_tests(0x91); }

#[test] fn opcode_86_stx_zp() { run_opcode_tests(0x86); }
#[test] fn opcode_96_stx_zpy() { run_opcode_tests(0x96); }
#[test] fn opcode_8e_stx_abs() { run_opcode_tests(0x8E); }

#[test] fn opcode_84_sty_zp() { run_opcode_tests(0x84); }
#[test] fn opcode_94_sty_zpx() { run_opcode_tests(0x94); }
#[test] fn opcode_8c_sty_abs() { run_opcode_tests(0x8C); }
```

- [ ] **Step 2: Verify the tests fail with "unimplemented opcode"**

```bash
cargo test -p nies-core --test singlestep_tests opcode_a5
```

Expected: panic with "unimplemented opcode $A5".

- [ ] **Step 3: Implement the opcodes**

Replace the body of `instructions.rs` with the full Load/Store family. Add to the dispatch `match`:

```rust
use crate::cpu::addressing as addr;

pub fn dispatch<B: BusLike>(opcode: u8, cpu: &mut Cpu, bus: &mut B) {
    match opcode {
        // LDA
        0xA9 => { let v = addr::fetch_byte(cpu, bus); ld_a(cpu, v); }
        0xA5 => { let a = addr::zp(cpu, bus); let v = bus.read(a); ld_a(cpu, v); }
        0xB5 => { let a = addr::zp_x(cpu, bus); let v = bus.read(a); ld_a(cpu, v); }
        0xAD => { let a = addr::abs(cpu, bus); let v = bus.read(a); ld_a(cpu, v); }
        0xBD => { let a = addr::abs_x_read(cpu, bus); let v = bus.read(a); ld_a(cpu, v); }
        0xB9 => { let a = addr::abs_y_read(cpu, bus); let v = bus.read(a); ld_a(cpu, v); }
        0xA1 => { let a = addr::ind_x(cpu, bus); let v = bus.read(a); ld_a(cpu, v); }
        0xB1 => { let a = addr::ind_y_read(cpu, bus); let v = bus.read(a); ld_a(cpu, v); }
        // LDX
        0xA2 => { let v = addr::fetch_byte(cpu, bus); ld_x(cpu, v); }
        0xA6 => { let a = addr::zp(cpu, bus); let v = bus.read(a); ld_x(cpu, v); }
        0xB6 => { let a = addr::zp_y(cpu, bus); let v = bus.read(a); ld_x(cpu, v); }
        0xAE => { let a = addr::abs(cpu, bus); let v = bus.read(a); ld_x(cpu, v); }
        0xBE => { let a = addr::abs_y_read(cpu, bus); let v = bus.read(a); ld_x(cpu, v); }
        // LDY
        0xA0 => { let v = addr::fetch_byte(cpu, bus); ld_y(cpu, v); }
        0xA4 => { let a = addr::zp(cpu, bus); let v = bus.read(a); ld_y(cpu, v); }
        0xB4 => { let a = addr::zp_x(cpu, bus); let v = bus.read(a); ld_y(cpu, v); }
        0xAC => { let a = addr::abs(cpu, bus); let v = bus.read(a); ld_y(cpu, v); }
        0xBC => { let a = addr::abs_x_read(cpu, bus); let v = bus.read(a); ld_y(cpu, v); }
        // STA
        0x85 => { let a = addr::zp(cpu, bus); bus.write(a, cpu.a); }
        0x95 => { let a = addr::zp_x(cpu, bus); bus.write(a, cpu.a); }
        0x8D => { let a = addr::abs(cpu, bus); bus.write(a, cpu.a); }
        0x9D => { let a = addr::abs_x_rmw(cpu, bus); bus.write(a, cpu.a); }
        0x99 => { let a = addr::abs_y_rmw(cpu, bus); bus.write(a, cpu.a); }
        0x81 => { let a = addr::ind_x(cpu, bus); bus.write(a, cpu.a); }
        0x91 => { let a = addr::ind_y_rmw(cpu, bus); bus.write(a, cpu.a); }
        // STX
        0x86 => { let a = addr::zp(cpu, bus); bus.write(a, cpu.x); }
        0x96 => { let a = addr::zp_y(cpu, bus); bus.write(a, cpu.x); }
        0x8E => { let a = addr::abs(cpu, bus); bus.write(a, cpu.x); }
        // STY
        0x84 => { let a = addr::zp(cpu, bus); bus.write(a, cpu.y); }
        0x94 => { let a = addr::zp_x(cpu, bus); bus.write(a, cpu.y); }
        0x8C => { let a = addr::abs(cpu, bus); bus.write(a, cpu.y); }
        _ => panic!(
            "CPU executed unimplemented opcode ${opcode:02X} at PC=${:04X}",
            cpu.pc.wrapping_sub(1)
        ),
    }
}

fn ld_a(cpu: &mut Cpu, v: u8) { cpu.a = v; set_nz(cpu, v); }
fn ld_x(cpu: &mut Cpu, v: u8) { cpu.x = v; set_nz(cpu, v); }
fn ld_y(cpu: &mut Cpu, v: u8) { cpu.y = v; set_nz(cpu, v); }

fn set_nz(cpu: &mut Cpu, val: u8) { /* ... unchanged from Task 13 ... */ }
```

- [ ] **Step 4: Run all 31 Load/Store tests**

```bash
cargo test -p nies-core --test singlestep_tests opcode_a5 opcode_b5 opcode_ad opcode_bd opcode_b9 opcode_a1 opcode_b1 \
  opcode_a2 opcode_a6 opcode_b6 opcode_ae opcode_be \
  opcode_a0 opcode_a4 opcode_b4 opcode_ac opcode_bc \
  opcode_85 opcode_95 opcode_8d opcode_9d opcode_99 opcode_81 opcode_91 \
  opcode_86 opcode_96 opcode_8e \
  opcode_84 opcode_94 opcode_8c
```

Expected: all pass. Total test cases run: ~310,000 (31 opcodes × ~10K cases each).

- [ ] **Step 5: Commit**

```bash
git add crates/nies-core/src/cpu/instructions.rs crates/nies-core/tests/singlestep_tests.rs
git commit -m "$(cat <<'EOF'
feat(cpu): Load/Store family (LDA, LDX, LDY, STA, STX, STY)

31 opcodes covering all addressing modes for the load/store family.
SingleStepTests/65x02 corpus passes for every variant. Page-cross
dummy reads land in the addressing-mode helpers from Task 14; this
task only wires them to the operation logic.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Tasks 16–28: Remaining official opcode families

Each of the following tasks follows the **same five-step pattern as Task 15**: add `#[test]` entries for the opcodes, verify they fail, implement, verify they pass, commit. To keep this plan navigable, each task lists only the opcode set, the operation logic, and any non-obvious cycle-trace caveats. **The full implementation is in `cpu/instructions.rs`; the test wiring is in `tests/singlestep_tests.rs`.**

For each task, the commit message is:

```
feat(cpu): <family name>

<list of opcodes>. SingleStepTests corpus passes for every variant.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

### Task 16: Logic family (AND, ORA, EOR)

**Opcodes:**
- AND: $29 imm, $25 zp, $35 zp,X, $2D abs, $3D abs,X, $39 abs,Y, $21 (ind,X), $31 (ind),Y
- ORA: $09 imm, $05 zp, $15 zp,X, $0D abs, $1D abs,X, $19 abs,Y, $01 (ind,X), $11 (ind),Y
- EOR: $49 imm, $45 zp, $55 zp,X, $4D abs, $5D abs,X, $59 abs,Y, $41 (ind,X), $51 (ind),Y

**Logic:** `cpu.a = cpu.a OP operand; set_nz(a)` where OP is `&`, `|`, or `^`.

### Task 17: Arithmetic (ADC, SBC)

**Opcodes:**
- ADC: $69 imm, $65 zp, $75 zp,X, $6D abs, $7D abs,X, $79 abs,Y, $61 (ind,X), $71 (ind),Y
- SBC: $E9 imm, $E5 zp, $F5 zp,X, $ED abs, $FD abs,X, $F9 abs,Y, $E1 (ind,X), $F1 (ind),Y

**Logic:** ADC: `a + operand + C` with carry-out → C, signed-overflow → V, low byte → A, set N/Z. SBC: same as ADC with operand bitwise-inverted (NES 6502 omits BCD; the D flag is settable but ignored). The "official" $EB SBC variant is added in Task 29 with the illegals.

> **Implementer note on the V flag:** the canonical formula is `V = ((a ^ result) & (operand ^ result) & 0x80) != 0`. SingleStepTests will catch any deviation.

### Task 18: Compare (CMP, CPX, CPY)

**Opcodes:**
- CMP: $C9 imm, $C5 zp, $D5 zp,X, $CD abs, $DD abs,X, $D9 abs,Y, $C1 (ind,X), $D1 (ind),Y
- CPX: $E0 imm, $E4 zp, $EC abs
- CPY: $C0 imm, $C4 zp, $CC abs

**Logic:** `r - operand` (no result stored); set N from bit 7, set Z if `r == operand`, set C if `r >= operand`.

### Task 19: BIT (bit test)

**Opcodes:** $24 zp, $2C abs

**Logic:** `result = a AND operand` (not stored). N = bit 7 of operand, V = bit 6 of operand, Z if `result == 0`.

### Task 20: Shift/Rotate (ASL, LSR, ROL, ROR)

**Opcodes:**
- ASL: $0A acc, $06 zp, $16 zp,X, $0E abs, $1E abs,X
- LSR: $4A acc, $46 zp, $56 zp,X, $4E abs, $5E abs,X
- ROL: $2A acc, $26 zp, $36 zp,X, $2E abs, $3E abs,X
- ROR: $6A acc, $66 zp, $76 zp,X, $6E abs, $7E abs,X

**Logic:**
- ASL: shift left, bit 7 → C, bit 0 ← 0
- LSR: shift right, bit 0 → C, bit 7 ← 0
- ROL: shift left, bit 7 → new C, bit 0 ← old C
- ROR: shift right, bit 0 → new C, bit 7 ← old C

**Cycle profile:** memory variants are RMW: read, dummy-write of original value, modify, write modified value. The addressing helpers handle the dummy reads; the RMW write-old-then-write-new pattern is encoded in the operation:

```rust
fn asl_mem<B: BusLike>(cpu: &mut Cpu, bus: &mut B, addr: u16) {
    let v = bus.read(addr);
    bus.write(addr, v); // dummy write of original
    let result = ((v as u16) << 1) as u8;
    cpu.p = (cpu.p & !flags::FLAG_C) | if v & 0x80 != 0 { flags::FLAG_C } else { 0 };
    set_nz(cpu, result);
    bus.write(addr, result);
}
```

For the accumulator variant, the same logic but operates on `cpu.a` and writes back to `cpu.a`.

### Task 21: Increment/Decrement (INC, DEC, INX, INY, DEX, DEY)

**Opcodes:**
- INC: $E6 zp, $F6 zp,X, $EE abs, $FE abs,X
- DEC: $C6 zp, $D6 zp,X, $CE abs, $DE abs,X
- INX: $E8, INY: $C8, DEX: $CA, DEY: $88

**Logic:** memory: RMW (read, dummy write of original, +1 or -1, write); register: `r = r.wrapping_add_or_sub(1); set_nz(r)` plus a 2-cycle dummy read of PC as part of all 1-byte implied opcodes. SingleStepTests will catch missing dummy reads.

### Task 22: Branches (BCC, BCS, BEQ, BNE, BMI, BPL, BVC, BVS)

**Opcodes:** $90 BCC, $B0 BCS, $F0 BEQ, $D0 BNE, $30 BMI, $10 BPL, $50 BVC, $70 BVS

**Logic:** fetch signed 8-bit offset; check the condition; if taken, add cycle (dummy read of unmodified PC); if page-crossed, add another cycle (dummy read of unmasked-PC).

```rust
fn branch_if<B: BusLike>(cpu: &mut Cpu, bus: &mut B, taken: bool) {
    let offset = addr::relative(cpu, bus);
    if taken {
        let _ = bus.read(cpu.pc); // dummy read at unmodified PC
        let new_pc = (cpu.pc as i32 + offset as i32) as u16;
        if (cpu.pc & 0xFF00) != (new_pc & 0xFF00) {
            // Page-crossed: extra dummy read at unmasked-PC.
            let _ = bus.read((cpu.pc & 0xFF00) | (new_pc & 0x00FF));
        }
        cpu.pc = new_pc;
    }
}
```

### Task 23: Jump/Stack (JMP, JSR, RTS)

**Opcodes:** $4C JMP abs, $6C JMP ind (with page-wrap bug from `addressing::ind`), $20 JSR, $60 RTS

**JSR cycle profile:** read target low byte, internal op (read of PC), push PCH, push PCL, read target high byte. PC at push time is the address of the JSR opcode + 2 (i.e., the last byte of the JSR instruction, NOT the byte after).

**RTS cycle profile:** read PC byte (dummy), increment SP read, pull PCL, pull PCH, increment PC, dummy read at PC.

### Task 24: Stack ops (PHA, PHP, PLA, PLP)

**Opcodes:** $48 PHA, $08 PHP, $68 PLA, $28 PLP

**Logic:** Push: bus.write to `0x0100 + sp`, decrement sp. Pull: increment sp, bus.read from `0x0100 + sp`. PHP pushes P with B+U bits set. PLP loads P with B/U masked off (B remains 0, U remains 1 in CPU's internal P).

### Task 25: BRK + RTI

**Opcodes:** $00 BRK, $40 RTI

**BRK cycle profile (7 cycles):** fetch opcode, fetch padding byte (dummy), push PCH, push PCL, push P with B|U set, read $FFFE (low IRQ vector), read $FFFF (high), set I flag.

**RTI cycle profile:** dummy read, increment SP read, pull P (mask B, set U), pull PCL, pull PCH.

### Task 26: Transfer (TAX, TAY, TXA, TYA, TSX, TXS)

**Opcodes:** $AA TAX, $A8 TAY, $8A TXA, $98 TYA, $BA TSX, $9A TXS

**Logic:** copy register; set N/Z (except TXS — TXS does NOT set flags).

### Task 27: Status flag ops (CLC, SEC, CLI, SEI, CLV, CLD, SED)

**Opcodes:** $18 CLC, $38 SEC, $58 CLI, $78 SEI, $B8 CLV, $D8 CLD, $F8 SED

**Logic:** set/clear the corresponding flag bit. Each is 2 cycles (opcode read + dummy read of PC).

### Task 28: NOPs (official + multiple unofficial variants)

**Opcodes:**
- $EA (the only "official" NOP)
- Implied unofficial NOPs: $1A, $3A, $5A, $7A, $DA, $FA (1-byte, 2 cycles)
- Immediate unofficial NOP: $80 (2 bytes, 2 cycles — the imm operand is read but discarded)
- Zero-page unofficial NOPs: $04, $44, $64 (2 bytes, 3 cycles)
- Zero-page,X unofficial NOPs: $14, $34, $54, $74, $D4, $F4 (2 bytes, 4 cycles)
- Absolute unofficial NOPs: $0C (3 bytes, 4 cycles)
- Absolute,X unofficial NOPs: $1C, $3C, $5C, $7C, $DC, $FC (3 bytes, 4 or 5 cycles depending on page-cross)

**Logic:** read the operand bytes, discard them. The cycle profile (and thus the bus access trace SingleStepTests checks) comes from issuing the right `bus.read` calls.

> **Implementer note:** treat the absolute,X NOPs as "read variant" (page-cross-only dummy read), matching how a real 6502 handles them. SingleStepTests will catch a mistake.

---

## Phase F — Illegal Opcodes (Tasks 29–34)

### Task 29: Stable illegals — LAX, SAX, DCP, ISC

**Opcodes:**
- LAX (LDA + LDX combined): $A3 (ind,X), $A7 zp, $AF abs, $B3 (ind),Y, $B7 zp,Y, $BF abs,Y. *Excludes $AB which is the magic-constant variant covered in Task 33.*
- SAX (A AND X store, no flag changes): $83 (ind,X), $87 zp, $8F abs, $97 zp,Y
- DCP (DEC + CMP): $C3 (ind,X), $C7 zp, $CF abs, $D3 (ind),Y, $D7 zp,X, $DB abs,Y, $DF abs,X
- ISC / ISB (INC + SBC): $E3 (ind,X), $E7 zp, $EF abs, $F3 (ind),Y, $F7 zp,X, $FB abs,Y, $FF abs,X

### Task 30: Stable illegals — SLO, RLA, SRE, RRA

**Opcodes:**
- SLO (ASL + ORA): $03, $07, $0F, $13, $17, $1B, $1F
- RLA (ROL + AND): $23, $27, $2F, $33, $37, $3B, $3F
- SRE (LSR + EOR): $43, $47, $4F, $53, $57, $5B, $5F
- RRA (ROR + ADC): $63, $67, $6F, $73, $77, $7B, $7F

(Each is RMW — read, dummy write, modify, write — across all 7 addressing modes.)

### Task 31: Stable illegals — ANC, ALR, ARR, AXS, official-illegal SBC

**Opcodes:**
- ANC (AND + carry from bit 7): $0B, $2B
- ALR / ASR (AND + LSR): $4B
- ARR (AND + ROR with weird flags): $6B. The flag handling is unusual: V = bit 6 ⊕ bit 5 of result; C = bit 6 of result; N/Z from result.
- AXS / SBX (CMP-style A AND X minus operand → X): $CB
- $EB: official-illegal SBC (alias for $E9)

### Task 32: SHX/SHY/SHA/TAS family (with page-cross instability note)

**Opcodes:**
- SHY: $9C abs,X — `Y AND (high+1)` stored at addr; if page-crossed, the high byte of the *target address itself* gets ANDed too.
- SHX: $9E abs,Y — same as SHY but with X and Y swapped roles.
- SHA / AHX: $93 (ind),Y, $9F abs,Y — `A AND X AND (high+1)` stored.
- TAS / SHS: $9B abs,Y — `S = A AND X; store S AND (high+1)`.

**Logic implementation:** match the most-cited reference behavior (which is what SingleStepTests assumes). Add a comment block at the top of these handlers:

```rust
// Note: real 6502 hardware behavior of SHX/SHY/SHA/TAS varies with chip,
// temperature, and decoupling. We implement the "common reference"
// behavior used by SingleStepTests/65x02 and Nintendulator. No commercial
// game depends on these.
```

### Task 33: XAA/LXA (magic-constant illegals)

**Opcodes:** $8B XAA / ANE, $AB LXA / LAX#imm

**Logic:** `A = (A | magic) & X & operand` for XAA; `A = X = (A | magic) & operand` for LXA. The "magic" value SingleStepTests uses is what most references call CONST = 0xFF (matching Visual6502 simulation). A long block comment at the top explaining the variance.

### Task 34: JAM/KIL/HLT — hang behavior

**Opcodes:** $02, $12, $22, $32, $42, $52, $62, $72, $92, $B2, $D2, $F2 (all variants)

**Logic:** set `cpu.jammed = true`. `Cpu::step` already handles the jammed case (just keep ticking the bus on PC reads).

> **Implementer note:** SingleStepTests for these opcodes will check that PC doesn't advance and that the CPU repeatedly reads from the same PC. The `cpu.jammed` flag plus the existing handler in `Cpu::step` should produce the right behavior; verify carefully.

---

## Phase G — Interrupts (Tasks 35–37)

### Task 35: Reset vector validation

The reset path (Task 9) is already covered. This task adds an integration-style test where a multi-byte program sets up state, then a second `Cpu::reset(&mut bus)` from arbitrary state correctly returns to the reset vector.

**Files:**
- Modify: `crates/nies-core/src/cpu/mod.rs` (add a unit test)

(Per the bite-sized rule: just one test asserting reset behavior end-to-end. Concrete assertions inline in the file.)

### Task 36: NMI handling

**Files:**
- Modify: `crates/nies-core/src/cpu/mod.rs` — implement `service_nmi`
- Add unit test driving NMI through the CPU end-to-end

**NMI cycle profile (7 cycles):** dummy read of PC, dummy read of PC, push PCH, push PCL, push P with B clear and U set, read $FFFA (low NMI vector), read $FFFB (high). Set I flag, clear `nmi_pending`.

```rust
fn service_nmi<B: crate::bus::BusLike>(&mut self, bus: &mut B) {
    let _ = bus.read(self.pc);
    let _ = bus.read(self.pc);
    self.push(bus, (self.pc >> 8) as u8);
    self.push(bus, (self.pc & 0xFF) as u8);
    let p = (self.p & !flags::FLAG_B) | flags::FLAG_U;
    self.push(bus, p);
    self.p |= flags::FLAG_I;
    let lo = bus.read(0xFFFA) as u16;
    let hi = bus.read(0xFFFB) as u16;
    self.pc = (hi << 8) | lo;
    self.nmi_pending = false;
}

fn push<B: crate::bus::BusLike>(&mut self, bus: &mut B, val: u8) {
    bus.write(0x0100 | self.sp as u16, val);
    self.sp = self.sp.wrapping_sub(1);
}
```

### Task 37: IRQ handling

**Files:**
- Modify: `crates/nies-core/src/cpu/mod.rs` — implement `service_irq`

**IRQ cycle profile:** identical to NMI but reads the IRQ vector at $FFFE/$FFFF. Pushed P has B clear and U set. The IRQ is *not* serviced if the I flag is already set (the dispatcher checks this before calling `service_irq`).

> **Note:** at M1 there is no IRQ source — the bus's `mapper.irq_pending()` always returns false (NROM has no IRQ), and the APU stub doesn't generate frame IRQs. The infrastructure has to exist anyway because nestest tests BRK/IRQ paths.

---

## Phase H — Test ROM Harness (Tasks 38–46)

### Task 38: Test ROM type + the $6000 status protocol

**Files:**
- Create: `crates/nies-core/tests/roms/manifest.toml` (placeholder — populated as ROMs land)
- Create: `crates/nies-core/tests/test_roms.rs` (harness, no test ROMs yet)

The harness loads a ROM file, builds a `Cartridge`, builds a `Bus`, builds a `Cpu`, runs `cpu.reset(&mut bus)`, then loops calling `cpu.step(&mut bus)` until either:
- $6000 contains a non-`0x80` value (test complete; success if 0x00, failure otherwise)
- A wall-clock timeout (default 60 seconds) expires
- A frame-count timeout (default 3000 frames) expires

The status string is read from $6004 onward as null-terminated ASCII (or up to a max length).

```rust
//! Integration test harness for blargg-style and nestest test ROMs.
//! Each ROM follows the convention: writes status code to $6000-$6003
//! and an ASCII status string to $6004-$60FF.

use nies_core::bus::{Bus, BusLike};
use nies_core::cartridge::Cartridge;
use nies_core::cpu::Cpu;
use nies_core::mapper::MapperKind;

pub fn run_test_rom(path: &str, max_cycles: u64) -> RomResult {
    let bytes = std::fs::read(path).expect("read rom");
    let cart = Cartridge::from_bytes(&bytes).expect("parse rom");
    let mapper = MapperKind::from_cartridge(&cart).expect("build mapper");
    let mut bus = Bus::new(mapper);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    // Wait for the ROM to set the magic ready handshake at $6001-$6003.
    // Per blargg's protocol, the ROM writes 0xDB, 0x14, 0x65 to $6001-$6003
    // when it begins running; we don't need to verify it but should at
    // least allow the bus to settle.
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
    RomResult::Done {
        status,
        message: s,
    }
}

#[derive(Debug)]
pub enum RomResult {
    Done { status: u8, message: String },
    Timeout,
}
```

- [ ] **Step 1: Write the harness module**
- [ ] **Step 2: Verify it compiles**
- [ ] **Step 3: Commit** — `test(core): test ROM harness for $6000 status protocol`

### Task 39: Vendor `nestest.nes` and the Nintendulator log

Download `nestest.nes` from the standard kevtris distribution (well-known SHA-256). Vendor under `crates/nies-core/tests/roms/nestest/nestest.nes`. Vendor the `nestest.log` file from Nintendulator (also kevtris-redistributed).

- [ ] **Step 1: Download and verify hashes**

```bash
# Verify expected SHA-256 hashes (publicly known)
# nestest.nes: 4131307f 4e803c2c 8c1c627e f2d0aa3a fb05d4f1 0db40a99 0d7d2bcd 0a08f9d6
# nestest.log: hash varies by source; document in LICENSES.md
```

- [ ] **Step 2: Place under `crates/nies-core/tests/roms/nestest/`**
- [ ] **Step 3: Add an entry to `LICENSES.md` documenting source + license**
- [ ] **Step 4: Commit** — `test(core): vendor nestest.nes + Nintendulator log`

### Tasks 40–45: Vendor blargg test ROMs

Each task vendors one blargg test ROM (or sub-test directory) and adds a `#[test]` entry that runs it via the test ROM harness with an appropriate cycle/wall-clock budget. Use the SHA-256 hashes from blargg's redistribution to verify.

- **Task 40:** `cpu_instrs` (combined + 16 sub-tests) — `crates/nies-core/tests/roms/blargg/cpu_instrs/`
- **Task 41:** `instr_misc.nes` — `crates/nies-core/tests/roms/blargg/instr_misc.nes`
- **Task 42:** `instr_timing` (combined + 2 sub-tests) — `crates/nies-core/tests/roms/blargg/instr_timing/`
- **Task 43:** `cpu_dummy_reads.nes` — `crates/nies-core/tests/roms/blargg/cpu_dummy_reads.nes`
- **Task 44:** Add `#[test]` runners for every vendored ROM, with expected `RomResult::Done { status: 0, .. }`
- **Task 45:** Update `LICENSES.md` with a comprehensive provenance/hash table for every vendored ROM

### Task 46: Nestest comparison vs Nintendulator log

**Files:**
- Create: `crates/nies-core/tests/nestest_compare.rs`

The harness runs nestest in "automated" mode (PC starts at $C000, not $C004), captures the per-instruction trace as it runs, and compares byte-for-byte against the Nintendulator log. Format details: each log line has PC, opcode bytes, mnemonic + operand, A:XX X:XX Y:XX P:XX SP:XX, PPU:000,000, CYC:7. M1 ignores the PPU and CYC columns (those are cycle-precision details that depend on PPU); we compare PC + registers + flags.

```rust
//! Nestest "automated" mode: starts PC at $C000 and steps through ~8991
//! instructions, comparing each one to the Nintendulator log.

use std::io::{BufRead, BufReader};

const NESTEST_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/roms/nestest/nestest.nes");
const LOG_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/roms/nestest/nestest.log");

#[test]
fn nestest_matches_nintendulator_log() {
    // ... (full implementation: parse log, run CPU, compare line by line)
}
```

- [ ] **Step 1: Write the test**
- [ ] **Step 2: Run** — `cargo test -p nies-core --test nestest_compare`
- [ ] **Step 3: Commit** — `test(cpu): nestest matches Nintendulator log byte-for-byte`

---

## Phase I — Final Gates (Tasks 47–52)

### Task 47: SingleStepTests sweep — every opcode green

The 256 per-opcode `#[test] fn opcode_NN_*()` entries already in `tests/singlestep_tests.rs` (added across Tasks 13, 15, 16-21, 22-28, 29-34) collectively *are* the gate. A single named sweep test was tried in an earlier revision of this plan but was removed: cargo's test runner parallelizes the 256 individual `#[test]` functions across cores (~1.5-2 s aggregate), whereas a single sweep test runs them serially in one thread (~25 s). The aggregate command `cargo test -p nies-core --test singlestep_tests opcode_` runs every per-opcode test by name pattern and is the equivalent of the old sweep.

**No new code in this task.** The verification command below confirms all 256 per-opcode tests are present and pass.

- [ ] **Step 1: Confirm.** Run `cargo test -p nies-core --test singlestep_tests opcode_` and verify the test count is exactly 256.
- [ ] **Step 2: No commit needed for this task** — the per-opcode tests landed in their respective opcode-family commits.

### Task 48: Run all blargg cpu_* tests as a single integration suite

**Files:**
- Modify: `crates/nies-core/tests/test_roms.rs`

A single `#[test]` per vendored ROM that calls `run_test_rom(path, 60_000_000_cycles)` and asserts success.

```rust
#[test] fn blargg_cpu_instrs() { /* ... */ }
#[test] fn blargg_instr_misc() { /* ... */ }
#[test] fn blargg_instr_timing() { /* ... */ }
#[test] fn blargg_cpu_dummy_reads() { /* ... */ }
```

- [ ] **Step 1:** Confirm all green
- [ ] **Step 2: Commit** — `test(cpu): blargg cpu_* test ROMs all pass`

### Task 49: Workspace fmt + clippy

- [ ] **Step 1:** `cargo fmt --all -- --check`
- [ ] **Step 2:** `cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings`
- [ ] **Step 3:** `cargo clippy -p nies-web --target wasm32-unknown-unknown --all-targets -- -D warnings`

If anything fails: fix and re-verify.

### Task 50: Update LICENSES.md with full ROM provenance

Add a new section listing every vendored test ROM with: source URL, author, redistribution permission, SHA-256 hash. Then commit.

### Task 51: Re-export and documentation pass

**Files:**
- Modify: `crates/nies-core/src/lib.rs`

Add convenient re-exports for the public API:

```rust
pub use bus::{Bus, BusLike};
pub use cartridge::{Cartridge, CartridgeError, Mirroring, NesFormat};
pub use cpu::Cpu;
pub use mapper::{MapperImpl, MapperKind};
```

- [ ] **Step 1: Add re-exports**
- [ ] **Step 2: Run** `cargo doc -p nies-core --no-deps` and confirm clean
- [ ] **Step 3: Commit**

### Task 52: M1 completion commit + branch finishing

A small "completes M1" wrap-up commit (often empty if no further changes are needed) plus the merge to master via the finishing-a-development-branch skill.

- [ ] **Step 1:** Verify CI passes locally for the whole workspace.
- [ ] **Step 2: Final commit if needed.**
- [ ] **Step 3:** Use `superpowers:finishing-a-development-branch` to merge to master.

---

## Acceptance checklist for M1

- [ ] All 256 6502 opcodes (official + illegal) implemented in `crates/nies-core/src/cpu/instructions.rs`.
- [ ] `cargo test -p nies-core --test singlestep_tests opcode_` runs 256 per-opcode tests, all pass (~2.5M test cases green; cargo parallelizes across CPU cores).
- [ ] `cargo test -p nies-core --test test_roms blargg_cpu_instrs blargg_instr_misc blargg_instr_timing blargg_cpu_dummy_reads` all pass.
- [ ] `cargo test -p nies-core --test nestest_compare nestest_matches_nintendulator_log` passes.
- [ ] iNES + NES 2.0 cartridge parser handles malformed input cleanly.
- [ ] NROM mapper (CPU side) implemented and exercised through the bus.
- [ ] Bus tick discipline: every `Bus::read` / `Bus::write` advances PPU by 3 dots, APU by 1 cycle, increments cycle counter.
- [ ] PPU/APU/Controller stubs in place for M2/M4/M5 to fill in without bus changes.
- [ ] NMI and IRQ servicing implemented (no IRQ source at M1; tested via nestest BRK paths).
- [ ] `cargo fmt --all -- --check` and `cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings` clean.
- [ ] `LICENSES.md` enumerates every vendored test ROM with source + redistribution permission.
- [ ] Branch merged to master.
