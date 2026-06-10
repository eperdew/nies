//! Interim wall-clock frame pacing (M4): run emulated frames at NTSC
//! rate regardless of display refresh, so 120 Hz ProMotion displays
//! don't play at 2× speed. Rendering still happens every vsync; this
//! only decides how many emulated frames each redraw should run.
//! Replaced wholesale by audio-driven pacing at M5 (global spec §8).
//!
//! Platform-free: callers supply "now" as milliseconds from any
//! monotonic-ish epoch (`Instant` on native, `performance.now()` on
//! web), so this stays testable and keeps nies-ui off platform APIs.

/// NTSC NES frame duration in milliseconds (60.0988 frames/sec).
pub const NTSC_FRAME_MS: f64 = 1000.0 / 60.0988;

/// Ceiling on frames run per redraw: a long stall (tab hidden, debugger
/// pause) fast-forwards at most this much instead of free-running.
pub const MAX_FRAMES_PER_UPDATE: u32 = 3;

/// Accumulates wall-clock time and converts it into "how many emulated
/// frames should run now".
#[derive(Debug, Default)]
pub struct FramePacer {
    last_ms: Option<f64>,
    acc_ms: f64,
}

impl FramePacer {
    pub fn new() -> Self {
        Self::default()
    }

    /// `now_ms`: current timestamp in milliseconds (any fixed epoch).
    /// Returns the number of emulated frames to run for this redraw,
    /// 0..=MAX_FRAMES_PER_UPDATE.
    pub fn frames_due(&mut self, now_ms: f64) -> u32 {
        let Some(last) = self.last_ms else {
            // First call: run exactly one frame so startup isn't blank.
            self.last_ms = Some(now_ms);
            return 1;
        };
        let dt = (now_ms - last).max(0.0); // clock regressions count as 0
        self.last_ms = Some(now_ms);
        self.acc_ms = (self.acc_ms + dt).min(f64::from(MAX_FRAMES_PER_UPDATE) * NTSC_FRAME_MS);
        let mut frames = 0;
        while self.acc_ms >= NTSC_FRAME_MS && frames < MAX_FRAMES_PER_UPDATE {
            self.acc_ms -= NTSC_FRAME_MS;
            frames += 1;
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_call_runs_one_frame() {
        let mut p = FramePacer::new();
        assert_eq!(p.frames_due(1000.0), 1);
    }

    #[test]
    fn steady_60hz_runs_one_frame_per_tick() {
        let mut p = FramePacer::new();
        let mut t = 0.0;
        p.frames_due(t); // prime
        let mut total = 0;
        for _ in 0..60 {
            t += 1000.0 / 60.0;
            total += p.frames_due(t);
        }
        // 1 second of 60 Hz ticks ≈ 60 NTSC frames (60.0988 Hz), ±1.
        assert!((59..=61).contains(&total), "got {total}");
    }

    #[test]
    fn steady_120hz_averages_one_frame_per_two_ticks() {
        let mut p = FramePacer::new();
        let mut t = 0.0;
        p.frames_due(t); // prime
        let mut total = 0;
        for _ in 0..120 {
            t += 1000.0 / 120.0;
            let n = p.frames_due(t);
            assert!(n <= 1, "never more than 1 frame per 120 Hz tick");
            total += n;
        }
        assert!((59..=61).contains(&total), "got {total}");
    }

    #[test]
    fn long_stall_is_clamped() {
        let mut p = FramePacer::new();
        p.frames_due(0.0); // prime
        // 5 seconds hidden: don't fast-forward 300 frames.
        assert_eq!(p.frames_due(5000.0), MAX_FRAMES_PER_UPDATE);
        // And the accumulator was clamped, not left huge.
        assert!(p.frames_due(5000.0 + 1.0) <= 1);
    }

    #[test]
    fn non_monotonic_timestamp_is_safe() {
        let mut p = FramePacer::new();
        p.frames_due(1000.0);
        assert_eq!(p.frames_due(900.0), 0); // clock went backwards: no frames
        // and recovers normally afterwards
        let n = p.frames_due(900.0 + NTSC_FRAME_MS);
        assert_eq!(n, 1);
    }
}
