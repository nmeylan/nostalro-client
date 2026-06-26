//! Shared caster→target travel for projectile effects.
//!
//! Every projectile effect needs the same thing: advance from a source toward a
//! target along the ground heading, know when it has arrived, and end **exactly**
//! on the target point. Hand-rolling this per effect drifts — some overshoot,
//! some stop short. [`ProjectileCursor`] is the single source of truth: it walks
//! a fixed step per frame and snaps to the target on arrival. Effects layer their
//! own look on top (an arc via [`progress`](ProjectileCursor::progress), a trail
//! of spikes via repeated [`pos`](ProjectileCursor::pos), particles, …).

/// Below this ground distance the source and target are treated as coincident
/// (self-cast / point spawn): the cursor is born already arrived.
const COINCIDENT_EPS: f32 = 1e-3;

/// A cursor advancing along the `from`→`to` ground line at a fixed speed.
#[derive(Clone, Copy, Debug)]
pub struct ProjectileCursor {
    from: [f32; 3],
    to: [f32; 3],
    /// XZ heading of `from`→`to`, in the `dx.atan2(dz)` convention (`sin = dx/dist`,
    /// `cos = dz/dist`) shared by the effect motion code.
    sin_h: f32,
    cos_h: f32,
    /// Ground (XZ) distance from `from` to `to`.
    dist: f32,
    /// World units advanced along the heading per [`advance`](Self::advance).
    step: f32,
    /// Distance advanced so far, clamped to `dist`.
    traveled: f32,
}

impl ProjectileCursor {
    /// New cursor travelling `from`→`to` at `step` world units per frame.
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

    /// Advance one frame. Returns `true` once the cursor has reached `to`.
    pub fn advance(&mut self) -> bool {
        self.traveled = (self.traveled + self.step).min(self.dist);
        self.arrived()
    }

    /// Whether the cursor has reached the target.
    pub fn arrived(&self) -> bool {
        self.traveled >= self.dist
    }

    /// Travel fraction in `[0, 1]` — for arc/scale/alpha that resolve at impact.
    pub fn progress(&self) -> f32 {
        if self.dist <= COINCIDENT_EPS {
            1.0
        } else {
            (self.traveled / self.dist).clamp(0.0, 1.0)
        }
    }

    /// Current world position. XZ follows the heading; Y interpolates linearly
    /// `from`→`to`. Returns `to` exactly once arrived, so the effect always ends
    /// on the target point regardless of step rounding.
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

    /// XZ heading components (`sin`, `cos`) for effects that orient billboards
    /// or spawn along the line themselves.
    pub fn heading_sin_cos(&self) -> (f32, f32) {
        (self.sin_h, self.cos_h)
    }

    /// Ground distance from source to target.
    pub fn dist(&self) -> f32 {
        self.dist
    }

    /// Distance advanced along the heading so far (clamped to `dist`).
    pub fn traveled(&self) -> f32 {
        self.traveled
    }

    /// The target point.
    pub fn target(&self) -> [f32; 3] {
        self.to
    }

    /// Frames the cursor takes to reach the target at its step (≥ 1).
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
        // 31 units at 2.0/frame: 15 whole steps (30) + a partial — the cursor
        // must snap to the target rather than overshoot to 32.
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
