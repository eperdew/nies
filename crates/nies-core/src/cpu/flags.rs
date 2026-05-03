//! 6502 status register bit positions.

pub const FLAG_C: u8 = 0b0000_0001; // Carry
pub const FLAG_Z: u8 = 0b0000_0010; // Zero
pub const FLAG_I: u8 = 0b0000_0100; // Interrupt-disable
pub const FLAG_D: u8 = 0b0000_1000; // Decimal (settable; BCD not implemented on NES)
pub const FLAG_B: u8 = 0b0001_0000; // Break (only set in pushed P from BRK/PHP)
pub const FLAG_U: u8 = 0b0010_0000; // Unused (always reads as 1 in pushed P)
pub const FLAG_V: u8 = 0b0100_0000; // Overflow
pub const FLAG_N: u8 = 0b1000_0000; // Negative
