# nies

An NES emulator written in Rust. Targets native macOS / Linux / Windows
and the web (WebAssembly).

## Status

In active development. See
[`docs/superpowers/specs/2026-05-02-nes-emulator-design.md`](docs/superpowers/specs/2026-05-02-nes-emulator-design.md)
for the full design and milestone roadmap.

Current milestone: **M0 — Project Skeleton**. The native binary opens a
window with a sentinel color clear; the WASM build loads in a browser
canvas. No emulation yet.

## Workspace layout

- `crates/nies-core` — emulator backend (CPU, PPU, APU, mapper, save
  state, debugger). No I/O dependencies.
- `crates/nies-ui` — egui panels shared by both frontends.
- `crates/nies-app` — native binary (winit + wgpu).
- `crates/nies-web` — WASM library, bundled with Trunk.

## Build

Native:

```bash
cargo run -p nies-app
```

WASM (requires `trunk` installed via `cargo install trunk --version 0.21.14 --locked`):

```bash
trunk serve --release
# Then open http://127.0.0.1:8080 in a WebGPU-capable browser.
```

## Testing

```bash
cargo test --workspace --exclude nies-web
```

## License

Dual-licensed under MIT or Apache-2.0. See [`LICENSES.md`](LICENSES.md) for details.
