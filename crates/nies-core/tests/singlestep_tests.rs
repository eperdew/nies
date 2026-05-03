//! Per-opcode integration tests driven by the SingleStepTests/65x02 corpus.

mod common;

use common::{TestCase, load_opcode_cases};
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
        return Err(format!(
            "PC: expected {:04X}, got {:04X}",
            case.r#final.pc, cpu.pc
        ));
    }
    if cpu.sp != case.r#final.s {
        return Err(format!(
            "S: expected {:02X}, got {:02X}",
            case.r#final.s, cpu.sp
        ));
    }
    if cpu.a != case.r#final.a {
        return Err(format!(
            "A: expected {:02X}, got {:02X}",
            case.r#final.a, cpu.a
        ));
    }
    if cpu.x != case.r#final.x {
        return Err(format!(
            "X: expected {:02X}, got {:02X}",
            case.r#final.x, cpu.x
        ));
    }
    if cpu.y != case.r#final.y {
        return Err(format!(
            "Y: expected {:02X}, got {:02X}",
            case.r#final.y, cpu.y
        ));
    }
    if cpu.p != case.r#final.p {
        return Err(format!(
            "P: expected {:02X}, got {:02X}",
            case.r#final.p, cpu.p
        ));
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
                expected.0, expected.1, expected.2, actual.0, actual.1, actual.2
            ));
        }
    }
    Ok(())
}

#[test]
fn opcode_a9_lda_imm() {
    run_opcode_tests(0xA9);
}

// Load/Store family
#[test]
fn opcode_a5_lda_zp() {
    run_opcode_tests(0xA5);
}
#[test]
fn opcode_b5_lda_zpx() {
    run_opcode_tests(0xB5);
}
#[test]
fn opcode_ad_lda_abs() {
    run_opcode_tests(0xAD);
}
#[test]
fn opcode_bd_lda_absx() {
    run_opcode_tests(0xBD);
}
#[test]
fn opcode_b9_lda_absy() {
    run_opcode_tests(0xB9);
}
#[test]
fn opcode_a1_lda_indx() {
    run_opcode_tests(0xA1);
}
#[test]
fn opcode_b1_lda_indy() {
    run_opcode_tests(0xB1);
}

#[test]
fn opcode_a2_ldx_imm() {
    run_opcode_tests(0xA2);
}
#[test]
fn opcode_a6_ldx_zp() {
    run_opcode_tests(0xA6);
}
#[test]
fn opcode_b6_ldx_zpy() {
    run_opcode_tests(0xB6);
}
#[test]
fn opcode_ae_ldx_abs() {
    run_opcode_tests(0xAE);
}
#[test]
fn opcode_be_ldx_absy() {
    run_opcode_tests(0xBE);
}

#[test]
fn opcode_a0_ldy_imm() {
    run_opcode_tests(0xA0);
}
#[test]
fn opcode_a4_ldy_zp() {
    run_opcode_tests(0xA4);
}
#[test]
fn opcode_b4_ldy_zpx() {
    run_opcode_tests(0xB4);
}
#[test]
fn opcode_ac_ldy_abs() {
    run_opcode_tests(0xAC);
}
#[test]
fn opcode_bc_ldy_absx() {
    run_opcode_tests(0xBC);
}

#[test]
fn opcode_85_sta_zp() {
    run_opcode_tests(0x85);
}
#[test]
fn opcode_95_sta_zpx() {
    run_opcode_tests(0x95);
}
#[test]
fn opcode_8d_sta_abs() {
    run_opcode_tests(0x8D);
}
#[test]
fn opcode_9d_sta_absx() {
    run_opcode_tests(0x9D);
}
#[test]
fn opcode_99_sta_absy() {
    run_opcode_tests(0x99);
}
#[test]
fn opcode_81_sta_indx() {
    run_opcode_tests(0x81);
}
#[test]
fn opcode_91_sta_indy() {
    run_opcode_tests(0x91);
}

#[test]
fn opcode_86_stx_zp() {
    run_opcode_tests(0x86);
}
#[test]
fn opcode_96_stx_zpy() {
    run_opcode_tests(0x96);
}
#[test]
fn opcode_8e_stx_abs() {
    run_opcode_tests(0x8E);
}

#[test]
fn opcode_84_sty_zp() {
    run_opcode_tests(0x84);
}
#[test]
fn opcode_94_sty_zpx() {
    run_opcode_tests(0x94);
}
#[test]
fn opcode_8c_sty_abs() {
    run_opcode_tests(0x8C);
}

// Logic family (AND, ORA, EOR)
#[test]
fn opcode_29_and_imm() {
    run_opcode_tests(0x29);
}
#[test]
fn opcode_25_and_zp() {
    run_opcode_tests(0x25);
}
#[test]
fn opcode_35_and_zpx() {
    run_opcode_tests(0x35);
}
#[test]
fn opcode_2d_and_abs() {
    run_opcode_tests(0x2D);
}
#[test]
fn opcode_3d_and_absx() {
    run_opcode_tests(0x3D);
}
#[test]
fn opcode_39_and_absy() {
    run_opcode_tests(0x39);
}
#[test]
fn opcode_21_and_indx() {
    run_opcode_tests(0x21);
}
#[test]
fn opcode_31_and_indy() {
    run_opcode_tests(0x31);
}

#[test]
fn opcode_09_ora_imm() {
    run_opcode_tests(0x09);
}
#[test]
fn opcode_05_ora_zp() {
    run_opcode_tests(0x05);
}
#[test]
fn opcode_15_ora_zpx() {
    run_opcode_tests(0x15);
}
#[test]
fn opcode_0d_ora_abs() {
    run_opcode_tests(0x0D);
}
#[test]
fn opcode_1d_ora_absx() {
    run_opcode_tests(0x1D);
}
#[test]
fn opcode_19_ora_absy() {
    run_opcode_tests(0x19);
}
#[test]
fn opcode_01_ora_indx() {
    run_opcode_tests(0x01);
}
#[test]
fn opcode_11_ora_indy() {
    run_opcode_tests(0x11);
}

#[test]
fn opcode_49_eor_imm() {
    run_opcode_tests(0x49);
}
#[test]
fn opcode_45_eor_zp() {
    run_opcode_tests(0x45);
}
#[test]
fn opcode_55_eor_zpx() {
    run_opcode_tests(0x55);
}
#[test]
fn opcode_4d_eor_abs() {
    run_opcode_tests(0x4D);
}
#[test]
fn opcode_5d_eor_absx() {
    run_opcode_tests(0x5D);
}
#[test]
fn opcode_59_eor_absy() {
    run_opcode_tests(0x59);
}
#[test]
fn opcode_41_eor_indx() {
    run_opcode_tests(0x41);
}
#[test]
fn opcode_51_eor_indy() {
    run_opcode_tests(0x51);
}

// Arithmetic family (ADC, SBC)
#[test]
fn opcode_69_adc_imm() {
    run_opcode_tests(0x69);
}
#[test]
fn opcode_65_adc_zp() {
    run_opcode_tests(0x65);
}
#[test]
fn opcode_75_adc_zpx() {
    run_opcode_tests(0x75);
}
#[test]
fn opcode_6d_adc_abs() {
    run_opcode_tests(0x6D);
}
#[test]
fn opcode_7d_adc_absx() {
    run_opcode_tests(0x7D);
}
#[test]
fn opcode_79_adc_absy() {
    run_opcode_tests(0x79);
}
#[test]
fn opcode_61_adc_indx() {
    run_opcode_tests(0x61);
}
#[test]
fn opcode_71_adc_indy() {
    run_opcode_tests(0x71);
}

#[test]
fn opcode_e9_sbc_imm() {
    run_opcode_tests(0xE9);
}
#[test]
fn opcode_e5_sbc_zp() {
    run_opcode_tests(0xE5);
}
#[test]
fn opcode_f5_sbc_zpx() {
    run_opcode_tests(0xF5);
}
#[test]
fn opcode_ed_sbc_abs() {
    run_opcode_tests(0xED);
}
#[test]
fn opcode_fd_sbc_absx() {
    run_opcode_tests(0xFD);
}
#[test]
fn opcode_f9_sbc_absy() {
    run_opcode_tests(0xF9);
}
#[test]
fn opcode_e1_sbc_indx() {
    run_opcode_tests(0xE1);
}
#[test]
fn opcode_f1_sbc_indy() {
    run_opcode_tests(0xF1);
}

// Compare family (CMP, CPX, CPY)
#[test]
fn opcode_c9_cmp_imm() {
    run_opcode_tests(0xC9);
}
#[test]
fn opcode_c5_cmp_zp() {
    run_opcode_tests(0xC5);
}
#[test]
fn opcode_d5_cmp_zpx() {
    run_opcode_tests(0xD5);
}
#[test]
fn opcode_cd_cmp_abs() {
    run_opcode_tests(0xCD);
}
#[test]
fn opcode_dd_cmp_absx() {
    run_opcode_tests(0xDD);
}
#[test]
fn opcode_d9_cmp_absy() {
    run_opcode_tests(0xD9);
}
#[test]
fn opcode_c1_cmp_indx() {
    run_opcode_tests(0xC1);
}
#[test]
fn opcode_d1_cmp_indy() {
    run_opcode_tests(0xD1);
}

#[test]
fn opcode_e0_cpx_imm() {
    run_opcode_tests(0xE0);
}
#[test]
fn opcode_e4_cpx_zp() {
    run_opcode_tests(0xE4);
}
#[test]
fn opcode_ec_cpx_abs() {
    run_opcode_tests(0xEC);
}

#[test]
fn opcode_c0_cpy_imm() {
    run_opcode_tests(0xC0);
}
#[test]
fn opcode_c4_cpy_zp() {
    run_opcode_tests(0xC4);
}
#[test]
fn opcode_cc_cpy_abs() {
    run_opcode_tests(0xCC);
}

// BIT
#[test]
fn opcode_24_bit_zp() {
    run_opcode_tests(0x24);
}
#[test]
fn opcode_2c_bit_abs() {
    run_opcode_tests(0x2C);
}

// Shift/Rotate (ASL, LSR, ROL, ROR)
#[test]
fn opcode_0a_asl_acc() {
    run_opcode_tests(0x0A);
}
#[test]
fn opcode_06_asl_zp() {
    run_opcode_tests(0x06);
}
#[test]
fn opcode_16_asl_zpx() {
    run_opcode_tests(0x16);
}
#[test]
fn opcode_0e_asl_abs() {
    run_opcode_tests(0x0E);
}
#[test]
fn opcode_1e_asl_absx() {
    run_opcode_tests(0x1E);
}

#[test]
fn opcode_4a_lsr_acc() {
    run_opcode_tests(0x4A);
}
#[test]
fn opcode_46_lsr_zp() {
    run_opcode_tests(0x46);
}
#[test]
fn opcode_56_lsr_zpx() {
    run_opcode_tests(0x56);
}
#[test]
fn opcode_4e_lsr_abs() {
    run_opcode_tests(0x4E);
}
#[test]
fn opcode_5e_lsr_absx() {
    run_opcode_tests(0x5E);
}

#[test]
fn opcode_2a_rol_acc() {
    run_opcode_tests(0x2A);
}
#[test]
fn opcode_26_rol_zp() {
    run_opcode_tests(0x26);
}
#[test]
fn opcode_36_rol_zpx() {
    run_opcode_tests(0x36);
}
#[test]
fn opcode_2e_rol_abs() {
    run_opcode_tests(0x2E);
}
#[test]
fn opcode_3e_rol_absx() {
    run_opcode_tests(0x3E);
}

#[test]
fn opcode_6a_ror_acc() {
    run_opcode_tests(0x6A);
}
#[test]
fn opcode_66_ror_zp() {
    run_opcode_tests(0x66);
}
#[test]
fn opcode_76_ror_zpx() {
    run_opcode_tests(0x76);
}
#[test]
fn opcode_6e_ror_abs() {
    run_opcode_tests(0x6E);
}
#[test]
fn opcode_7e_ror_absx() {
    run_opcode_tests(0x7E);
}

// Increment/Decrement (INC, DEC, INX, INY, DEX, DEY)
#[test]
fn opcode_e6_inc_zp() {
    run_opcode_tests(0xE6);
}
#[test]
fn opcode_f6_inc_zpx() {
    run_opcode_tests(0xF6);
}
#[test]
fn opcode_ee_inc_abs() {
    run_opcode_tests(0xEE);
}
#[test]
fn opcode_fe_inc_absx() {
    run_opcode_tests(0xFE);
}

#[test]
fn opcode_c6_dec_zp() {
    run_opcode_tests(0xC6);
}
#[test]
fn opcode_d6_dec_zpx() {
    run_opcode_tests(0xD6);
}
#[test]
fn opcode_ce_dec_abs() {
    run_opcode_tests(0xCE);
}
#[test]
fn opcode_de_dec_absx() {
    run_opcode_tests(0xDE);
}

#[test]
fn opcode_e8_inx() {
    run_opcode_tests(0xE8);
}
#[test]
fn opcode_c8_iny() {
    run_opcode_tests(0xC8);
}
#[test]
fn opcode_ca_dex() {
    run_opcode_tests(0xCA);
}
#[test]
fn opcode_88_dey() {
    run_opcode_tests(0x88);
}

// Branches
#[test]
fn opcode_90_bcc() {
    run_opcode_tests(0x90);
}
#[test]
fn opcode_b0_bcs() {
    run_opcode_tests(0xB0);
}
#[test]
fn opcode_f0_beq() {
    run_opcode_tests(0xF0);
}
#[test]
fn opcode_d0_bne() {
    run_opcode_tests(0xD0);
}
#[test]
fn opcode_30_bmi() {
    run_opcode_tests(0x30);
}
#[test]
fn opcode_10_bpl() {
    run_opcode_tests(0x10);
}
#[test]
fn opcode_50_bvc() {
    run_opcode_tests(0x50);
}
#[test]
fn opcode_70_bvs() {
    run_opcode_tests(0x70);
}

// JMP / JSR / RTS
#[test]
fn opcode_4c_jmp_abs() {
    run_opcode_tests(0x4C);
}
#[test]
fn opcode_6c_jmp_ind() {
    run_opcode_tests(0x6C);
}
#[test]
fn opcode_20_jsr() {
    run_opcode_tests(0x20);
}
#[test]
fn opcode_60_rts() {
    run_opcode_tests(0x60);
}

// Stack ops
#[test]
fn opcode_48_pha() {
    run_opcode_tests(0x48);
}
#[test]
fn opcode_08_php() {
    run_opcode_tests(0x08);
}
#[test]
fn opcode_68_pla() {
    run_opcode_tests(0x68);
}
#[test]
fn opcode_28_plp() {
    run_opcode_tests(0x28);
}

// BRK / RTI
#[test]
fn opcode_00_brk() {
    run_opcode_tests(0x00);
}
#[test]
fn opcode_40_rti() {
    run_opcode_tests(0x40);
}

// Transfer (register-to-register)
#[test]
fn opcode_aa_tax() {
    run_opcode_tests(0xAA);
}
#[test]
fn opcode_a8_tay() {
    run_opcode_tests(0xA8);
}
#[test]
fn opcode_8a_txa() {
    run_opcode_tests(0x8A);
}
#[test]
fn opcode_98_tya() {
    run_opcode_tests(0x98);
}
#[test]
fn opcode_ba_tsx() {
    run_opcode_tests(0xBA);
}
#[test]
fn opcode_9a_txs() {
    run_opcode_tests(0x9A);
}

// Status flag ops
#[test]
fn opcode_18_clc() {
    run_opcode_tests(0x18);
}
#[test]
fn opcode_38_sec() {
    run_opcode_tests(0x38);
}
#[test]
fn opcode_58_cli() {
    run_opcode_tests(0x58);
}
#[test]
fn opcode_78_sei() {
    run_opcode_tests(0x78);
}
#[test]
fn opcode_b8_clv() {
    run_opcode_tests(0xB8);
}
#[test]
fn opcode_d8_cld() {
    run_opcode_tests(0xD8);
}
#[test]
fn opcode_f8_sed() {
    run_opcode_tests(0xF8);
}

// NOPs — official + unofficial variants
// Implied
#[test]
fn opcode_ea_nop() {
    run_opcode_tests(0xEA);
}
#[test]
fn opcode_1a_nop_imp() {
    run_opcode_tests(0x1A);
}
#[test]
fn opcode_3a_nop_imp() {
    run_opcode_tests(0x3A);
}
#[test]
fn opcode_5a_nop_imp() {
    run_opcode_tests(0x5A);
}
#[test]
fn opcode_7a_nop_imp() {
    run_opcode_tests(0x7A);
}
#[test]
fn opcode_da_nop_imp() {
    run_opcode_tests(0xDA);
}
#[test]
fn opcode_fa_nop_imp() {
    run_opcode_tests(0xFA);
}
// Immediate
#[test]
fn opcode_80_nop_imm() {
    run_opcode_tests(0x80);
}
#[test]
fn opcode_82_nop_imm() {
    run_opcode_tests(0x82);
}
#[test]
fn opcode_89_nop_imm() {
    run_opcode_tests(0x89);
}
#[test]
fn opcode_c2_nop_imm() {
    run_opcode_tests(0xC2);
}
#[test]
fn opcode_e2_nop_imm() {
    run_opcode_tests(0xE2);
}
// Zero-page
#[test]
fn opcode_04_nop_zp() {
    run_opcode_tests(0x04);
}
#[test]
fn opcode_44_nop_zp() {
    run_opcode_tests(0x44);
}
#[test]
fn opcode_64_nop_zp() {
    run_opcode_tests(0x64);
}
// Zero-page,X
#[test]
fn opcode_14_nop_zpx() {
    run_opcode_tests(0x14);
}
#[test]
fn opcode_34_nop_zpx() {
    run_opcode_tests(0x34);
}
#[test]
fn opcode_54_nop_zpx() {
    run_opcode_tests(0x54);
}
#[test]
fn opcode_74_nop_zpx() {
    run_opcode_tests(0x74);
}
#[test]
fn opcode_d4_nop_zpx() {
    run_opcode_tests(0xD4);
}
#[test]
fn opcode_f4_nop_zpx() {
    run_opcode_tests(0xF4);
}
// Absolute
#[test]
fn opcode_0c_nop_abs() {
    run_opcode_tests(0x0C);
}
// Absolute,X
#[test]
fn opcode_1c_nop_absx() {
    run_opcode_tests(0x1C);
}
#[test]
fn opcode_3c_nop_absx() {
    run_opcode_tests(0x3C);
}
#[test]
fn opcode_5c_nop_absx() {
    run_opcode_tests(0x5C);
}
#[test]
fn opcode_7c_nop_absx() {
    run_opcode_tests(0x7C);
}
#[test]
fn opcode_dc_nop_absx() {
    run_opcode_tests(0xDC);
}
#[test]
fn opcode_fc_nop_absx() {
    run_opcode_tests(0xFC);
}

// Stable illegals: LAX (LDA + LDX combined load)
#[test]
fn opcode_a3_lax_indx() {
    run_opcode_tests(0xA3);
}
#[test]
fn opcode_a7_lax_zp() {
    run_opcode_tests(0xA7);
}
#[test]
fn opcode_af_lax_abs() {
    run_opcode_tests(0xAF);
}
#[test]
fn opcode_b3_lax_indy() {
    run_opcode_tests(0xB3);
}
#[test]
fn opcode_b7_lax_zpy() {
    run_opcode_tests(0xB7);
}
#[test]
fn opcode_bf_lax_absy() {
    run_opcode_tests(0xBF);
}

// Stable illegals: SAX (store A AND X)
#[test]
fn opcode_83_sax_indx() {
    run_opcode_tests(0x83);
}
#[test]
fn opcode_87_sax_zp() {
    run_opcode_tests(0x87);
}
#[test]
fn opcode_8f_sax_abs() {
    run_opcode_tests(0x8F);
}
#[test]
fn opcode_97_sax_zpy() {
    run_opcode_tests(0x97);
}

// Stable illegals: DCP (DEC + CMP combined RMW)
#[test]
fn opcode_c3_dcp_indx() {
    run_opcode_tests(0xC3);
}
#[test]
fn opcode_c7_dcp_zp() {
    run_opcode_tests(0xC7);
}
#[test]
fn opcode_cf_dcp_abs() {
    run_opcode_tests(0xCF);
}
#[test]
fn opcode_d3_dcp_indy() {
    run_opcode_tests(0xD3);
}
#[test]
fn opcode_d7_dcp_zpx() {
    run_opcode_tests(0xD7);
}
#[test]
fn opcode_db_dcp_absy() {
    run_opcode_tests(0xDB);
}
#[test]
fn opcode_df_dcp_absx() {
    run_opcode_tests(0xDF);
}

// Stable illegals: ISC / ISB (INC + SBC combined RMW)
#[test]
fn opcode_e3_isc_indx() {
    run_opcode_tests(0xE3);
}
#[test]
fn opcode_e7_isc_zp() {
    run_opcode_tests(0xE7);
}
#[test]
fn opcode_ef_isc_abs() {
    run_opcode_tests(0xEF);
}
#[test]
fn opcode_f3_isc_indy() {
    run_opcode_tests(0xF3);
}
#[test]
fn opcode_f7_isc_zpx() {
    run_opcode_tests(0xF7);
}
#[test]
fn opcode_fb_isc_absy() {
    run_opcode_tests(0xFB);
}
#[test]
fn opcode_ff_isc_absx() {
    run_opcode_tests(0xFF);
}

// Stable illegals: SLO (ASL + ORA combined RMW)
#[test]
fn opcode_03_slo_indx() {
    run_opcode_tests(0x03);
}
#[test]
fn opcode_07_slo_zp() {
    run_opcode_tests(0x07);
}
#[test]
fn opcode_0f_slo_abs() {
    run_opcode_tests(0x0F);
}
#[test]
fn opcode_13_slo_indy() {
    run_opcode_tests(0x13);
}
#[test]
fn opcode_17_slo_zpx() {
    run_opcode_tests(0x17);
}
#[test]
fn opcode_1b_slo_absy() {
    run_opcode_tests(0x1B);
}
#[test]
fn opcode_1f_slo_absx() {
    run_opcode_tests(0x1F);
}

// Stable illegals: RLA (ROL + AND combined RMW)
#[test]
fn opcode_23_rla_indx() {
    run_opcode_tests(0x23);
}
#[test]
fn opcode_27_rla_zp() {
    run_opcode_tests(0x27);
}
#[test]
fn opcode_2f_rla_abs() {
    run_opcode_tests(0x2F);
}
#[test]
fn opcode_33_rla_indy() {
    run_opcode_tests(0x33);
}
#[test]
fn opcode_37_rla_zpx() {
    run_opcode_tests(0x37);
}
#[test]
fn opcode_3b_rla_absy() {
    run_opcode_tests(0x3B);
}
#[test]
fn opcode_3f_rla_absx() {
    run_opcode_tests(0x3F);
}

// Stable illegals: SRE (LSR + EOR combined RMW)
#[test]
fn opcode_43_sre_indx() {
    run_opcode_tests(0x43);
}
#[test]
fn opcode_47_sre_zp() {
    run_opcode_tests(0x47);
}
#[test]
fn opcode_4f_sre_abs() {
    run_opcode_tests(0x4F);
}
#[test]
fn opcode_53_sre_indy() {
    run_opcode_tests(0x53);
}
#[test]
fn opcode_57_sre_zpx() {
    run_opcode_tests(0x57);
}
#[test]
fn opcode_5b_sre_absy() {
    run_opcode_tests(0x5B);
}
#[test]
fn opcode_5f_sre_absx() {
    run_opcode_tests(0x5F);
}

// Stable illegals: RRA (ROR + ADC combined RMW)
#[test]
fn opcode_63_rra_indx() {
    run_opcode_tests(0x63);
}
#[test]
fn opcode_67_rra_zp() {
    run_opcode_tests(0x67);
}
#[test]
fn opcode_6f_rra_abs() {
    run_opcode_tests(0x6F);
}
#[test]
fn opcode_73_rra_indy() {
    run_opcode_tests(0x73);
}
#[test]
fn opcode_77_rra_zpx() {
    run_opcode_tests(0x77);
}
#[test]
fn opcode_7b_rra_absy() {
    run_opcode_tests(0x7B);
}
#[test]
fn opcode_7f_rra_absx() {
    run_opcode_tests(0x7F);
}

// Stable illegals: immediate-mode bespoke (ANC, ALR, ARR, AXS, $EB SBC)
#[test]
fn opcode_0b_anc_imm() {
    run_opcode_tests(0x0B);
}
#[test]
fn opcode_2b_anc_imm() {
    run_opcode_tests(0x2B);
}
#[test]
fn opcode_4b_alr_imm() {
    run_opcode_tests(0x4B);
}
#[test]
fn opcode_6b_arr_imm() {
    run_opcode_tests(0x6B);
}
#[test]
fn opcode_cb_axs_imm() {
    run_opcode_tests(0xCB);
}
#[test]
fn opcode_eb_sbc_imm() {
    run_opcode_tests(0xEB);
}

// Unstable illegals: SHX / SHY / SHA / TAS (store with AND-of-high-byte+1)
#[test]
fn opcode_9c_shy_absx() {
    run_opcode_tests(0x9C);
}
#[test]
fn opcode_9e_shx_absy() {
    run_opcode_tests(0x9E);
}
#[test]
fn opcode_93_sha_indy() {
    run_opcode_tests(0x93);
}
#[test]
fn opcode_9f_sha_absy() {
    run_opcode_tests(0x9F);
}
#[test]
fn opcode_9b_tas_absy() {
    run_opcode_tests(0x9B);
}

// Magic-constant illegals: XAA / ANE, LXA / LAX#imm
#[test]
fn opcode_8b_xaa_imm() {
    run_opcode_tests(0x8B);
}
#[test]
fn opcode_ab_lxa_imm() {
    run_opcode_tests(0xAB);
}

// JAM / KIL / HLT illegal opcodes — hang the CPU
#[test]
fn opcode_02_jam() {
    run_opcode_tests(0x02);
}
#[test]
fn opcode_12_jam() {
    run_opcode_tests(0x12);
}
#[test]
fn opcode_22_jam() {
    run_opcode_tests(0x22);
}
#[test]
fn opcode_32_jam() {
    run_opcode_tests(0x32);
}
#[test]
fn opcode_42_jam() {
    run_opcode_tests(0x42);
}
#[test]
fn opcode_52_jam() {
    run_opcode_tests(0x52);
}
#[test]
fn opcode_62_jam() {
    run_opcode_tests(0x62);
}
#[test]
fn opcode_72_jam() {
    run_opcode_tests(0x72);
}
#[test]
fn opcode_92_jam() {
    run_opcode_tests(0x92);
}
#[test]
fn opcode_b2_jam() {
    run_opcode_tests(0xB2);
}
#[test]
fn opcode_d2_jam() {
    run_opcode_tests(0xD2);
}
#[test]
fn opcode_f2_jam() {
    run_opcode_tests(0xF2);
}

// Stable illegal: LAS / LAR (A := X := S := value AND S)
#[test]
fn opcode_bb_las_absy() {
    run_opcode_tests(0xBB);
}

// Note: an earlier revision of this file had a single
// `all_256_opcodes_pass` test that looped over every opcode
// sequentially. It was removed because:
//
//   - Cargo's test runner already parallelizes the 256 per-opcode
//     `#[test] fn opcode_NN_*()` entries above across CPU cores.
//     A single sweep test ran them serially in one thread, taking
//     ~25 s in debug vs ~5-10 s aggregate when the per-opcode
//     tests run in parallel.
//   - "All opcodes pass" is exactly what `cargo test --test
//     singlestep_tests opcode_` already produces, with naming
//     and parallelism for free.
//   - The CI gate is `cargo test`, which runs every `#[test]`
//     including all 256 per-opcode entries; no consolidated test
//     was adding coverage.
