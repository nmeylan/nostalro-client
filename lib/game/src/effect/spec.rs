/// How an effect should be positioned in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Attach {
    /// Follow the entity each frame (e.g. Aura on a player).
    Entity(u32),
    /// Fixed world position (e.g. Ice Wall, ground rings).
    WorldPos([f32; 3]),
    /// Projectile from one entity to another.
    Projectile { from: u32, to: u32 },
}

/// What "kind" of effect this is - selects which subsystem renders it.
#[derive(Clone, Debug)]
pub enum EffectSpec {
    /// Single STR file played once.
    Str {
        file: &'static str,
        duration_ms: u32,
    },
    /// Behaviour dispatched by [`super::effect_id::EffectId`] via
    /// [`super::factory::make_effect`]. Per-effect parameters live inside the
    /// effect struct, not here.
    Custom { duration_ms: u32 },
    /// Single looping SPR billboard (torches, simple ambient).
    Spr {
        sprite: &'static str,
        duration_ms: u32,
    },
}
