//! Frame-rate counter shared by the viewers and the game client.
//!
//! Smooths the instantaneous `1/dt` over a short window so the readout doesn't
//! flicker every frame. Feed it the same per-frame delta used for simulation.

/// Exponential moving average of frames-per-second.
pub struct Fps {
    smoothed: f32,
    /// Smoothing factor: fraction of each new sample folded in per frame.
    alpha: f32,
}

impl Default for Fps {
    fn default() -> Self {
        Self {
            smoothed: 0.0,
            alpha: 0.1,
        }
    }
}

impl Fps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one frame's delta (seconds) into the average.
    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let sample = 1.0 / dt;
        if self.smoothed == 0.0 {
            self.smoothed = sample;
        } else {
            self.smoothed += (sample - self.smoothed) * self.alpha;
        }
    }

    /// Current smoothed frames-per-second.
    pub fn get(&self) -> f32 {
        self.smoothed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_toward_steady_frame_rate() {
        let mut fps = Fps::new();
        for _ in 0..200 {
            fps.tick(1.0 / 60.0);
        }
        assert!((fps.get() - 60.0).abs() < 1.0, "got {}", fps.get());
    }
}
