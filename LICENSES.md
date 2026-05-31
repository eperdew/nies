# Licenses

## nies (this project)

`nies` is dual-licensed under the MIT License and the Apache License 2.0
at the user's option. Both licenses are reproduced in `LICENSE-MIT` and
`LICENSE-APACHE` (to be added).

The license decision is provisional and may change before v1 ships.
See `docs/superpowers/specs/2026-05-02-nes-emulator-design.md` §9 for context.

## Test ROMs

Test ROMs vendored under `crates/nies-core/tests/roms/` are listed
below with their source, license, and SHA-256 hash. All are
public-domain / freely-redistributable test programs written
specifically for emulator validation; none are commercial game ROMs.

Sources:

- **christopherpow/nes-test-roms** (https://github.com/christopherpow/nes-test-roms)
  is a community-maintained mirror of public-domain test programs from
  several authors. The files we vendor here originate with kevtris
  (`nestest`, redistributed via nesdev) and Shay Green / "blargg"
  (cpu_instrs, instr_misc, instr_timing, cpu_dummy_reads — all
  released under public-domain terms by the author).

| ROM | Source | License | SHA-256 |
|---|---|---|---|
| `nestest/nestest.nes` | kevtris via christopherpow/nes-test-roms `other/nestest.nes` | Public domain | `f67d55fd6b3cf0bad1cc85f1df0d739c65b53e79cecb7fea8f77ec0eadab0004` |
| `nestest/nestest.log` | Nintendulator trace via christopherpow/nes-test-roms `other/nestest.log` | Public domain | `442c4dd5539c7e88b3fd73c7b732a7eadbd22b47c2cd9e58397ef147f64f6f8f` |
| `nestest/nestest.txt` | kevtris notes via christopherpow/nes-test-roms `other/nestest.txt` | Public domain | `8291241ba9a0885b9a604a4685101a1473e22b3aa070bc828e3b8c342d7f71fb` |
| `blargg/cpu_instrs/cpu_instrs.nes` | blargg via christopherpow `instr_test-v5/all_instrs.nes` | Public domain | `353870c157242e3d428ef7387109deaee0d2e158bdb432ab9aae4e657072c785` |
| `blargg/cpu_instrs/01-basics.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `4dd1cdd406bc3f747972e7da314ce8ca89321eb7a836c1ced569ee54ae44a384` |
| `blargg/cpu_instrs/02-implied.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `1c4d4fa130cf6feebc072543a5cd3627ae71063b56b08642bf43e9a6c6f44996` |
| `blargg/cpu_instrs/03-immediate.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `6f7ad8ff31c762c37deaee0f323df03eb94025cf1f3b0343ebe6fe567da0e943` |
| `blargg/cpu_instrs/04-zero_page.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `7a8feada4bb4460250c8f05401e5d728878bbe71956756d0b11d488e57eb12fd` |
| `blargg/cpu_instrs/05-zp_xy.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `767f422dc4e651e331456b207f7c6d60d19329fde0c0827e83591dbd91ae5e23` |
| `blargg/cpu_instrs/06-absolute.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `98df36dc4fcc4f37d9eb0539c71283020776b1e5dc6a6ce58671739a8d6534af` |
| `blargg/cpu_instrs/07-abs_xy.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `9ff58d77d8d384cc918fcd3ed877898c5e7330cd475ed2dafb11cbe80ff32eff` |
| `blargg/cpu_instrs/08-ind_x.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `2ec6f5d4a8caee5d8295cebe563f203c26ea9bc05f1dbc967feb88f5dc4f261f` |
| `blargg/cpu_instrs/09-ind_y.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `0fbc8b228d5daa83a4a083bf87ae3a61b5247ebdd91a6b91c8cf8c42784804ac` |
| `blargg/cpu_instrs/10-branches.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `63ab768e88931db6f7dfcfafe43d5e29ebc3dcb80da8fc7fcda8c930f34aef54` |
| `blargg/cpu_instrs/11-stack.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `c534191fe3ea4c8940944fda98dd58eb42710268d453f97e8e2c4ae7f15f9cdb` |
| `blargg/cpu_instrs/12-jmp_jsr.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `f5b4652690fc04e6b573a2b3b54a29407ad0615d3c264e7cb618b6694b50de55` |
| `blargg/cpu_instrs/13-rts.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `b711d25bc55585c252046a1304a0bc64c13cacce7c96a1bac5c8e91f9fc2597f` |
| `blargg/cpu_instrs/14-rti.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `f084b00605be1840946b53935032581e68abe1bb24479942751cfe46ddfcb280` |
| `blargg/cpu_instrs/15-brk.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `da7ae9a191c4483b540771e15b1f6f18df68f1d1ecd717b59ea8b1ee3596ec3e` |
| `blargg/cpu_instrs/16-special.nes` | blargg via christopherpow `instr_test-v5/rom_singles/` | Public domain | `7d03410b61784e49920901e84b00a4f31a19078391f20005c6fac9036d2190f7` |
| `blargg/instr_misc.nes` | blargg via christopherpow `instr_misc/instr_misc.nes` | Public domain | `b6762e20a285216304dfd2b5e1f192459354b23a5e48b2f5f9fb7cb0dac51243` |
| `blargg/instr_timing/instr_timing.nes` | blargg via christopherpow `instr_timing/instr_timing.nes` | Public domain | `3d1bca14266f1e25b75a34ddd29c9df1ce9c6d990c8663a218f72e7861660fb0` |
| `blargg/instr_timing/1-instr_timing.nes` | blargg via christopherpow `instr_timing/rom_singles/` | Public domain | `e260068839fe3d0402376e97e4ee15f5790ee77c701fd0700bba057527910222` |
| `blargg/instr_timing/2-branch_timing.nes` | blargg via christopherpow `instr_timing/rom_singles/` | Public domain | `0afaa393f375844ab98834c1ecba7fa6d8c44880c8b6e738936d0f04a84c8538` |
| `blargg/cpu_dummy_reads.nes` | blargg via christopherpow `cpu_dummy_reads/cpu_dummy_reads.nes` | Public domain | `db4f91b80c5fbc123e7dcb420fb7fea9b8a18613edf4de7f3d1e3ed95e3117c9` |

## M2 Test ROMs

Vendored at the M2 milestone (PPU implementation). All public-domain
test programs by Shay Green ("blargg"), mirrored at
https://github.com/christopherpow/nes-test-roms.

### blargg ppu_vbl_nmi (10 sub-tests)

| ROM | Source | License | SHA-256 |
|---|---|---|---|
| `blargg/ppu_vbl_nmi/01-vbl_basics.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/01-vbl_basics.nes` | Public domain | `06aea5af4edab4e3141c939cd5ac9936f8758203b25dcaf84ae1a09db49e024a` |
| `blargg/ppu_vbl_nmi/02-vbl_set_time.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes` | Public domain | `dd98856130078844e3aa4bd95a9be8ab501ea84c089f1d8ad49a1b20af4b3a80` |
| `blargg/ppu_vbl_nmi/03-vbl_clear_time.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/03-vbl_clear_time.nes` | Public domain | `787fdaa4dd6c5b6df5f4308fb6d55b57e2c2f69bd5ecdf8ad5c69735db4fcc72` |
| `blargg/ppu_vbl_nmi/04-nmi_control.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/04-nmi_control.nes` | Public domain | `84722c75b896c47c8642f83220230fe14f0a31e55e26ecb83c400e6a26d91b32` |
| `blargg/ppu_vbl_nmi/05-nmi_timing.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/05-nmi_timing.nes` | Public domain | `72e515d689d7404ae5779b8c9c4c7b3563a755a94bd44864516f1b03df044482` |
| `blargg/ppu_vbl_nmi/06-suppression.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/06-suppression.nes` | Public domain | `811dd5997bbf48c2e5687ab06845f17ea76b2be472786596c334137582cc72aa` |
| `blargg/ppu_vbl_nmi/07-nmi_on_timing.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes` | Public domain | `1ed154363660b5775b112ae63ce9bb4e400ebde2afef4d0ac12fc433efda3702` |
| `blargg/ppu_vbl_nmi/08-nmi_off_timing.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes` | Public domain | `1d2a4093091c8e58a7f99d6a3531bbc6346b52cfc59bcb17ca04c1f2376cf2fc` |
| `blargg/ppu_vbl_nmi/09-even_odd_frames.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes` | Public domain | `1ac04283021ddd9294cc74ee709c55e20a350dc4815c15a8a93b3654837e858d` |
| `blargg/ppu_vbl_nmi/10-even_odd_timing.nes` | blargg via christopherpow `ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes` | Public domain | `7217d2d172ce11ad45c4da40c2f22201cf0eb758bc2cd8dd39d2cf0a7d4ca83e` |

### blargg sprite_hit_tests_2005.10.05 (11 sub-tests)

| ROM | Source | License | SHA-256 |
|---|---|---|---|
| `blargg/sprite_hit_tests_2005.10.05/01.basics.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/01.basics.nes` | Public domain | `51819e8e502bd88fe3b7244198a074dbeef2e848f66c587be04b04f1f0d4bb52` |
| `blargg/sprite_hit_tests_2005.10.05/02.alignment.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/02.alignment.nes` | Public domain | `125bbb3ce1e67370f1f4559c2ad3221e52a3e98880b9789400292b5f3a8b39e6` |
| `blargg/sprite_hit_tests_2005.10.05/03.corners.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/03.corners.nes` | Public domain | `9dd57776bc6267fe6183c5521d67cbe3fccc6662ae545eb2c419949bf39644d3` |
| `blargg/sprite_hit_tests_2005.10.05/04.flip.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/04.flip.nes` | Public domain | `5f7142bddb51b7577f93fa22f9f668efebbeea00346d7255089e1863acb9d46a` |
| `blargg/sprite_hit_tests_2005.10.05/05.left_clip.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/05.left_clip.nes` | Public domain | `69b329658c17b953f149c2f0de77eb272089df22c815bd2fd3d6f43206791c13` |
| `blargg/sprite_hit_tests_2005.10.05/06.right_edge.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/06.right_edge.nes` | Public domain | `8e6653fcb869e06873e29e5e4423122ea72ba0bf38f3ba9e39f471420db759a4` |
| `blargg/sprite_hit_tests_2005.10.05/07.screen_bottom.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/07.screen_bottom.nes` | Public domain | `05849956f80267838c5b6556310266b794078a4300841cbb36339fd141905a0b` |
| `blargg/sprite_hit_tests_2005.10.05/08.double_height.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/08.double_height.nes` | Public domain | `127fd966b6b32d6d88a53c5f59d7e938827783c9ad056091f119be1c4ab21c71` |
| `blargg/sprite_hit_tests_2005.10.05/09.timing_basics.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/09.timing_basics.nes` | Public domain | `311698c717e50150edd0b5fd0016c41de686463205c20efb5630d6adb90859fd` |
| `blargg/sprite_hit_tests_2005.10.05/10.timing_order.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/10.timing_order.nes` | Public domain | `0f36bc07bfe51c416e3cc1a5231053572aa6b15aa60e6d2fd0568be49b6dc2e9` |
| `blargg/sprite_hit_tests_2005.10.05/11.edge_timing.nes` | blargg via christopherpow `sprite_hit_tests_2005.10.05/11.edge_timing.nes` | Public domain | `5a7c121f6e76617be88a0a7035c0e402293be5c685c95b97190a8d70835736ab` |

### blargg OAM tests

| ROM | Source | License | SHA-256 |
|---|---|---|---|
| `blargg/oam_read.nes` | blargg via christopherpow `oam_read/oam_read.nes` | Public domain | `f298973dabeb61ca35007445f7a615f77e87703c958c870986af83b1aabde926` |
| `blargg/oam_stress.nes` | blargg via christopherpow `oam_stress/oam_stress.nes` | Public domain | `95882d72a7acabe928fd277e3b3e0372f21ef3d41e36d7d8fb17fc017a356f70` |

### nmi_sync

| ROM | Source | License | SHA-256 |
|---|---|---|---|
| `nmi_sync/demo_ntsc.nes` | blargg via christopherpow `nmi_sync/demo_ntsc.nes` | Redistribution permitted by author | `6f630cf1b37fea5c34d62800855a5384e8bacd578aba3131752f1fb777b4638a` |

## Third-party crates

Third-party Rust crate licenses are surfaced via `cargo about`
(or equivalent) prior to v1 release. None are included verbatim here.

### Palettes

- `crates/nies-ui/assets/smooth_fbx.pal` — "Smooth (FBX)" NES palette by
  FirebrandX. 64 entries × RGB (192 bytes). Freely redistributable; used
  as the default M3 palette LUT.
