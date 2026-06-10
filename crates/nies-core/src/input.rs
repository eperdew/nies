//! Controller state. Polled by CPU $4016/$4017 reads. See spec §5.5.
//!
//! At M1 this is a stub: $4016/$4017 always read 0 and writes are
//! ignored. M4 implements the strobe latch and 8-bit shift register.

/// Button state in hardware bit order — the order the shift register
/// reports them (bit 0 = A): A, B, Select, Start, Up, Down, Left, Right.
///
/// Deliberately faithful: the core never masks simultaneous opposing
/// d-pad directions (real hardware allows Up+Down); any masking policy
/// belongs to frontend settings (M10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Buttons(pub u8);

impl Buttons {
    pub const A: Buttons = Buttons(0x01);
    pub const B: Buttons = Buttons(0x02);
    pub const SELECT: Buttons = Buttons(0x04);
    pub const START: Buttons = Buttons(0x08);
    pub const UP: Buttons = Buttons(0x10);
    pub const DOWN: Buttons = Buttons(0x20);
    pub const LEFT: Buttons = Buttons(0x40);
    pub const RIGHT: Buttons = Buttons(0x80);

    /// This state plus `other`'s buttons.
    pub const fn with(self, other: Buttons) -> Buttons {
        Buttons(self.0 | other.0)
    }

    /// This state minus `other`'s buttons.
    pub const fn without(self, other: Buttons) -> Buttons {
        Buttons(self.0 & !other.0)
    }

    /// True when every button in `other` is pressed in `self`.
    pub const fn contains(self, other: Buttons) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for Buttons {
    type Output = Buttons;
    fn bitor(self, rhs: Buttons) -> Buttons {
        self.with(rhs)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Controller {
    /// Latched button state. Real bit assignments are A/B/Select/Start/
    /// Up/Down/Left/Right (LSB first); M4 fills this in.
    pub buttons: u8,
}

impl Controller {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the next bit out of the shift register. M1 stub: always 0.
    pub fn read(&mut self) -> u8 {
        0
    }

    /// Write to $4016 (strobe). M1 stub: no-op.
    pub fn write_strobe(&mut self, _val: u8) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_read_returns_zero() {
        let mut c = Controller::new();
        assert_eq!(c.read(), 0);
        c.write_strobe(1);
        assert_eq!(c.read(), 0);
    }

    #[test]
    fn buttons_bit_layout_is_hardware_order() {
        assert_eq!(Buttons::A.0, 0x01);
        assert_eq!(Buttons::B.0, 0x02);
        assert_eq!(Buttons::SELECT.0, 0x04);
        assert_eq!(Buttons::START.0, 0x08);
        assert_eq!(Buttons::UP.0, 0x10);
        assert_eq!(Buttons::DOWN.0, 0x20);
        assert_eq!(Buttons::LEFT.0, 0x40);
        assert_eq!(Buttons::RIGHT.0, 0x80);
    }

    #[test]
    fn buttons_set_operations() {
        let ab = Buttons::A.with(Buttons::B);
        assert_eq!(ab, Buttons(0x03));
        assert_eq!(ab, Buttons::A | Buttons::B);
        assert!(ab.contains(Buttons::A));
        assert!(ab.contains(Buttons::B));
        assert!(!ab.contains(Buttons::START));
        assert_eq!(ab.without(Buttons::A), Buttons::B);
        // Removing an unpressed button is a no-op.
        assert_eq!(ab.without(Buttons::START), ab);
        assert_eq!(Buttons::default(), Buttons(0));
    }
}
