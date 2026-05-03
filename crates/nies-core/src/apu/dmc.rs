//! DMC channel state. Real implementation lands in M5.
//!
//! At M1 this exists only so the bus tick has a `pending_fetch` slot
//! to check (always None) and a no-op `take_pending_fetch` method.

#[derive(Debug, Clone, Default)]
pub struct DmcChannel {
    pending_fetch: Option<u16>,
    /// Number of stall cycles to consume on the next fetch service.
    /// M5 will populate this; M1 always returns 0.
    stall_cycles: u32,
}

impl DmcChannel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the address of a pending CPU-bus sample fetch, if any, and
    /// clears it. The bus services the fetch from inside `Bus::tick`.
    pub fn take_pending_fetch(&mut self) -> Option<u16> {
        self.pending_fetch.take()
    }

    pub fn deliver_sample(&mut self, _val: u8) {
        // M1 stub: no sample buffer yet.
    }

    pub fn stall_cycles(&self) -> u32 {
        self.stall_cycles
    }
}
