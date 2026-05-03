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

## Third-party crates

Third-party Rust crate licenses are surfaced via `cargo about`
(or equivalent) prior to v1 release. None are included verbatim here.
