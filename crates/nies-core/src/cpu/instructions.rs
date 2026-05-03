//! 6502 opcode dispatch and instruction handlers.
//!
//! Per spec §3.4 we use a function-table dispatch on opcode byte. To make
//! the production Cpu and the SingleStepTests harness share code, we
//! parameterize handlers on `B: BusLike`. Generic function pointers can't
//! live in a static table, so we dispatch via a `match` switch instead.
//! The match is monomorphized per concrete bus type at compile time.

use crate::bus::BusLike;
use crate::cpu::Cpu;
use crate::cpu::addressing as addr;
use crate::cpu::flags;

pub fn dispatch<B: BusLike>(opcode: u8, cpu: &mut Cpu, bus: &mut B) {
    match opcode {
        // LDA
        0xA9 => {
            let v = addr::fetch_byte(cpu, bus);
            ld_a(cpu, v);
        }
        0xA5 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            ld_a(cpu, v);
        }
        0xB5 => {
            let a = addr::zp_x(cpu, bus);
            let v = bus.read(a);
            ld_a(cpu, v);
        }
        0xAD => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            ld_a(cpu, v);
        }
        0xBD => {
            let a = addr::abs_x_read(cpu, bus);
            let v = bus.read(a);
            ld_a(cpu, v);
        }
        0xB9 => {
            let a = addr::abs_y_read(cpu, bus);
            let v = bus.read(a);
            ld_a(cpu, v);
        }
        0xA1 => {
            let a = addr::ind_x(cpu, bus);
            let v = bus.read(a);
            ld_a(cpu, v);
        }
        0xB1 => {
            let a = addr::ind_y_read(cpu, bus);
            let v = bus.read(a);
            ld_a(cpu, v);
        }
        // LDX
        0xA2 => {
            let v = addr::fetch_byte(cpu, bus);
            ld_x(cpu, v);
        }
        0xA6 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            ld_x(cpu, v);
        }
        0xB6 => {
            let a = addr::zp_y(cpu, bus);
            let v = bus.read(a);
            ld_x(cpu, v);
        }
        0xAE => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            ld_x(cpu, v);
        }
        0xBE => {
            let a = addr::abs_y_read(cpu, bus);
            let v = bus.read(a);
            ld_x(cpu, v);
        }
        // LDY
        0xA0 => {
            let v = addr::fetch_byte(cpu, bus);
            ld_y(cpu, v);
        }
        0xA4 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            ld_y(cpu, v);
        }
        0xB4 => {
            let a = addr::zp_x(cpu, bus);
            let v = bus.read(a);
            ld_y(cpu, v);
        }
        0xAC => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            ld_y(cpu, v);
        }
        0xBC => {
            let a = addr::abs_x_read(cpu, bus);
            let v = bus.read(a);
            ld_y(cpu, v);
        }
        // STA
        0x85 => {
            let a = addr::zp(cpu, bus);
            bus.write(a, cpu.a);
        }
        0x95 => {
            let a = addr::zp_x(cpu, bus);
            bus.write(a, cpu.a);
        }
        0x8D => {
            let a = addr::abs(cpu, bus);
            bus.write(a, cpu.a);
        }
        0x9D => {
            let a = addr::abs_x_rmw(cpu, bus);
            bus.write(a, cpu.a);
        }
        0x99 => {
            let a = addr::abs_y_rmw(cpu, bus);
            bus.write(a, cpu.a);
        }
        0x81 => {
            let a = addr::ind_x(cpu, bus);
            bus.write(a, cpu.a);
        }
        0x91 => {
            let a = addr::ind_y_rmw(cpu, bus);
            bus.write(a, cpu.a);
        }
        // STX
        0x86 => {
            let a = addr::zp(cpu, bus);
            bus.write(a, cpu.x);
        }
        0x96 => {
            let a = addr::zp_y(cpu, bus);
            bus.write(a, cpu.x);
        }
        0x8E => {
            let a = addr::abs(cpu, bus);
            bus.write(a, cpu.x);
        }
        // STY
        0x84 => {
            let a = addr::zp(cpu, bus);
            bus.write(a, cpu.y);
        }
        0x94 => {
            let a = addr::zp_x(cpu, bus);
            bus.write(a, cpu.y);
        }
        0x8C => {
            let a = addr::abs(cpu, bus);
            bus.write(a, cpu.y);
        }
        // AND
        0x29 => {
            let v = addr::fetch_byte(cpu, bus);
            and_a(cpu, v);
        }
        0x25 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            and_a(cpu, v);
        }
        0x35 => {
            let a = addr::zp_x(cpu, bus);
            let v = bus.read(a);
            and_a(cpu, v);
        }
        0x2D => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            and_a(cpu, v);
        }
        0x3D => {
            let a = addr::abs_x_read(cpu, bus);
            let v = bus.read(a);
            and_a(cpu, v);
        }
        0x39 => {
            let a = addr::abs_y_read(cpu, bus);
            let v = bus.read(a);
            and_a(cpu, v);
        }
        0x21 => {
            let a = addr::ind_x(cpu, bus);
            let v = bus.read(a);
            and_a(cpu, v);
        }
        0x31 => {
            let a = addr::ind_y_read(cpu, bus);
            let v = bus.read(a);
            and_a(cpu, v);
        }
        // ORA
        0x09 => {
            let v = addr::fetch_byte(cpu, bus);
            ora_a(cpu, v);
        }
        0x05 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            ora_a(cpu, v);
        }
        0x15 => {
            let a = addr::zp_x(cpu, bus);
            let v = bus.read(a);
            ora_a(cpu, v);
        }
        0x0D => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            ora_a(cpu, v);
        }
        0x1D => {
            let a = addr::abs_x_read(cpu, bus);
            let v = bus.read(a);
            ora_a(cpu, v);
        }
        0x19 => {
            let a = addr::abs_y_read(cpu, bus);
            let v = bus.read(a);
            ora_a(cpu, v);
        }
        0x01 => {
            let a = addr::ind_x(cpu, bus);
            let v = bus.read(a);
            ora_a(cpu, v);
        }
        0x11 => {
            let a = addr::ind_y_read(cpu, bus);
            let v = bus.read(a);
            ora_a(cpu, v);
        }
        // EOR
        0x49 => {
            let v = addr::fetch_byte(cpu, bus);
            eor_a(cpu, v);
        }
        0x45 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            eor_a(cpu, v);
        }
        0x55 => {
            let a = addr::zp_x(cpu, bus);
            let v = bus.read(a);
            eor_a(cpu, v);
        }
        0x4D => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            eor_a(cpu, v);
        }
        0x5D => {
            let a = addr::abs_x_read(cpu, bus);
            let v = bus.read(a);
            eor_a(cpu, v);
        }
        0x59 => {
            let a = addr::abs_y_read(cpu, bus);
            let v = bus.read(a);
            eor_a(cpu, v);
        }
        0x41 => {
            let a = addr::ind_x(cpu, bus);
            let v = bus.read(a);
            eor_a(cpu, v);
        }
        0x51 => {
            let a = addr::ind_y_read(cpu, bus);
            let v = bus.read(a);
            eor_a(cpu, v);
        }
        // ADC
        0x69 => {
            let v = addr::fetch_byte(cpu, bus);
            adc_core(cpu, v);
        }
        0x65 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v);
        }
        0x75 => {
            let a = addr::zp_x(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v);
        }
        0x6D => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v);
        }
        0x7D => {
            let a = addr::abs_x_read(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v);
        }
        0x79 => {
            let a = addr::abs_y_read(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v);
        }
        0x61 => {
            let a = addr::ind_x(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v);
        }
        0x71 => {
            let a = addr::ind_y_read(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v);
        }
        // SBC
        0xE9 => {
            let v = addr::fetch_byte(cpu, bus);
            adc_core(cpu, v ^ 0xFF);
        }
        0xE5 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v ^ 0xFF);
        }
        0xF5 => {
            let a = addr::zp_x(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v ^ 0xFF);
        }
        0xED => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v ^ 0xFF);
        }
        0xFD => {
            let a = addr::abs_x_read(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v ^ 0xFF);
        }
        0xF9 => {
            let a = addr::abs_y_read(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v ^ 0xFF);
        }
        0xE1 => {
            let a = addr::ind_x(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v ^ 0xFF);
        }
        0xF1 => {
            let a = addr::ind_y_read(cpu, bus);
            let v = bus.read(a);
            adc_core(cpu, v ^ 0xFF);
        }
        // CMP
        0xC9 => {
            let v = addr::fetch_byte(cpu, bus);
            compare(cpu, cpu.a, v);
        }
        0xC5 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.a, v);
        }
        0xD5 => {
            let a = addr::zp_x(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.a, v);
        }
        0xCD => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.a, v);
        }
        0xDD => {
            let a = addr::abs_x_read(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.a, v);
        }
        0xD9 => {
            let a = addr::abs_y_read(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.a, v);
        }
        0xC1 => {
            let a = addr::ind_x(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.a, v);
        }
        0xD1 => {
            let a = addr::ind_y_read(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.a, v);
        }
        // CPX
        0xE0 => {
            let v = addr::fetch_byte(cpu, bus);
            compare(cpu, cpu.x, v);
        }
        0xE4 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.x, v);
        }
        0xEC => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.x, v);
        }
        // CPY
        0xC0 => {
            let v = addr::fetch_byte(cpu, bus);
            compare(cpu, cpu.y, v);
        }
        0xC4 => {
            let a = addr::zp(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.y, v);
        }
        0xCC => {
            let a = addr::abs(cpu, bus);
            let v = bus.read(a);
            compare(cpu, cpu.y, v);
        }
        _ => panic!(
            "CPU executed unimplemented opcode ${opcode:02X} at PC=${:04X}",
            cpu.pc.wrapping_sub(1)
        ),
    }
}

fn ld_a(cpu: &mut Cpu, v: u8) {
    cpu.a = v;
    set_nz(cpu, v);
}
fn ld_x(cpu: &mut Cpu, v: u8) {
    cpu.x = v;
    set_nz(cpu, v);
}
fn ld_y(cpu: &mut Cpu, v: u8) {
    cpu.y = v;
    set_nz(cpu, v);
}

fn and_a(cpu: &mut Cpu, v: u8) {
    cpu.a &= v;
    set_nz(cpu, cpu.a);
}
fn ora_a(cpu: &mut Cpu, v: u8) {
    cpu.a |= v;
    set_nz(cpu, cpu.a);
}
fn eor_a(cpu: &mut Cpu, v: u8) {
    cpu.a ^= v;
    set_nz(cpu, cpu.a);
}

/// ADC core: A := A + operand + C, setting C/V/N/Z. SBC variants call this
/// with `operand` bitwise-inverted. NES 6502 ignores the D flag.
fn adc_core(cpu: &mut Cpu, operand: u8) {
    let a = cpu.a;
    let c_in = cpu.p & flags::FLAG_C;
    let sum = (a as u16) + (operand as u16) + (c_in as u16);
    let result = sum as u8;
    let carry = sum > 0xFF;
    let overflow = ((a ^ result) & (operand ^ result) & 0x80) != 0;
    cpu.p &= !(flags::FLAG_C | flags::FLAG_V);
    if carry {
        cpu.p |= flags::FLAG_C;
    }
    if overflow {
        cpu.p |= flags::FLAG_V;
    }
    cpu.a = result;
    set_nz(cpu, result);
}

/// CMP/CPX/CPY: register - operand. Sets N/Z from low byte of difference,
/// C if register >= operand. Register itself is not modified.
fn compare(cpu: &mut Cpu, reg: u8, operand: u8) {
    let result = reg.wrapping_sub(operand);
    cpu.p &= !flags::FLAG_C;
    if reg >= operand {
        cpu.p |= flags::FLAG_C;
    }
    set_nz(cpu, result);
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
