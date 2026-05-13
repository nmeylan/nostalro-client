use super::effect_id::EffectId;

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
    /// STR animation plus a supplementary custom-primitive layer running
    /// alongside it. The original game uses both for a handful of skills
    /// (e.g. Stormgust = stormgust.str + spike-row ice shards).
    StrHybrid {
        file: &'static str,
        family: CustomFamily,
        duration_ms: u32,
    },
    /// Behaviour dispatched by [`EffectId`] via [`super::factory::make_effect`].
    /// Per-effect parameters live inside the effect struct, not here.
    Custom { duration_ms: u32 },
    /// Single looping SPR billboard (torches, simple ambient).
    Spr {
        sprite: &'static str,
        duration_ms: u32,
    },
}

/// Legacy family enum still used by [`EffectSpec::StrHybrid`] for the
/// deprecated overlay path. The `Custom` variant no longer uses it — those
/// effects dispatch through the factory by `EffectId`. New code should not
/// add `CustomFamily` variants; the enum disappears in slice F.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomFamily {
    Aura,
    CastCircle,
    SpikeRow,
    Wall,
    CylinderPillar,
    CrossBeam,
    SplineProjectile,
    RadialBurst,
    ScreenFlash,
    FlatQuad,
    HealBurst,
    MeleeImpact,
    AirSwirl,
    StatusOrb,
    FloatingSpirit,
    Waterfall,
    /// Truly bespoke effect - `EffectId` distinguishes which one.
    Bespoke(EffectId),
}
