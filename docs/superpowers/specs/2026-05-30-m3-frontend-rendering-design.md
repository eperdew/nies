# M3 — Frontend Rendering Design

**Status:** Approved for implementation planning
**Date:** 2026-05-30
**Author:** Eric Perdew (with Claude Code as co-designer)
**Predecessor:** [`2026-05-02-nes-emulator-design.md`](2026-05-02-nes-emulator-design.md) §5 (Frontend), §8 (M3 milestone), §9 (open questions). Read the global spec first; this document refines M3 only.

## 1. Goal and gate

Put the emulator's palette-index framebuffer on screen at 60 Hz on both the native (`nies-app`) and WASM (`nies-web`) frontends, and lock in a deterministic cross-platform correctness gate.

**Gate (from global spec §8, made concrete here):**

- `nmi_sync/demo_ntsc.nes` run for a fixed frame count produces an index framebuffer whose hash equals a **pinned golden constant**, asserted in a native test **and** a `wasm-bindgen-test` (headless Chrome) — i.e. the core is bit-identical on `x86_64`/`aarch64` native and `wasm32`.
- Both binaries render the demo ROM correctly (manual eyeball — the renderer is the debug aid for any hash mismatch).
- CI records the built WASM bundle size for regression tracking.

This milestone is the first time the emulator core is driven by a real frontend rather than a test loop, so it introduces the top-level driver the rest of v1 builds on.

## 2. Scope

### 2.1 In scope

- A `Nes` top-level driver in `nies-core` (global spec §3.3): owns `Cpu` + `Bus`, exposes `from_rom_bytes` / `run_frame` / `frame` / `frame_count` / `reset`.
- A shared, platform-agnostic `NesRenderer` in `nies-ui`: R8 index texture → WGSL palette-LUT shader → integer-scaled blit. Raw wgpu, **no egui**.
- Both binaries wired to load a ROM, run a frame per redraw, and render it. Native takes an optional CLI path; both embed `demo_ntsc.nes` as a fallback/default.
- Default palette **FBX Smooth**, baked as a 64-entry RGB lookup table and pinned as data.
- Golden-hash test refactored onto `Nes`; new `wasm-bindgen-test` asserting the same constant; CI runs `wasm-pack test` and records bundle size.
- Removal of the duplicated M0 `GpuState` sentinel-clear code from both binaries.

### 2.2 Out of scope (deferred, with milestone)

| Deferred | Milestone |
|---|---|
| egui / menu bar / UI shell | M10 |
| Keyboard/gamepad input, controller registers | M4 |
| Audio + audio-driven frame pacing | M5 |
| File-open dialog (rfd native, `<input type=file>`/gloo-file web) | later ROM-picker step (≤ M6) |
| 8:7 pixel aspect-ratio correction, window-scale settings | M10 |
| NTSC composite filter, CRT shader, scanlines, overscan crop | M14 |
| Runtime palette selection UI | M10 |
| WASM hard size budget assertion | M10 (M3 only *records* the size) |

M3 frame pacing is **vsync-driven** (temporary, per global spec §8); it is replaced by audio-driven pacing at M5.

## 3. Core: the `Nes` driver

New module `crates/nies-core/src/nes.rs`, re-exported from `lib.rs`. Pure logic — it honors the crate's no-I/O contract (global spec §3.1): no `std::fs`, no clocks, no threads.

```rust
pub struct Nes {
    cpu: Cpu,
    bus: Bus,
}

impl Nes {
    /// Parse an iNES/NES 2.0 image, build the mapper + bus, reset the CPU.
    pub fn from_rom_bytes(bytes: &[u8]) -> Result<Self, CartridgeError>;

    /// Step the CPU until the PPU completes one frame
    /// (`bus.ppu.frames()` increments). Runs whole instructions; the
    /// boundary is "first instruction that pushes the frame counter over".
    pub fn run_frame(&mut self);

    /// Current frame's palette-index framebuffer (0..=63 per pixel).
    pub fn frame(&self) -> &[u8; 256 * 240];

    /// Total frames completed since power-on (monotonic).
    pub fn frame_count(&self) -> u64;

    /// Soft reset (re-run the CPU reset sequence). Does not rebuild the cart.
    pub fn reset(&mut self);
}
```

Design notes:

- `from_rom_bytes` wraps the existing `Cartridge::from_bytes` → `MapperKind::from_cartridge` → `Bus::new` → `Cpu::new` + `cpu.reset(&mut bus)` sequence that today lives inline in `ppu_determinism.rs`. Error type is `CartridgeError` (mapper-build failures surface through it; if `MapperKind::from_cartridge` has a distinct error, `from_rom_bytes` maps it into `CartridgeError` or a small wrapping enum — chosen during implementation, but the public signature returns one error type).
- `run_frame` reads `frame_count` before the loop and steps `cpu.step(&mut bus)` until it changes. This is the exact loop in the current determinism test, lifted behind the API.
- `frame` / `frame_count` delegate to `bus.ppu.frame()` / `bus.ppu.frames()`. `Bus`'s `ppu` field stays as-is; `Nes` is a thin owner, not a re-plumbing.
- No rendering, audio, input, or save-state affordances are added here — those arrive in their own milestones. `Nes` is deliberately minimal at M3.

`Cpu` and `Bus` remain public for the existing per-opcode and ROM tests; `Nes` is an additional, higher-level entry point, not a replacement.

## 4. Shared renderer: `NesRenderer` (nies-ui)

`nies-ui` gains its first real code: a platform-agnostic renderer. It owns GPU *resources* but never creates or owns the surface, device, queue, or window — those belong to each binary, which passes references in. This keeps `nies-ui` testable against a headless device and free of winit/web bootstrap concerns. No egui yet (the egui UI shell is M10).

### 4.1 Interface

```rust
pub struct NesRenderer { /* texture + view + sampler, render pipeline,
                            bind group, palette LUT buffer, vertex/index data */ }

impl NesRenderer {
    /// Build pipeline + resources for a given swapchain format.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue,
               target_format: wgpu::TextureFormat) -> Self;

    /// Upload one 256x240 index frame into the GPU texture.
    pub fn upload_frame(&self, queue: &wgpu::Queue, frame: &[u8; 256 * 240]);

    /// Encode a render pass that draws the integer-scaled, centered image
    /// into `view`, letterboxing the rest of `target` (width, height) black.
    pub fn render(&self, encoder: &mut wgpu::CommandEncoder,
                  view: &wgpu::TextureView, target: (u32, u32));
}
```

### 4.2 Pipeline (global spec §5.4)

- **Index texture:** one persistent 256×240 texture, written every frame via `wgpu::Queue::write_texture` (~61 KiB/frame, trivial bandwidth). Preferred format **R8Uint**, sampled as an integer texture in WGSL with the index read directly.
- **Vertex stage:** a fullscreen triangle generated from `vertex_index` (no vertex buffer) — the standard 3-vertex oversized-triangle trick.
- **Fragment stage:** read the pixel's palette index (0..=63), look up RGB in a **64-entry palette LUT** held in a uniform buffer, output RGB(A). Nearest-neighbor; no filtering.
- **Integer scaling:** in `render`, compute `scale = max(1, min(target.0 / 256, target.1 / 240))`, center the 256·scale × 240·scale image via `RenderPass::set_viewport`, and clear the surrounding area to black. No fractional scaling, no PAR at M3.

### 4.3 Palette LUT

- Ship **FBX Smooth** as a `const` 64-entry RGB table (`[[u8; 3]; 64]` or packed `u32`s) in `nies-ui` (e.g. `palette.rs`).
- Uploaded once into the uniform buffer at `new`. The table is **pinned as test data** so the golden-hash test and any future RGB checks have a fixed reference. Runtime palette switching is M10.

### 4.4 WebGL2 / downlevel risk

`nies-web` requests `Limits::downlevel_webgl2_defaults()`, so the renderer must work under the WebGL2 (GLES3) backend, not just native WebGPU. Two specifics to validate during implementation:

- Sampling an **R8Uint** texture (`texture_2d<u32>` / `usampler2D`) and dynamically indexing a uniform array are both GLES3-legal, but backend quirks exist.
- **Fallback:** if a backend rejects the integer-texture path, use **R8Unorm** and reconstruct the index as `round(value * 255.0)` in the shader. The choice is an implementation detail confined to `NesRenderer`; the public interface and the golden hash (which is over the *index* framebuffer, not GPU output) are unaffected.

The renderer carries no `unsafe` and no platform `#[cfg]`; both binaries use the identical type.

## 5. Frontends

Both binaries shrink to: create surface/device/queue (existing bootstrap), build a `Nes` and a `NesRenderer`, then per redraw `run_frame()` → `upload_frame()` → `render()` → present. The M0 magenta `GpuState` clear logic is deleted.

### 5.1 Native (`nies-app`)

- ROM source: optional CLI path — `cargo run -p nies-app -- path/to.nes`. If absent, fall back to an **embedded `demo_ntsc.nes`** via `include_bytes!`. A bad/missing path logs an error and exits (or falls back to the embedded ROM — decided in the plan; either is acceptable, no panic on user input).
- Loop: on `RedrawRequested`, `nes.run_frame()`, `renderer.upload_frame(&queue, nes.frame())`, encode `renderer.render(...)` against the acquired surface view, submit, present, request another redraw. **Pacing is vsync-driven** via the existing `PresentMode::AutoVsync`.
- Window: retitled; default inner size an integer multiple of 256×240 (e.g. 3×). Resize re-letterboxes (handled entirely by `render`'s viewport math; no pipeline rebuild).
- No new dependencies (CLI arg parsed from `std::env::args`; no clap).

### 5.2 Web (`nies-web`)

- ROM source: always the **embedded `demo_ntsc.nes`** (`include_bytes!`); no file picker at M3.
- Same `Nes` + `NesRenderer` loop, inside the existing `EventLoopProxy<UserEvent::GpuReady>` async-bootstrap path. Render begins once GPU is ready, as today.
- No new web dependencies.

### 5.3 Shared ROM asset

`demo_ntsc.nes` already lives at `crates/nies-core/tests/roms/nmi_sync/demo_ntsc.nes`. For embedding it into the binaries and the wasm test without a fragile relative `include_bytes!` across crate boundaries, the plan picks one of: (a) a small committed copy under each frontend (or a shared assets dir) referenced by `include_bytes!`, or (b) a `nies-core` accessor exposing the bytes. Decided in the plan; the constraint is that the **same bytes** feed the native test, the wasm test, and both binaries.

## 6. Testing and CI

### 6.1 Golden-hash test (the gate)

- Rewrite `crates/nies-core/tests/ppu_determinism.rs` to drive `Nes`: `Nes::from_rom_bytes(include_bytes!(demo_ntsc))`, `run_frame()` a fixed N times (carry over the current `N_FRAMES = 200` unless implementation finds a more stable count), hash `nes.frame()`.
- Assert the hash equals a **pinned constant** (computed once during implementation and committed). This upgrades the M2 run-twice self-determinism check into a true golden check, which §7.8 of the global spec deferred to M3.
- Keep using `std::hash::DefaultHasher` (no new deps); the constant is whatever it produces — its only contract is stability across platforms and runs, which holds because the framebuffer is integer-deterministic.

### 6.2 Cross-platform WASM test

- New `wasm-bindgen-test` (in a wasm-test target compiled for `wasm32-unknown-unknown`) that `include_bytes!`s the demo ROM, runs the **same N frames** through `Nes`, and asserts the **same constant**. No file I/O — the ROM is embedded, so it runs under headless Chrome.
- This is the "framebuffer matches golden hash on WASM" half of the gate: it proves `nies-core` is bit-identical on `wasm32`.

### 6.3 CI changes

- Wasm job: add `wasm-pack test --headless --chrome` (or the equivalent already implied by global spec §7.4) for the wasm-bindgen-test.
- After `trunk build --release`, emit the built `dist/*.wasm` size (raw and gzipped) into the job summary/log for regression tracking. **No hard budget assertion at M3** — that lands at M10.
- Native jobs gain the golden-hash assertion automatically (it's a normal `cargo test`).

### 6.4 What is not tested automatically

- GPU-rendered RGB output (driver-dependent; not deterministic across backends — hence the hash is over the index framebuffer, not pixels read back from the GPU).
- Visual correctness of the rendered image (manual eyeball on both binaries).
- Frame pacing under vsync (no display on CI).

## 7. Execution and review style

All M3 units are **agent-dispatched; the user reviews diffs at per-unit checkpoints**. (This milestone departs from M2's user-driven foundational units — the user opted for full dispatch on the GPU layer.) Each dispatched unit still gets a short pre-task background note, and we pause at unit boundaries for the user to read the diff and test output before continuing.

Anticipated unit breakdown (the implementation plan finalizes this):

1. **`Nes` driver** — `nes.rs`, `lib.rs` re-export, refactor `ppu_determinism.rs` onto it (still self-determinism at this point).
2. **`NesRenderer` + shader + palette LUT** — `nies-ui` renderer, WGSL, FBX Smooth table; unit-tested against a headless wgpu device where feasible.
3. **Native frontend** — wire `nies-app` to `Nes` + `NesRenderer`, CLI path + embedded fallback, delete `GpuState` clear.
4. **Web frontend** — wire `nies-web` likewise; embedded ROM.
5. **Golden hash + WASM test + CI** — pin the constant, add the `wasm-bindgen-test`, CI `wasm-pack test` + size recording, and amend the global spec (§8 M3 gate satisfied, §9 palette decision = FBX Smooth).

## 8. Risks and mitigations

- **Integer-texture sampling under WebGL2** — mitigated by the R8Unorm fallback (§4.4); confined to the renderer.
- **Golden constant instability** — the framebuffer is pure integer logic with no clock/RNG/threading (global spec §4.1), so the hash is stable by construction; if a chosen N lands on a visually noisy/transitional frame, pick a steadier N during implementation.
- **`wasm-pack`/headless-Chrome flakiness in CI** — standard `wasm-bindgen-test` setup; if Chrome provisioning is flaky, the native half still gates correctness and the wasm half can be marked required once stable.
- **ROM-asset duplication** — resolved by sourcing the same bytes everywhere (§5.3); avoid divergent copies.

## 9. Open questions resolved by this milestone

- **Default palette (global spec §9):** **FBX Smooth.** Recorded here; the global spec §9 entry is updated in unit 5.
- Other §9 items (project license, audio sample rate, Trunk vs alternatives, tier-3) are untouched by M3.
