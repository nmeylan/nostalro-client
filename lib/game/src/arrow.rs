//! Bow arrow projectile.
//!
//! The flying arrow is a transient world sprite, not an `EffectId` effect —
//! the original game spawns a standalone projectile (not a numbered effect)
//! whenever a bow/whip/instrument wielder enters attack state. It travels in a
//! straight line from shooter to target over the attack motion, faces its
//! direction of travel, and despawns on arrival. There is no `EF_ARROW`, and
//! the server sends no ranged flag — the client decides to draw it purely from
//! the attacker's equipped weapon.

/// The arrow sprite path and preload slice live with the effect preload
/// aggregator in `ragnarok-effects`. (The skill-variant arrow sprite is absent
/// from the classic GRF and needs the skill high-bit flag we don't plumb yet.)
pub use crate::effect::{ARROW_SPRITE, SPRITES};

/// In the original game the arrow covers any distance in a fixed ~192 ms
/// (8 frames × 24 ms), flying proportionally faster for targets closer than
/// 8 cells so close shots don't feel sluggish.
pub fn flight_secs_for_cell_distance(dist_cells: f32) -> f32 {
    const BASE: f32 = 0.192; // 8 frames * 24ms
    BASE * (dist_cells / 8.0).clamp(0.05, 1.0)
}

pub struct ArrowProjectile {
    shooter_pos: [f32; 3],
    target_pos: [f32; 3],
    age: f32,
    /// The arrow stays hidden (not yet released) for this long after spawn,
    /// matching the original game's pre-release hold. Multi-hit skills also
    /// fold their per-hit stagger into this delay.
    delay_secs: f32,
    flight_secs: f32,
}

impl ArrowProjectile {
    pub fn new(shooter_pos: [f32; 3], target_pos: [f32; 3], delay_secs: f32, flight_secs: f32) -> Self {
        Self {
            shooter_pos,
            target_pos,
            age: 0.0,
            delay_secs: delay_secs.max(0.0),
            flight_secs: flight_secs.max(0.05),
        }
    }

    pub fn advance(&mut self, delta: f32) {
        self.age += delta;
    }

    /// The arrow is released (and drawn) only after its hide delay elapses.
    pub fn is_visible(&self) -> bool {
        self.age >= self.delay_secs
    }

    pub fn is_done(&self) -> bool {
        self.age >= self.delay_secs + self.flight_secs
    }

    pub fn sprite_path(&self) -> &'static str {
        ARROW_SPRITE
    }

    pub fn target_pos(&self) -> [f32; 3] {
        self.target_pos
    }

    /// Linear interpolation shooter → target over the flight time, after the
    /// hide delay.
    pub fn current_position(&self) -> [f32; 3] {
        let t = ((self.age - self.delay_secs) / self.flight_secs).clamp(0.0, 1.0);
        [
            self.shooter_pos[0] + (self.target_pos[0] - self.shooter_pos[0]) * t,
            self.shooter_pos[1] + (self.target_pos[1] - self.shooter_pos[1]) * t,
            self.shooter_pos[2] + (self.target_pos[2] - self.shooter_pos[2]) * t,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_hidden_then_zips_from_shooter_to_target() {
        // Far target: fixed ~192ms flight, faster up close.
        assert!((flight_secs_for_cell_distance(16.0) - 0.192).abs() < 1e-6);
        assert!(flight_secs_for_cell_distance(4.0) < 0.192);

        let flight = flight_secs_for_cell_distance(16.0);
        let mut arrow = ArrowProjectile::new([0.0, 0.0, 0.0], [10.0, 0.0, 20.0], 0.5, flight);

        // Hidden during the delay, parked at the shooter.
        assert!(!arrow.is_visible());
        assert_eq!(arrow.current_position(), [0.0, 0.0, 0.0]);
        assert!(!arrow.is_done());

        arrow.advance(0.5 + flight * 0.5);
        assert!(arrow.is_visible());
        let mid = arrow.current_position();
        assert!((mid[0] - 5.0).abs() < 0.001);
        assert!((mid[2] - 10.0).abs() < 0.001);
        assert!(!arrow.is_done());

        arrow.advance(flight * 0.5 + 0.01);
        let end = arrow.current_position();
        assert!((end[0] - 10.0).abs() < 0.001);
        assert!((end[2] - 20.0).abs() < 0.001);
        assert!(arrow.is_done());
    }
}
