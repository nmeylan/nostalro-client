pub const DEFAULT_MAX_DIST: f32 = 250.0;
pub const DEFAULT_MIN_DIST: f32 = 40.0;

pub fn attenuate(dx: f32, dy: f32, dz: f32, min_dist: f32, max_dist: f32) -> f32 {
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    min_dist / dist.clamp(min_dist, max_dist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_distance_clamped() {
        assert_eq!(attenuate(0.0, 0.0, 0.0, 40.0, 250.0), 1.0);
        assert_eq!(attenuate(40.0, 0.0, 0.0, 40.0, 250.0), 1.0);
        assert_eq!(attenuate(80.0, 0.0, 0.0, 40.0, 250.0), 0.5);
        assert_eq!(attenuate(250.0, 0.0, 0.0, 40.0, 250.0), 40.0 / 250.0);
        assert_eq!(attenuate(500.0, 0.0, 0.0, 40.0, 250.0), 40.0 / 250.0);
    }

    #[test]
    fn dy_acts_as_volume_knob() {
        let g = attenuate(0.0, -50.0, 0.0, 40.0, 250.0);
        assert!((g - 0.8).abs() < 1e-6);
        let g = attenuate(0.0, -100.0, 0.0, 40.0, 250.0);
        assert!((g - 0.4).abs() < 1e-6);
    }
}
