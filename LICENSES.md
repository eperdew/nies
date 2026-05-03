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

## Third-party crates

Third-party Rust crate licenses are surfaced via `cargo about`
(or equivalent) prior to v1 release. None are included verbatim here.
