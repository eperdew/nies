//! CPU bus. Exposes `Bus::read` and `Bus::write`, both of which tick the
//! rest of the system (PPU/APU/mapper) one CPU cycle on every access.
//! See spec §3.3.
