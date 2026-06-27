pub struct Fps {
    smoothed: f32,
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
