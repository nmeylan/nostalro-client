/// Sliding window over the last six `/doridori` head flips. The SP bonus is
/// granted only when all six land inside a window that is strictly longer than
/// 1.5 s and shorter than 3 s — the upper bound asks for a burst, the lower one
/// rejects a macro firing them instantly.
#[derive(Debug, Default)]
pub struct DoridoriTracker {
    flips: [Option<u32>; 6],
}

impl DoridoriTracker {
    const MIN_SPAN_MS: u32 = 1500;
    const MAX_SPAN_MS: u32 = 3000;

    /// Returns whether this flip completes a qualifying burst.
    pub fn record_flip(&mut self, now_ms: u32) -> bool {
        self.flips.rotate_left(1);
        self.flips[5] = Some(now_ms);
        let Some(oldest) = self.flips[0] else {
            return false;
        };
        let span = now_ms.saturating_sub(oldest);
        span > Self::MIN_SPAN_MS && span < Self::MAX_SPAN_MS
    }

    pub fn reset(&mut self) {
        self.flips = [None; 6];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn six_flips(interval_ms: u32) -> bool {
        let mut tracker = DoridoriTracker::default();
        let mut qualified = false;
        for i in 0..6 {
            qualified = tracker.record_flip(i * interval_ms);
        }
        qualified
    }

    #[test]
    fn only_a_burst_between_one_and_a_half_and_three_seconds_qualifies() {
        assert!(six_flips(400));
        assert!(!six_flips(100));
        assert!(!six_flips(700));
        assert!(!six_flips(300), "a 1500 ms span is excluded by both bounds");
    }

    #[test]
    fn five_flips_never_qualify_and_reset_clears_the_window() {
        let mut tracker = DoridoriTracker::default();
        for i in 0..5 {
            assert!(!tracker.record_flip(i * 300));
        }
        tracker.reset();
        assert!(!tracker.record_flip(1500));
    }
}
