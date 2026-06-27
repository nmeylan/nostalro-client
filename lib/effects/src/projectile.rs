const COINCIDENT_EPS: f32 = 1e-3;

#[derive(Clone, Copy, Debug)]
pub struct ProjectileCursor {
    from: [f32; 3],
    to: [f32; 3],
    sin_h: f32,
    cos_h: f32,
    dist: f32,
    step: f32,
    traveled: f32,
}

impl ProjectileCursor {
    pub fn new(from: [f32; 3], to: [f32; 3], step: f32) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let dist = (dx * dx + dz * dz).sqrt();
        let (sin_h, cos_h) = if dist > COINCIDENT_EPS {
            dx.atan2(dz).sin_cos()
        } else {
            (0.0, 1.0)
        };
        Self {
            from,
            to,
            sin_h,
            cos_h,
            dist,
            step: step.max(COINCIDENT_EPS),
            traveled: 0.0,
        }
    }

    pub fn advance(&mut self) -> bool {
        self.traveled = (self.traveled + self.step).min(self.dist);
        self.arrived()
    }

    pub fn arrived(&self) -> bool {
        self.traveled >= self.dist
    }

    pub fn progress(&self) -> f32 {
        if self.dist <= COINCIDENT_EPS {
            1.0
        } else {
            (self.traveled / self.dist).clamp(0.0, 1.0)
        }
    }

    pub fn pos(&self) -> [f32; 3] {
        if self.arrived() {
            return self.to;
        }
        let t = self.progress();
        [
            self.from[0] + self.sin_h * self.traveled,
            self.from[1] + (self.to[1] - self.from[1]) * t,
            self.from[2] + self.cos_h * self.traveled,
        ]
    }

    pub fn heading_sin_cos(&self) -> (f32, f32) {
        (self.sin_h, self.cos_h)
    }

    pub fn dist(&self) -> f32 {
        self.dist
    }

    pub fn traveled(&self) -> f32 {
        self.traveled
    }

    pub fn target(&self) -> [f32; 3] {
        self.to
    }

    pub fn frames_to_target(&self) -> f32 {
        (self.dist / self.step).ceil().max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: [f32; 3], b: [f32; 3]) -> bool {
        (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3 && (a[2] - b[2]).abs() < 1e-3
    }

    #[test]
    fn lands_exactly_on_target_for_non_divisible_distance() {
        let to = [10.0, 5.0, 41.0];
        let mut c = ProjectileCursor::new([10.0, 0.0, 10.0], to, 2.0);
        let mut arrived = false;
        for _ in 0..100 {
            if c.advance() {
                arrived = true;
                break;
            }
        }
        assert!(arrived, "cursor should reach the target");
        assert!(approx(c.pos(), to), "pos {:?} != target {:?}", c.pos(), to);
        assert_eq!(c.progress(), 1.0);
    }

    #[test]
    fn advances_along_heading_toward_target() {
        let mut c = ProjectileCursor::new([0.0, 0.0, 0.0], [0.0, 0.0, 30.0], 3.0);
        c.advance();
        let p = c.pos();
        assert!(p[2] > 0.0 && p[2] < 30.0, "advanced part-way: {p:?}");
        assert!((p[0]).abs() < 1e-3, "stays on the straight heading: {p:?}");
    }

    #[test]
    fn coincident_endpoints_start_arrived() {
        let p = [4.0, 1.0, 7.0];
        let c = ProjectileCursor::new(p, p, 2.0);
        assert!(c.arrived());
        assert!(approx(c.pos(), p));
    }
}
