# M3 — Frontend Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **All units are agent-dispatched; the user reviews diffs at per-unit checkpoints** (Eric opted for full dispatch on the GPU layer this milestone). Pause at each `--- CHECKPOINT ---` for the user to read the diff and test output before continuing.

**Goal:** Put the PPU's palette-index framebuffer on screen at 60 Hz on both the native (`nies-app`) and WASM (`nies-web`) frontends, driven by a new `Nes` top-level core driver, and lock in a cross-platform golden-hash gate.

**Architecture:** A thin `Nes { cpu, bus }` driver in `nies-core` exposes `from_rom_bytes` / `run_frame` / `frame`. A platform-agnostic `NesRenderer` in `nies-ui` uploads the 256×240 index framebuffer into an R8Uint texture and runs a WGSL fullscreen-triangle shader that looks each index up in a 64-entry FBX Smooth palette LUT, integer-scaled and letterboxed via `set_viewport`. Both binaries shrink to: bootstrap GPU → build `Nes` + `NesRenderer` → per redraw `run_frame` → `upload_frame` → `render` → present (vsync-paced, temporary until M5).

**Tech Stack:** Rust 2024, `wgpu 29` (WGSL shaders), `winit 0.30`. New dev-dependency `wasm-bindgen-test` in `nies-web` for the cross-platform determinism test (run via `wasm-pack test --headless --chrome`). No new runtime crates. FBX Smooth palette vendored as a 192-byte `.pal` asset.

**Predecessor design spec:** [`docs/superpowers/specs/2026-05-30-m3-frontend-rendering-design.md`](../specs/2026-05-30-m3-frontend-rendering-design.md) — read it before this plan. The spec is rationale; this plan is execution.

---

## Reference: spec section ↔ task mapping

| Design spec § | Requirement | Tasks |
|---|---|---|
| §3 | `Nes` driver (`from_rom_bytes`/`run_frame`/`frame`/`frame_count`/`reset`) | 2, 3, 4 |
| §5.3 | Single-source demo ROM bytes (`demo_rom_bytes`) | 1 |
| §4.1 | `NesRenderer` interface | 9, 10 |
| §4.2 | R8Uint texture + WGSL palette-LUT shader + integer-scaled blit | 7, 8, 9, 10 |
| §4.3 | FBX Smooth 64-entry palette LUT, pinned as data | 6, 7 |
| §4.4 | WebGL2 downlevel / R8Unorm fallback documented | 10 |
| §5.1 | Native frontend: CLI path + embedded fallback, vsync loop | 11, 12 |
| §5.2 | Web frontend: embedded demo, same loop | 13 |
| §6.1 | Native golden-hash test on `Nes` | 5 |
| §6.2 | `wasm-bindgen-test` cross-platform determinism | 14 |
| §6.3 | CI: `wasm-pack test` + bundle-size record | 15 |
| §9 | Global spec amendments (M3 gate, palette = FBX Smooth) | 16 |

---

## Notes for the implementer

- **Working directory** is `/Users/eperdew/Software/nies`. The branch `m3-frontend-rendering` already exists with the design spec committed at the branch head.
- **State from M2 (merged to `master`):** `Cpu` complete; `Bus` with tick discipline (`pub ppu: Ppu`, `pub fn new(MapperKind) -> Self`); `Ppu` per-dot machine exposing `frame() -> &[u8; 256*240]` (palette indices 0..=63) and `frames() -> u64`; NROM mapper; `Cartridge::from_bytes(&[u8]) -> Result<_, CartridgeError>`; `MapperKind::from_cartridge(&Cartridge) -> Result<_, CartridgeError>`; `Cpu::new()`, `Cpu::reset<B: BusLike>(&mut self, &mut B)`, `Cpu::step<B: BusLike>(&mut self, &mut B)`. `Bus: BusLike`.
- **The frontends are still M0 sentinel-clears.** `nies-app/src/main.rs` and `nies-web/src/lib.rs` each hold a duplicated `GpuState` that clears magenta. The wgpu-29 bootstrap there (instance, adapter, device, surface config, `CurrentSurfaceTexture` acquire enum, `RenderPassColorAttachment { depth_slice: None }`, `RenderPassDescriptor { multiview_mask: None, .. }`) is correct and is **kept**; only the clear is replaced by a real render. `nies-web` additionally downlevels to `Limits::downlevel_webgl2_defaults()` and uses the `EventLoopProxy<UserEvent::GpuReady>` async path — preserve both.
- **TDD where it pays.** Core logic (`Nes`, `demo_rom_bytes`, integer-scale math, palette parsing, golden hash) is unit-tested first. GPU pipeline code and frontend event loops are **not** unit-tested (CI runners have no GPU; a wgpu device request would fail). They are verified by `cargo build` + the golden-hash gate + manual eyeball. Don't add GPU-device unit tests.
- **Per-task commits.** One commit per task, single-quoted message as shown, ending with the `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>` line. Working tree clean after each task.
- **`cargo fmt --all` and `cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings`** must stay clean after every task. For `nies-web`/`nies-ui` wasm paths also run `cargo clippy -p nies-web --target wasm32-unknown-unknown --all-targets -- -D warnings`.
- **Do not introduce egui, input handling, audio, or file dialogs.** Those are M10/M4/M5/later. M3 is pixels-on-screen only.

---

## File map

New files:

```
crates/nies-core/
└── src/
    └── nes.rs                 (Nes driver: from_rom_bytes/run_frame/frame/frame_count/reset + demo_rom_bytes)
crates/nies-ui/
├── assets/
│   └── smooth_fbx.pal         (vendored 192-byte FBX Smooth palette; 64×RGB)
└── src/
    ├── palette.rs             (parse .pal → [[u8;3];64]; FBX Smooth)
    ├── scaling.rs             (pure integer-scale viewport math)
    ├── renderer.rs            (NesRenderer: texture + pipeline + LUT + render)
    └── nes.wgsl               (fullscreen-triangle + palette-LUT shader)
crates/nies-web/
└── tests/
    └── determinism_wasm.rs    (wasm-bindgen-test: demo_ntsc golden hash == native constant)
```

Modified files:

```
crates/nies-core/src/lib.rs    (re-export Nes, demo_rom_bytes)
crates/nies-core/tests/ppu_determinism.rs  (rewrite onto Nes; pin golden constant)
crates/nies-ui/src/lib.rs      (module decls + re-exports)
crates/nies-ui/Cargo.toml      (add wgpu)
crates/nies-app/src/main.rs    (real render loop; delete magenta clear; CLI arg)
crates/nies-web/src/lib.rs     (real render loop; delete magenta clear; embedded demo)
crates/nies-web/Cargo.toml     (add wasm-bindgen-test dev-dep)
.github/workflows/ci.yml       (wasm job: wasm-pack test + size record)
LICENSES.md                    (FBX Smooth palette provenance)
docs/superpowers/specs/2026-05-02-nes-emulator-design.md  (§8 M3 gate, §9 palette)
```

The canonical `demo_ntsc.nes` stays at `crates/nies-core/tests/roms/nmi_sync/demo_ntsc.nes`; `demo_rom_bytes()` embeds it via `include_bytes!` so binaries and both tests share one byte source (spec §5.3).

---

## Phase A — Unit 1: `Nes` driver + golden-hash gate (native half)

Introduces the top-level driver and upgrades the M2 self-determinism check into a pinned golden hash.

### Task 1: `demo_rom_bytes()` accessor

**Files:**
- Modify: `crates/nies-core/src/nes.rs` (created here)
- Modify: `crates/nies-core/src/lib.rs`

- [ ] **Step 1: Create `nes.rs` with the accessor and a failing test**

Create `crates/nies-core/src/nes.rs`:

```rust
//! `Nes` — top-level emulator driver (CPU + Bus), and the embedded demo ROM.
//!
//! Pure logic: honors the crate's no-I/O contract. `include_bytes!` is a
//! compile-time data embed, not runtime I/O.

/// Bytes of the bundled `nmi_sync/demo_ntsc.nes` test ROM. Single source
/// shared by both frontends and the golden-hash tests (spec §5.3).
pub fn demo_rom_bytes() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/roms/nmi_sync/demo_ntsc.nes"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;

    #[test]
    fn demo_rom_parses_as_cartridge() {
        let bytes = demo_rom_bytes();
        assert!(bytes.len() > 16, "demo ROM should be larger than an iNES header");
        Cartridge::from_bytes(bytes).expect("demo ROM parses as a cartridge");
    }
}
```

Add to `crates/nies-core/src/lib.rs` after the existing `pub mod` block:

```rust
pub mod nes;
```

and to the re-export block:

```rust
pub use nes::{demo_rom_bytes, Nes};
```

(`Nes` does not exist yet — it is added in Task 2. To keep this task compiling on its own, **omit `Nes` from the re-export for now** and re-export only `demo_rom_bytes`; Task 2 adds `Nes` to the re-export. Use this line in Task 1:)

```rust
pub use nes::demo_rom_bytes;
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test -p nies-core --lib nes::tests::demo_rom_parses_as_cartridge`
Expected: PASS (the ROM is already vendored).

- [ ] **Step 3: fmt + clippy**

Run: `cargo fmt --all && cargo clippy -p nies-core --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/nies-core/src/nes.rs crates/nies-core/src/lib.rs
git commit -m 'feat(core): embed demo_ntsc ROM via demo_rom_bytes()

Single-source accessor for the bundled nmi_sync/demo_ntsc.nes, shared by
both frontends and the M3 golden-hash tests (spec §5.3). include_bytes is
a compile-time embed; the no-I/O contract is preserved.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 2: `Nes::from_rom_bytes` + `frame` / `frame_count`

**Files:**
- Modify: `crates/nies-core/src/nes.rs`
- Modify: `crates/nies-core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/nies-core/src/nes.rs` `tests` module:

```rust
    #[test]
    fn new_nes_has_zero_frames_and_full_framebuffer() {
        let nes = Nes::from_rom_bytes(demo_rom_bytes()).expect("build Nes");
        assert_eq!(nes.frame_count(), 0);
        assert_eq!(nes.frame().len(), 256 * 240);
    }

    #[test]
    fn from_rom_bytes_rejects_garbage() {
        assert!(Nes::from_rom_bytes(&[0, 1, 2, 3]).is_err());
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p nies-core --lib nes::tests::new_nes_has_zero_frames`
Expected: FAIL — `Nes` is not defined.

- [ ] **Step 3: Implement `Nes` construction + accessors**

At the top of `crates/nies-core/src/nes.rs` (above the `demo_rom_bytes` fn), add:

```rust
use crate::bus::Bus;
use crate::cartridge::{Cartridge, CartridgeError};
use crate::cpu::Cpu;
use crate::mapper::MapperKind;

/// Top-level NES driver: owns the CPU and the bus (which owns PPU, APU,
/// mapper, RAM). The single entry point a frontend uses to run the
/// emulator. No rendering/audio/input here — those belong to later
/// milestones; M3 needs only "run a frame, give me the framebuffer".
pub struct Nes {
    cpu: Cpu,
    bus: Bus,
}

impl Nes {
    /// Parse an iNES / NES 2.0 image, build the mapper and bus, and run
    /// the CPU reset sequence. Returns the cartridge parse/mapper error
    /// on malformed or unsupported ROMs.
    pub fn from_rom_bytes(bytes: &[u8]) -> Result<Self, CartridgeError> {
        let cart = Cartridge::from_bytes(bytes)?;
        let mapper = MapperKind::from_cartridge(&cart)?;
        let mut bus = Bus::new(mapper);
        let mut cpu = Cpu::new();
        cpu.reset(&mut bus);
        Ok(Self { cpu, bus })
    }

    /// The current frame's palette-index framebuffer (one byte per pixel,
    /// value 0..=63), row-major, 256×240.
    pub fn frame(&self) -> &[u8; 256 * 240] {
        self.bus.ppu.frame()
    }

    /// Total frames completed since power-on (monotonic).
    pub fn frame_count(&self) -> u64 {
        self.bus.ppu.frames()
    }
}
```

Update the `lib.rs` re-export from Task 1 to include `Nes`:

```rust
pub use nes::{demo_rom_bytes, Nes};
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p nies-core --lib nes::tests`
Expected: PASS (all three `nes::tests`).

- [ ] **Step 5: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy -p nies-core --all-targets -- -D warnings
git add crates/nies-core/src/nes.rs crates/nies-core/src/lib.rs
git commit -m 'feat(core): add Nes driver construction + frame accessors

Nes { cpu, bus } owns the system; from_rom_bytes wraps the
parse→mapper→bus→reset sequence that previously lived inline in tests.
frame()/frame_count() delegate to the PPU.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 3: `Nes::run_frame` + `reset`

**Files:**
- Modify: `crates/nies-core/src/nes.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module:

```rust
    #[test]
    fn run_frame_advances_frame_count_by_one() {
        let mut nes = Nes::from_rom_bytes(demo_rom_bytes()).expect("build Nes");
        let before = nes.frame_count();
        nes.run_frame();
        assert_eq!(nes.frame_count(), before + 1);
    }

    #[test]
    fn run_frame_is_repeatable() {
        let mut nes = Nes::from_rom_bytes(demo_rom_bytes()).expect("build Nes");
        for _ in 0..10 {
            nes.run_frame();
        }
        assert_eq!(nes.frame_count(), 10);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p nies-core --lib nes::tests::run_frame_advances`
Expected: FAIL — no `run_frame` method.

- [ ] **Step 3: Implement `run_frame` and `reset`**

Add to `impl Nes`:

```rust
    /// Run the CPU until the PPU completes one frame. Executes whole
    /// instructions; the boundary is the first instruction that pushes
    /// the PPU frame counter over.
    pub fn run_frame(&mut self) {
        let target = self.bus.ppu.frames() + 1;
        while self.bus.ppu.frames() < target {
            self.cpu.step(&mut self.bus);
        }
    }

    /// Soft reset: re-run the CPU reset sequence. Does not rebuild the
    /// cartridge or clear the framebuffer.
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.bus);
    }
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p nies-core --lib nes::tests`
Expected: PASS (all five).

- [ ] **Step 5: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy -p nies-core --all-targets -- -D warnings
git add crates/nies-core/src/nes.rs
git commit -m 'feat(core): Nes::run_frame and Nes::reset

run_frame steps the CPU until the PPU frame counter increments; reset
re-runs the CPU reset sequence.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 4: Refactor `ppu_determinism.rs` onto `Nes`

**Files:**
- Modify: `crates/nies-core/tests/ppu_determinism.rs`

- [ ] **Step 1: Replace the manual driver loop with `Nes`**

Rewrite `crates/nies-core/tests/ppu_determinism.rs` to:

```rust
//! M3 PPU determinism: demo_ntsc.nes produces a stable framebuffer hash.
//! Task 4 keeps this as a self-determinism check (run-twice equality);
//! Task 5 upgrades it to a pinned golden constant.

use nies_core::Nes;
use std::hash::{DefaultHasher, Hash, Hasher};

const N_FRAMES: u64 = 200;

#[test]
fn demo_ntsc_framebuffer_is_self_deterministic() {
    let h1 = run_and_hash(N_FRAMES);
    let h2 = run_and_hash(N_FRAMES);
    assert_eq!(h1, h2, "framebuffer hash differs across identical runs");
}

fn run_and_hash(n_frames: u64) -> u64 {
    let mut nes = Nes::from_rom_bytes(nies_core::demo_rom_bytes()).expect("build Nes");
    for _ in 0..n_frames {
        nes.run_frame();
    }
    let mut h = DefaultHasher::new();
    nes.frame().hash(&mut h);
    h.finish()
}
```

- [ ] **Step 2: Run to verify it passes**

Run: `cargo test -p nies-core --test ppu_determinism`
Expected: PASS.

- [ ] **Step 3: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy -p nies-core --all-targets -- -D warnings
git add crates/nies-core/tests/ppu_determinism.rs
git commit -m 'refactor(test): drive ppu_determinism via Nes

Collapse the inline Cpu+Bus+step loop into Nes::run_frame; use
demo_rom_bytes() instead of std::fs. Still a self-determinism check;
golden constant lands next.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 5: Pin the golden constant (native gate)

**Files:**
- Modify: `crates/nies-core/tests/ppu_determinism.rs`

- [ ] **Step 1: Capture the actual hash**

Temporarily add this test to `ppu_determinism.rs` and run it with output shown:

```rust
#[test]
fn print_golden_hash() {
    eprintln!("GOLDEN = {:#018x}", run_and_hash(N_FRAMES));
}
```

Run: `cargo test -p nies-core --test ppu_determinism print_golden_hash -- --nocapture`
Record the printed `GOLDEN = 0x...` value. **This value is the pinned constant.** (It is stable by construction: the framebuffer is pure integer logic with no clock/RNG/threading — global spec §4.1.)

- [ ] **Step 2: Replace the temporary test with the pinned assertion**

Delete `print_golden_hash` and add (substituting the recorded value for `0xKEEP_THE_RECORDED_VALUE`):

```rust
/// Pinned golden hash of the demo_ntsc index framebuffer after N_FRAMES.
/// Recorded once from a known-good run (Task 5, Step 1). The same constant
/// is asserted under wasm32 in crates/nies-web/tests/determinism_wasm.rs;
/// keep the two in sync. A change here is a real determinism regression —
/// render the frame and diff against a reference before touching it.
const GOLDEN_FB_HASH: u64 = 0xKEEP_THE_RECORDED_VALUE;

#[test]
fn demo_ntsc_framebuffer_matches_golden_hash() {
    assert_eq!(
        run_and_hash(N_FRAMES),
        GOLDEN_FB_HASH,
        "demo_ntsc framebuffer hash drifted from the pinned golden value"
    );
}
```

- [ ] **Step 3: Run to verify it passes**

Run: `cargo test -p nies-core --test ppu_determinism`
Expected: PASS (both the self-determinism and golden tests).

- [ ] **Step 4: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy -p nies-core --all-targets -- -D warnings
git add crates/nies-core/tests/ppu_determinism.rs
git commit -m 'test(core): pin demo_ntsc golden framebuffer hash

Upgrade the M2 self-determinism check into a pinned golden constant — the
native half of the M3 gate (spec §6.1). The wasm half (Task 14) asserts
the same value.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

--- CHECKPOINT (end of Unit 1) ---
Show the user: the `Nes` API, the rewritten determinism test, and `cargo test -p nies-core` output. Confirm the golden constant is recorded before proceeding.

---

## Phase B — Unit 2: `NesRenderer` (nies-ui)

The shared, platform-agnostic renderer. GPU pipeline code is **not** unit-tested; the pure pieces (palette parsing, scale math) are.

### Task 6: Vendor the FBX Smooth palette + parse it

**Files:**
- Create: `crates/nies-ui/assets/smooth_fbx.pal` (vendored binary, 192 bytes)
- Create: `crates/nies-ui/src/palette.rs`
- Modify: `crates/nies-ui/src/lib.rs`
- Modify: `LICENSES.md`

**Vendoring note (prep, like the test ROMs):** `smooth_fbx.pal` is the canonical "Smooth (FBX)" NES palette: 192 bytes = 64 entries × 3 bytes (R, G, B). Source: FirebrandX's NES palette distribution (the "Smooth (FBX)" `.pal`). It is freely redistributable; add a `LICENSES.md` entry. If the file cannot be fetched in-session, the orchestrator vendors it as a prep step before this task runs (the asset is a binary blob, not generated code).

- [ ] **Step 1: Add the LICENSES.md entry**

Append to `LICENSES.md`:

```markdown
### Palettes

- `crates/nies-ui/assets/smooth_fbx.pal` — "Smooth (FBX)" NES palette by
  FirebrandX. 64 entries × RGB (192 bytes). Freely redistributable; used
  as the default M3 palette LUT.
```

- [ ] **Step 2: Write the failing palette test**

Create `crates/nies-ui/src/palette.rs`:

```rust
//! FBX Smooth NES palette: 64 RGB entries, the default M3 LUT.

/// Raw "Smooth (FBX)" palette: 64 entries × (R, G, B). Pinned as data so
/// the rendered colors are reproducible; the M3 golden hash is over
/// palette *indices*, so these exact bytes are cosmetic, not gated.
const SMOOTH_FBX: &[u8] = include_bytes!("../assets/smooth_fbx.pal");

/// The 64-entry palette as RGB triplets.
pub fn fbx_smooth() -> [[u8; 3]; 64] {
    assert_eq!(SMOOTH_FBX.len(), 64 * 3, "palette must be 192 bytes");
    let mut out = [[0u8; 3]; 64];
    for (i, chunk) in SMOOTH_FBX.chunks_exact(3).enumerate() {
        out[i] = [chunk[0], chunk[1], chunk[2]];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_has_64_entries() {
        let p = fbx_smooth();
        assert_eq!(p.len(), 64);
    }

    #[test]
    fn raw_palette_is_192_bytes() {
        assert_eq!(SMOOTH_FBX.len(), 192);
    }
}
```

Add to `crates/nies-ui/src/lib.rs`:

```rust
pub mod palette;
```

- [ ] **Step 3: Run to verify it passes**

Run: `cargo test -p nies-ui palette`
Expected: PASS. (If FAIL with a length panic, the vendored `.pal` is wrong size — re-vendor a 192-byte file.)

- [ ] **Step 4: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy -p nies-ui --all-targets -- -D warnings
git add crates/nies-ui/assets/smooth_fbx.pal crates/nies-ui/src/palette.rs crates/nies-ui/src/lib.rs LICENSES.md
git commit -m 'feat(ui): vendor FBX Smooth palette + parser

64-entry RGB LUT loaded from a vendored .pal. Default M3 palette
(global spec §9 decision). Colors are cosmetic — the golden hash is
index-based.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 7: Integer-scale viewport math (pure)

**Files:**
- Create: `crates/nies-ui/src/scaling.rs`
- Modify: `crates/nies-ui/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/nies-ui/src/scaling.rs`:

```rust
//! Pure integer-scaling math for the game viewport. Keeps the geometry
//! testable without a GPU.

/// NES visible resolution.
pub const NES_W: u32 = 256;
pub const NES_H: u32 = 240;

/// A centered, integer-scaled viewport rectangle within `target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Largest integer-scaled, centered NES rectangle that fits in
/// `target_w × target_h`. Scale is at least 1 (the image may overflow a
/// tiny window rather than vanish). The surrounding area is letterboxed.
pub fn integer_scale(target_w: u32, target_h: u32) -> Viewport {
    let scale = (target_w / NES_W).min(target_h / NES_H).max(1);
    let w = NES_W * scale;
    let h = NES_H * scale;
    let x = target_w.saturating_sub(w) / 2;
    let y = target_h.saturating_sub(h) / 2;
    Viewport { x, y, w, h }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_fit_scale_1() {
        assert_eq!(integer_scale(256, 240), Viewport { x: 0, y: 0, w: 256, h: 240 });
    }

    #[test]
    fn picks_largest_integer_scale_and_centers() {
        // 800×600: max scale = min(3, 2) = 2 → 512×480, centered.
        assert_eq!(
            integer_scale(800, 600),
            Viewport { x: (800 - 512) / 2, y: (600 - 480) / 2, w: 512, h: 480 }
        );
    }

    #[test]
    fn tiny_window_clamps_to_scale_1() {
        assert_eq!(integer_scale(100, 100), Viewport { x: 0, y: 0, w: 256, h: 240 });
    }
}
```

Add to `crates/nies-ui/src/lib.rs`:

```rust
pub mod scaling;
```

- [ ] **Step 2: Run to verify it passes**

Run: `cargo test -p nies-ui scaling`
Expected: PASS.

- [ ] **Step 3: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy -p nies-ui --all-targets -- -D warnings
git add crates/nies-ui/src/scaling.rs crates/nies-ui/src/lib.rs
git commit -m 'feat(ui): pure integer-scale viewport math

Largest centered integer scale fitting the target, clamped to ≥1.
Tested without a GPU; the renderer feeds this to set_viewport.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 8: WGSL palette-LUT shader

**Files:**
- Create: `crates/nies-ui/src/nes.wgsl`

- [ ] **Step 1: Write the shader**

Create `crates/nies-ui/src/nes.wgsl`:

```wgsl
// Fullscreen-triangle palette-LUT shader.
// Vertex stage emits an oversized triangle covering the viewport.
// Fragment stage reads the palette index from an R8Uint texture and
// looks the RGB up in a 64-entry uniform LUT.

struct Palette {
    colors: array<vec4<f32>, 64>,
};

@group(0) @binding(0) var index_tex: texture_2d<u32>;
@group(0) @binding(1) var<uniform> palette: Palette;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    // Oversized triangle: clip-space corners (-1,-1),(3,-1),(-1,3).
    let x = f32((vi << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(vi & 2u) * 2.0 - 1.0;
    var out: VsOut;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    // UV with row 0 at the top of the image.
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(256.0, 240.0);
    let coord = vec2<i32>(clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0)) * dim);
    let c = clamp(coord, vec2<i32>(0), vec2<i32>(255, 239));
    let index = textureLoad(index_tex, c, 0).r;
    return palette.colors[index];
}
```

- [ ] **Step 2: Verify it parses (build the crate that includes it)**

The shader is validated when `NesRenderer` loads it (Task 9) via `cargo build`. No standalone test. Confirm the file is saved and well-formed by eye.

- [ ] **Step 3: Commit**

```bash
git add crates/nies-ui/src/nes.wgsl
git commit -m 'feat(ui): WGSL palette-LUT fullscreen shader

Oversized-triangle vertex stage; fragment stage reads an R8Uint index
texture via textureLoad and indexes a 64-entry uniform palette LUT.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 9: `NesRenderer::new` + `upload_frame`

**Files:**
- Modify: `crates/nies-ui/Cargo.toml`
- Create: `crates/nies-ui/src/renderer.rs`
- Modify: `crates/nies-ui/src/lib.rs`

- [ ] **Step 1: Add wgpu to nies-ui**

Edit `crates/nies-ui/Cargo.toml` `[dependencies]` to add (keep it backend-agnostic so the same crate builds for native and wasm; binaries select backends):

```toml
wgpu = { version = "29", default-features = false, features = ["wgsl"] }
```

> If `wgpu` rejects the bare `wgsl` feature name on 29.x, use `default-features = true` here and rely on the binaries to constrain backends — verify `cargo build -p nies-ui` and `cargo build -p nies-web --target wasm32-unknown-unknown` both succeed before committing.

- [ ] **Step 2: Implement the renderer's resources + upload**

Create `crates/nies-ui/src/renderer.rs`:

```rust
//! `NesRenderer` — uploads the 256×240 palette-index framebuffer into an
//! R8Uint texture and draws it through the palette-LUT shader, integer
//! scaled and centered. Platform-agnostic: owns GPU resources but not the
//! surface, device, queue, or window — the binary passes those in.

use crate::palette::fbx_smooth;
use crate::scaling::{integer_scale, NES_H, NES_W};
use wgpu::util::DeviceExt;

pub struct NesRenderer {
    index_tex: wgpu::Texture,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl NesRenderer {
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let index_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nes-index-tex"),
            size: wgpu::Extent3d { width: NES_W, height: NES_H, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let index_view = index_tex.create_view(&wgpu::TextureViewDescriptor::default());

        // Palette LUT: 64 × vec4<f32> (RGB + pad), normalized 0..1.
        let pal = fbx_smooth();
        let mut lut = [0f32; 64 * 4];
        for (i, [r, g, b]) in pal.iter().enumerate() {
            lut[i * 4] = *r as f32 / 255.0;
            lut[i * 4 + 1] = *g as f32 / 255.0;
            lut[i * 4 + 2] = *b as f32 / 255.0;
            lut[i * 4 + 3] = 1.0;
        }
        let lut_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nes-palette-lut"),
            contents: bytemuck::cast_slice(&lut),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nes-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nes-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&index_view) },
                wgpu::BindGroupEntry { binding: 1, resource: lut_buf.as_entire_binding() },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nes-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("nes.wgsl").into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nes-pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nes-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { index_tex, pipeline, bind_group }
    }

    /// Upload one 256×240 palette-index frame into the GPU texture.
    pub fn upload_frame(&self, queue: &wgpu::Queue, frame: &[u8; 256 * 240]) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.index_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            frame,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(NES_W), // 256, already a 256-multiple
                rows_per_image: Some(NES_H),
            },
            wgpu::Extent3d { width: NES_W, height: NES_H, depth_or_array_layers: 1 },
        );
    }
}
```

> **wgpu 29 API note:** the exact names of the texture/buffer copy descriptor structs (`TexelCopyTextureInfo` / `TexelCopyBufferLayout` vs older `ImageCopyTexture` / `ImageDataLayout`) and `entry_point: Some(..)` vs `&str` shifted across recent wgpu versions. Cross-check against the working bootstrap in `crates/nies-app/src/main.rs` (already on 29) and adjust names if the compiler disagrees. `bytemuck` is needed for `cast_slice`; add `bytemuck = "1"` to `nies-ui` `[dependencies]` in this step if not already present.

Add to `crates/nies-ui/src/lib.rs`:

```rust
pub mod renderer;
pub use renderer::NesRenderer;
```

- [ ] **Step 3: Build to verify it compiles (native + wasm)**

Run: `cargo build -p nies-ui`
Then: `cargo build -p nies-web --target wasm32-unknown-unknown`
Expected: both compile. (Shader WGSL is validated at `create_shader_module` build time on native.)

- [ ] **Step 4: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy -p nies-ui --all-targets -- -D warnings
git add crates/nies-ui/Cargo.toml crates/nies-ui/src/renderer.rs crates/nies-ui/src/lib.rs
git commit -m 'feat(ui): NesRenderer resources + frame upload

R8Uint index texture, 64-entry palette uniform LUT, bind group, and
palette-LUT render pipeline. upload_frame writes the index framebuffer
each frame.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 10: `NesRenderer::render` (integer-scaled blit)

**Files:**
- Modify: `crates/nies-ui/src/renderer.rs`

- [ ] **Step 1: Implement `render`**

Add to `impl NesRenderer`:

```rust
    /// Draw the uploaded frame into `view`, integer-scaled and centered in
    /// `target` (width, height in physical pixels); letterbox the rest
    /// black. Caller submits the encoder.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        target: (u32, u32),
    ) {
        let vp = integer_scale(target.0, target.1);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("nes-render"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_viewport(vp.x as f32, vp.y as f32, vp.w as f32, vp.h as f32, 0.0, 1.0);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
```

> The `RenderPassColorAttachment { depth_slice: None }` and `RenderPassDescriptor { multiview_mask: None, .. }` fields match the M0 bootstrap in `nies-app`; copy their exact shape if wgpu 29 disagrees.

**WebGL2 downlevel note (spec §4.4):** the R8Uint + `texture_2d<u32>` + `textureLoad` path is GLES3-legal but verify it renders under the WebGL backend during Unit 4. If a backend rejects integer textures, switch the texture to `R8Unorm`, change the shader binding to `texture_2d<f32>` and the index read to `u32(round(textureLoad(...).r * 255.0))`. This stays confined to `NesRenderer`; the public API and the index-based golden hash are unaffected.

- [ ] **Step 2: Build to verify it compiles**

Run: `cargo build -p nies-ui && cargo build -p nies-web --target wasm32-unknown-unknown`
Expected: compile clean.

- [ ] **Step 3: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy -p nies-ui --all-targets -- -D warnings
git add crates/nies-ui/src/renderer.rs
git commit -m 'feat(ui): NesRenderer::render integer-scaled blit

Clears black, sets a centered integer-scaled viewport, draws the
fullscreen-triangle palette-LUT pass. Letterboxes non-integer windows.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

--- CHECKPOINT (end of Unit 2) ---
Show the user the renderer module and `cargo build -p nies-ui` + wasm build output. The renderer has no visual output yet — that arrives when the binaries call it (Units 3–4).

---

## Phase C — Unit 3: Native frontend (`nies-app`)

Replace the magenta clear with a real render loop.

### Task 11: Hold `Nes` + `NesRenderer`; render each frame

**Files:**
- Modify: `crates/nies-app/src/main.rs`

- [ ] **Step 1: Wire the emulator and renderer into the app**

In `crates/nies-app/src/main.rs`:

1. Add fields to `GpuState`: `renderer: nies_ui::NesRenderer` and a `nes: nies_core::Nes`. Construct the renderer in `GpuState::new` after the device/queue/config are built, using `config.format`:

```rust
let renderer = nies_ui::NesRenderer::new(&device, &queue, config.format);
```

and pass a `Nes` into `GpuState` (built in `App::resumed` from the ROM bytes — see Task 12; for this task, build it from `nies_core::demo_rom_bytes()`):

```rust
let nes = nies_core::Nes::from_rom_bytes(nies_core::demo_rom_bytes())
    .expect("demo ROM builds");
```

2. Replace the body of `GpuState::render` so that, instead of the clear-only pass, it runs a frame and draws it. Replace the `begin_render_pass { ... clear ... }` block with:

```rust
self.nes.run_frame();
self.renderer.upload_frame(&self.queue, self.nes.frame());
let mut encoder = self
    .device
    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
self.renderer.render(&mut encoder, &view, (self.config.width, self.config.height));
self.queue.submit(std::iter::once(encoder.finish()));
frame.present();
RenderOutcome::Presented
```

(Delete the now-unused `SENTINEL_CLEAR` const and the old encoder/clear block.)

3. Thread `nes` and `renderer` through `GpuState`'s struct definition and `GpuState::new`'s return. Keep the existing surface-acquire `match` (the `CurrentSurfaceTexture` enum handling) unchanged.

- [ ] **Step 2: Build + run (manual eyeball)**

Run: `cargo build -p nies-app`
Then: `cargo run -p nies-app`
Expected: a window opens showing the animated `demo_ntsc` output (not magenta), integer-scaled and centered, resizable with black letterboxing. Close the window to exit.

- [ ] **Step 3: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-app/src/main.rs
git commit -m 'feat(app): render the emulator framebuffer (native)

Native app drives Nes::run_frame per redraw and blits via NesRenderer.
Replaces the M0 magenta clear; vsync-paced (temporary until M5).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 12: Optional ROM path CLI arg

**Files:**
- Modify: `crates/nies-app/src/main.rs`

- [ ] **Step 1: Parse an optional ROM path, fall back to the embedded demo**

In `main()`, before building the event loop, read an optional path argument and load the bytes; pass them into `App` so `resumed`/`GpuState::new` builds the `Nes` from them:

```rust
let rom_bytes: Vec<u8> = match std::env::args().nth(1) {
    Some(path) => match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("failed to read ROM '{path}': {e}; using embedded demo");
            nies_core::demo_rom_bytes().to_vec()
        }
    },
    None => nies_core::demo_rom_bytes().to_vec(),
};
```

Store `rom_bytes` on `App` (add a field), and in `GpuState::new` build the `Nes` from those bytes instead of always the demo:

```rust
let nes = nies_core::Nes::from_rom_bytes(&rom_bytes)
    .unwrap_or_else(|e| {
        log::error!("ROM failed to parse ({e:?}); falling back to embedded demo");
        nies_core::Nes::from_rom_bytes(nies_core::demo_rom_bytes()).expect("demo ROM builds")
    });
```

(Thread `&rom_bytes` into `GpuState::new`'s signature. `std::fs` here is fine — `nies-app` is a frontend, not the core.)

- [ ] **Step 2: Build + run with and without an arg**

Run: `cargo run -p nies-app` → embedded demo renders.
Run: `cargo run -p nies-app -- crates/nies-core/tests/roms/nmi_sync/demo_ntsc.nes` → same demo via the path.
Run: `cargo run -p nies-app -- nonexistent.nes` → logs an error, falls back to the demo (no panic).

- [ ] **Step 3: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
git add crates/nies-app/src/main.rs
git commit -m 'feat(app): optional ROM path CLI arg

cargo run -p nies-app -- path/to.nes loads a ROM from disk; missing or
unparseable paths log and fall back to the embedded demo (no panic on
user input).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

--- CHECKPOINT (end of Unit 3) ---
User runs `cargo run -p nies-app` and eyeballs the demo rendering + resize letterboxing.

---

## Phase D — Unit 4: Web frontend (`nies-web`)

### Task 13: Render the embedded demo in the browser

**Files:**
- Modify: `crates/nies-web/src/lib.rs`

- [ ] **Step 1: Mirror the native render loop with the embedded demo**

In `crates/nies-web/src/lib.rs`, apply the same changes as Task 11 (the web `GpuState` is structurally identical):

1. Add `renderer: nies_ui::NesRenderer` and `nes: nies_core::Nes` fields to `GpuState`. Build the renderer in `GpuState::new` from `config.format`; build the `Nes` from `nies_core::demo_rom_bytes()` (web always uses the embedded demo — no file picker at M3).
2. Replace the clear-only block in `GpuState::render` with the run/upload/render/submit/present sequence from Task 11 Step 1.
3. Delete the unused `SENTINEL_CLEAR` const.
4. Preserve the `UserEvent::GpuReady` async bootstrap and the `downlevel_webgl2_defaults()` device request — do not touch those.

- [ ] **Step 2: Build (native-excluded crate → wasm target)**

Run: `cargo build -p nies-web --target wasm32-unknown-unknown`
Then: `trunk build` (debug is fine here; release is heavier)
Expected: both succeed.

- [ ] **Step 3: Manual browser check**

Run: `trunk serve` and open `http://127.0.0.1:8080`.
Expected: the canvas shows the animated `demo_ntsc` output (not magenta). **This is where the WebGL2 integer-texture path is validated** — if the canvas is black/garbage, apply the R8Unorm fallback from Task 10's note and rebuild.

- [ ] **Step 4: fmt + clippy + commit**

```bash
cargo fmt --all
cargo clippy -p nies-web --target wasm32-unknown-unknown --all-targets -- -D warnings
git add crates/nies-web/src/lib.rs
git commit -m 'feat(web): render the embedded demo framebuffer (WASM)

WASM frontend drives Nes::run_frame and blits via NesRenderer, same loop
as native. Always loads the embedded demo (no file picker at M3).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

--- CHECKPOINT (end of Unit 4) ---
User opens the browser build and confirms the demo renders. Resolve any WebGL2 fallback here before Unit 5.

---

## Phase E — Unit 5: Cross-platform gate + CI + spec amendment

### Task 14: `wasm-bindgen-test` golden-hash assertion

**Files:**
- Modify: `crates/nies-web/Cargo.toml`
- Create: `crates/nies-web/tests/determinism_wasm.rs`

- [ ] **Step 1: Add the dev-dependency**

In `crates/nies-web/Cargo.toml`, add:

```toml
[dev-dependencies]
wasm-bindgen-test = "0.3"
```

- [ ] **Step 2: Write the wasm determinism test**

Create `crates/nies-web/tests/determinism_wasm.rs` (substitute the **same** constant pinned in Task 5):

```rust
//! Cross-platform determinism gate: the demo_ntsc index framebuffer hashes
//! to the same pinned constant under wasm32 as it does natively
//! (crates/nies-core/tests/ppu_determinism.rs). Proves nies-core is
//! bit-identical on the web target. Run via:
//!   wasm-pack test --headless --chrome crates/nies-web

use nies_core::Nes;
use std::hash::{DefaultHasher, Hash, Hasher};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const N_FRAMES: u64 = 200;
/// MUST match GOLDEN_FB_HASH in crates/nies-core/tests/ppu_determinism.rs.
const GOLDEN_FB_HASH: u64 = 0xKEEP_THE_RECORDED_VALUE;

#[wasm_bindgen_test]
fn demo_ntsc_framebuffer_matches_golden_hash_on_wasm() {
    let mut nes = Nes::from_rom_bytes(nies_core::demo_rom_bytes()).expect("build Nes");
    for _ in 0..N_FRAMES {
        nes.run_frame();
    }
    let mut h = DefaultHasher::new();
    nes.frame().hash(&mut h);
    assert_eq!(h.finish(), GOLDEN_FB_HASH, "wasm framebuffer hash != native golden");
}
```

- [ ] **Step 3: Run the wasm test locally**

Run: `wasm-pack test --headless --chrome crates/nies-web`
Expected: PASS. (Requires `wasm-pack` and Chrome installed. If `wasm-pack` is absent: `cargo install wasm-pack`.) If the hash mismatches, that is a real cross-platform determinism bug — investigate before proceeding; do not edit the constant to match.

- [ ] **Step 4: Commit**

```bash
git add crates/nies-web/Cargo.toml crates/nies-web/tests/determinism_wasm.rs
git commit -m 'test(web): cross-platform golden-hash gate (wasm-bindgen-test)

Asserts the demo_ntsc index framebuffer hashes to the same pinned
constant under wasm32 as natively — the "on WASM" half of the M3 gate
(spec §6.2).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 15: CI — run the wasm test + record bundle size

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Extend the `wasm` job**

In `.github/workflows/ci.yml`, in the `wasm` job, after `targets: wasm32-unknown-unknown` is set up and before `trunk build --release`, add a wasm-pack install + test step; keep the existing size-report step. The `wasm` job's `steps` become:

```yaml
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: 1.94.0
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - name: Install wasm-pack
        run: cargo install wasm-pack --version 0.13.1 --locked
      - name: Install trunk
        run: cargo install trunk --version 0.21.14 --locked
      - name: WASM determinism test (headless Chrome)
        run: wasm-pack test --headless --chrome crates/nies-web
      - run: cargo build --target wasm32-unknown-unknown -p nies-web
      - run: trunk build --release
      - name: WASM size report
        run: |
          echo "### WASM bundle size" >> "$GITHUB_STEP_SUMMARY"
          ls -lh dist/*.wasm | tee -a "$GITHUB_STEP_SUMMARY"
          gzip -c dist/*.wasm | wc -c | awk '{print "gzipped bytes: " $1}' | tee -a "$GITHUB_STEP_SUMMARY"
```

(ubuntu-latest ships Chrome, which `wasm-pack test --headless --chrome` uses. No hard size budget yet — M10. Pin `wasm-pack 0.13.1` deliberately, like the other tool pins.)

- [ ] **Step 2: Sanity-check the YAML locally**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo OK`
Expected: `OK` (valid YAML).

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m 'ci(wasm): run determinism test + record bundle size

wasm job now runs wasm-pack test --headless --chrome (the cross-platform
golden gate) and writes raw+gzipped dist/*.wasm sizes to the job summary
for regression tracking. Hard size budget deferred to M10.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

### Task 16: Amend the global design spec

**Files:**
- Modify: `docs/superpowers/specs/2026-05-02-nes-emulator-design.md`

- [ ] **Step 1: Update the §8 M3 gate and §9 palette decision**

In `docs/superpowers/specs/2026-05-02-nes-emulator-design.md`:

1. In the **M3** milestone block (§8), append a line noting the gate is satisfied and pointing at the M3 spec/plan, e.g. after the existing `**Gate:**` paragraph:

```markdown
> **Status (M3 complete):** `Nes` driver added; shared `NesRenderer` (R8Uint
> index texture → FBX Smooth palette-LUT WGSL shader → integer-scaled blit)
> in `nies-ui`; both binaries render. Golden hash of the demo_ntsc index
> framebuffer pinned and asserted natively and via `wasm-bindgen-test`. See
> [`2026-05-30-m3-frontend-rendering-design.md`](2026-05-30-m3-frontend-rendering-design.md).
```

2. In **§9 Open questions**, update the **Default palette** bullet to record the decision:

```markdown
- **Default palette**: **FBX Smooth**, chosen at M3 (vendored 64-entry
  `smooth_fbx.pal`). Nostalgia (FBX) remains a future selectable option (M10).
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-05-02-nes-emulator-design.md
git commit -m 'docs(spec): record M3 completion + FBX Smooth palette decision

Mark the M3 gate satisfied and resolve the §9 default-palette open
question to FBX Smooth.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>'
```

- [ ] **Step 3: Full-suite verification**

Run:
```bash
cargo fmt --all -- --check
cargo build --workspace --exclude nies-web
cargo test --workspace --exclude nies-web
cargo clippy --workspace --exclude nies-web --all-targets -- -D warnings
cargo build --target wasm32-unknown-unknown -p nies-web
cargo clippy -p nies-web --target wasm32-unknown-unknown --all-targets -- -D warnings
```
Expected: all clean/green. (The wasm-bindgen-test runs under `wasm-pack`, not `cargo test`; run `wasm-pack test --headless --chrome crates/nies-web` once more to confirm.)

--- CHECKPOINT (end of Unit 5 — M3 complete) ---
Native + WASM both render the demo; golden hash gates both targets; CI extended. Use `superpowers:finishing-a-development-branch` to open the M3 PR.

---

## Self-review notes

- **Spec coverage:** every spec section in the §-mapping table maps to ≥1 task. §4.4 (WebGL2 fallback) is documented in Tasks 9–10 and exercised in Task 13.
- **Golden constant:** `GOLDEN_FB_HASH` is recorded in Task 5 Step 1 and reused verbatim in Task 14; both sites cross-reference each other. The `0xKEEP_THE_RECORDED_VALUE` token is an intentional fill-from-run marker, not a placeholder requirement — Task 5 Step 1 produces the exact value.
- **Type consistency:** `Nes::{from_rom_bytes, run_frame, frame, frame_count, reset}`, `NesRenderer::{new, upload_frame, render}`, `integer_scale → Viewport`, `fbx_smooth() → [[u8;3];64]`, `demo_rom_bytes() → &[u8]` are used identically across all referencing tasks.
- **wgpu 29 surface area:** descriptor field/struct names (`TexelCopy*`, `entry_point: Some(_)`, `depth_slice`, `multiview_mask`, `cache: None`) are flagged in-task to cross-check against the known-good `nies-app` bootstrap, since wgpu renamed several across recent minors.
```
