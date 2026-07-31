/// Stereo pan for a source `dx`/`dz` away from the listener, projected onto the
/// listener's right axis. Returns -1 (hard left) to 1 (hard right); rotating the
/// listener re-pans every source without moving any of them.
pub fn pan(dx: f32, dz: f32, right_x: f32, right_z: f32) -> f32 {
    let dist = (dx * dx + dz * dz).sqrt();
    let right_len = (right_x * right_x + right_z * right_z).sqrt();
    if dist <= f32::EPSILON || right_len <= f32::EPSILON {
        return 0.0;
    }
    ((dx * right_x + dz * right_z) / (dist * right_len)).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pan_follows_the_listener_right_axis() {
        // Listener facing -Z, so screen-right is +X.
        assert_eq!(pan(10.0, 0.0, 1.0, 0.0), 1.0);
        assert_eq!(pan(-10.0, 0.0, 1.0, 0.0), -1.0);
        assert_eq!(pan(0.0, -10.0, 1.0, 0.0), 0.0);

        // A quarter turn puts the same source on the other side.
        assert_eq!(pan(10.0, 0.0, 0.0, 1.0), 0.0);
        assert_eq!(pan(10.0, 0.0, -1.0, 0.0), -1.0);

        // Degenerate inputs stay centred.
        assert_eq!(pan(0.0, 0.0, 1.0, 0.0), 0.0);
        assert_eq!(pan(10.0, 0.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn diagonal_is_partially_panned() {
        let p = pan(10.0, -10.0, 1.0, 0.0);
        assert!(p > 0.0 && p < 1.0);
        assert!((p - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    }
}
