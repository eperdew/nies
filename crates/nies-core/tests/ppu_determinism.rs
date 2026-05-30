//! M2 PPU determinism check: nmi_sync/demo_ntsc.nes produces a
//! self-deterministic framebuffer. Per the M2 design spec §4.3, this
//! is *not* a correctness check — that lands at M3 when there's a
//! renderer to debug a hash mismatch.

use nies_core::bus::Bus;
use nies_core::cartridge::Cartridge;
use nies_core::cpu::Cpu;
use nies_core::mapper::MapperKind;
use std::hash::{DefaultHasher, Hash, Hasher};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/roms");
const N_FRAMES: u64 = 200;

#[test]
fn nmi_sync_demo_ntsc_is_deterministic() {
    let path = format!("{ROOT}/nmi_sync/demo_ntsc.nes");
    let h1 = run_and_hash(&path, N_FRAMES);
    let h2 = run_and_hash(&path, N_FRAMES);
    assert_eq!(
        h1, h2,
        "framebuffer hash differs across two identical {N_FRAMES}-frame runs"
    );
}

fn run_and_hash(path: &str, n_frames: u64) -> u64 {
    let bytes = std::fs::read(path).expect("read rom");
    let cart = Cartridge::from_bytes(&bytes).expect("parse rom");
    let mapper = MapperKind::from_cartridge(&cart).expect("build mapper");
    let mut bus = Bus::new(mapper);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let target_frame = bus.ppu.state.frames + n_frames;
    while bus.ppu.state.frames < target_frame {
        cpu.step(&mut bus);
    }

    let mut h = DefaultHasher::new();
    bus.ppu.frame().hash(&mut h);
    h.finish()
}
