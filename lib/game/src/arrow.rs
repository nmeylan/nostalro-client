//! Bow arrow projectile.
//!
//! The flying arrow is a transient world sprite, not an `EffectId` effect —
//! the original game spawns a standalone projectile (not a numbered effect)
//! whenever a bow/whip/instrument wielder enters attack state. It travels in a
//! straight line from shooter to target over the attack motion, faces its
//! direction of travel, and despawns on arrival. There is no `EF_ARROW`, and
//! the server sends no ranged flag — the client decides to draw it purely from
//! the attacker's equipped weapon.

/// `data/sprite/몬스터/skel_archer_arrow{.spr,.act}` — the arrow sprite.
/// (The skill-variant arrow sprite is absent from the classic GRF and needs
/// the skill high-bit flag we don't plumb yet.)
pub const ARROW_SPRITE: &str = "data/sprite/몬스터/skel_archer_arrow";

/// Preloaded by `custom_effect_sprite_paths()` so the sprite is in the
/// `EffectSpriteCache` before the first bow attack arrives.
pub const SPRITES: &[&str] = &[ARROW_SPRITE];

pub struct ArrowProjectile {
    shooter_pos: [f32; 3],
    target_pos: [f32; 3],
    age: f32,
    flight_secs: f32,
}

impl ArrowProjectile {
    pub fn new(shooter_pos: [f32; 3], target_pos: [f32; 3], flight_secs: f32) -> Self {
        Self {
            shooter_pos,
            target_pos,
            age: 0.0,
            flight_secs: flight_secs.max(0.1),
        }
    }

    pub fn advance(&mut self, delta: f32) {
        self.age += delta;
    }

    pub fn is_done(&self) -> bool {
        self.age >= self.flight_secs
    }

    pub fn sprite_path(&self) -> &'static str {
        ARROW_SPRITE
    }

    pub fn target_pos(&self) -> [f32; 3] {
        self.target_pos
    }

    /// Linear interpolation shooter → target over the flight time.
    pub fn current_position(&self) -> [f32; 3] {
        let t = (self.age / self.flight_secs).clamp(0.0, 1.0);
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
    fn lerps_from_shooter_to_target_and_despawns_on_arrival() {
        let mut arrow = ArrowProjectile::new([0.0, 0.0, 0.0], [10.0, 0.0, 20.0], 1.0);

        assert_eq!(arrow.current_position(), [0.0, 0.0, 0.0]);
        assert!(!arrow.is_done());

        arrow.advance(0.5);
        let mid = arrow.current_position();
        assert!((mid[0] - 5.0).abs() < 0.001);
        assert!((mid[2] - 10.0).abs() < 0.001);
        assert!(!arrow.is_done());

        arrow.advance(0.6);
        let end = arrow.current_position();
        assert!((end[0] - 10.0).abs() < 0.001);
        assert!((end[2] - 20.0).abs() < 0.001);
        assert!(arrow.is_done());
    }
}
