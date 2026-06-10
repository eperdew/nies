//! Controller state. Polled by CPU $4016/$4017 reads. See spec §5.5 and
//! the M4 design spec (docs/superpowers/specs/2026-06-10-m4-input-design.md §3).
//!
//! Models the standard controller's 4021 shift register: while the strobe
//! is high the register continuously tracks the live buttons; the falling
//! edge latches; each read returns one bit (A, B, Select, Start, Up, Down,
//! Left, Right) and shifts in a 1, so reads 9+ return 1.

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
    /// Live button state, updated by `Nes::set_buttons`.
    buttons: Buttons,
    /// $4016 bit 0. While high, the shift register tracks `buttons`.
    strobe: bool,
    /// Serial shift register. Refilled with 1s from the top as it shifts.
    shift: u8,
}

impl Controller {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the live button state (does not touch the latch directly;
    /// the strobe/read logic decides when it becomes visible).
    pub fn set_buttons(&mut self, buttons: Buttons) {
        self.buttons = buttons;
    }

    /// Current live button state.
    pub fn buttons(&self) -> Buttons {
        self.buttons
    }

    /// Write to $4016 (strobe). While strobe is high the register tracks
    /// the live buttons, so the falling edge latches the state current at
    /// that write.
    pub fn write_strobe(&mut self, val: u8) {
        let new_strobe = val & 1 != 0;
        if self.strobe || new_strobe {
            self.shift = self.buttons.0;
        }
        self.strobe = new_strobe;
    }

    /// Read the next bit out of the shift register, shifting a 1 in from
    /// the top (official controllers report 1 after the 8th read). While
    /// strobed, returns the live A bit without ever advancing.
    pub fn read(&mut self) -> u8 {
        if self.strobe {
            self.shift = self.buttons.0;
        }
        let bit = self.shift & 1;
        self.shift = (self.shift >> 1) | 0x80;
        bit
    }

    /// The bit a `read` would return, with no mutation. For the
    /// debugger's non-destructive `Bus::peek` path.
    pub fn peek(&self) -> u8 {
        if self.strobe {
            self.buttons.0 & 1
        } else {
            self.shift & 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strobe pulse: latch the current button state into the shift register.
    fn pulse(c: &mut Controller) {
        c.write_strobe(1);
        c.write_strobe(0);
    }

    #[test]
    fn serial_read_order_a_first() {
        let mut c = Controller::new();
        c.set_buttons(Buttons::A | Buttons::UP); // 0x11
        pulse(&mut c);
        let bits: Vec<u8> = (0..8).map(|_| c.read()).collect();
        // A, B, Select, Start, Up, Down, Left, Right
        assert_eq!(bits, vec![1, 0, 0, 0, 1, 0, 0, 0]);
    }

    #[test]
    fn reads_after_eighth_return_one() {
        let mut c = Controller::new();
        c.set_buttons(Buttons::default()); // nothing pressed
        pulse(&mut c);
        for _ in 0..8 {
            assert_eq!(c.read(), 0);
        }
        for i in 8..24 {
            assert_eq!(c.read(), 1, "read #{i} should return 1");
        }
    }

    #[test]
    fn strobe_high_tracks_live_buttons_without_advancing() {
        let mut c = Controller::new();
        c.set_buttons(Buttons::A);
        c.write_strobe(1);
        // Repeated reads while strobed return the live A bit, never advancing.
        assert_eq!(c.read(), 1);
        assert_eq!(c.read(), 1);
        c.set_buttons(Buttons::default());
        assert_eq!(c.read(), 0); // live state, not a latched copy
    }

    #[test]
    fn falling_edge_latches_state_current_at_that_write() {
        let mut c = Controller::new();
        c.write_strobe(1);
        c.set_buttons(Buttons::B); // changes between strobe-on and strobe-off
        c.write_strobe(0);
        let bits: Vec<u8> = (0..8).map(|_| c.read()).collect();
        assert_eq!(bits, vec![0, 1, 0, 0, 0, 0, 0, 0]); // B in slot 2
    }

    #[test]
    fn changes_after_latch_do_not_affect_sequence() {
        let mut c = Controller::new();
        c.set_buttons(Buttons::A | Buttons::B); // 0x03
        pulse(&mut c);
        assert_eq!(c.read(), 1); // A
        c.set_buttons(Buttons::default()); // release everything mid-sequence
        assert_eq!(c.read(), 1); // B still 1 — sequence reads the latch
    }

    #[test]
    fn restrobe_mid_sequence_restarts_from_a() {
        let mut c = Controller::new();
        c.set_buttons(Buttons::A);
        pulse(&mut c);
        let _ = c.read();
        let _ = c.read();
        let _ = c.read();
        pulse(&mut c);
        assert_eq!(c.read(), 1); // back to bit A
    }

    #[test]
    fn peek_is_nondestructive() {
        let mut c = Controller::new();
        c.set_buttons(Buttons::A);
        pulse(&mut c);
        assert_eq!(c.peek(), 1);
        assert_eq!(c.peek(), 1); // unchanged — no shift
        assert_eq!(c.read(), 1); // the real read still sees bit A
        assert_eq!(c.peek(), 0); // now positioned on B (released)
        assert_eq!(c.read(), 0);
    }

    #[test]
    fn peek_while_strobed_reflects_live_a_bit() {
        let mut c = Controller::new();
        c.write_strobe(1);
        c.set_buttons(Buttons::A);
        assert_eq!(c.peek(), 1);
        c.set_buttons(Buttons::default());
        assert_eq!(c.peek(), 0);
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
