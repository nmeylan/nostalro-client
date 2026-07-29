/// A server-driven progress bar shown over the local player while an NPC action
/// runs. It reuses the cast gauge, and the server is told when it empties —
/// whether it filled or was cancelled.
pub struct ProgressBar {
    pub total_secs: f32,
    pub elapsed_secs: f32,
}

impl ProgressBar {
    pub fn new(duration_secs: u32) -> Self {
        Self {
            total_secs: duration_secs.max(1) as f32,
            elapsed_secs: 0.0,
        }
    }

    /// Advances the bar; returns true once it has filled.
    pub fn tick(&mut self, delta: f32) -> bool {
        self.elapsed_secs += delta;
        self.elapsed_secs >= self.total_secs
    }

    pub fn fraction(&self) -> f32 {
        (self.elapsed_secs / self.total_secs).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bar_fills_over_its_stated_duration() {
        let mut bar = ProgressBar::new(2);

        assert!(!bar.tick(1.0));
        assert!((bar.fraction() - 0.5).abs() < 1e-5);
        assert!(bar.tick(1.0));
        assert_eq!(bar.fraction(), 1.0);
    }

    #[test]
    fn a_zero_second_bar_still_has_a_duration_to_divide_by() {
        let mut bar = ProgressBar::new(0);

        assert!(bar.fraction().is_finite());
        assert!(bar.tick(1.0));
    }
}
