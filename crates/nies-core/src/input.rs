//! Controller state. Polled by CPU $4016/$4017 reads. See spec §5.5.
//!
//! At M1 this is a stub: $4016/$4017 always read 0 and writes are
//! ignored. M4 implements the strobe latch and 8-bit shift register.

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
}
