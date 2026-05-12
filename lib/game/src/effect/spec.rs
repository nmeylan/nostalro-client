use super::generated::EffectId;

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

/// What "kind" of effect this is — selects which subsystem renders it.
#[derive(Clone, Debug)]
pub enum EffectSpec {
    /// Single STR file played once.
    Str {
        file: &'static str,
        duration_ms: u32,
    },
    /// Family-dispatched custom effect (Aura, GroundRing, SpikeRow, ...).
    Custom {
        family: CustomFamily,
        duration_ms: u32,
    },
    /// Single looping SPR billboard (torches, simple ambient).
    Spr {
        sprite: &'static str,
        duration_ms: u32,
    },
}

/// Identifier for the custom-effect family. Each variant is implemented by
/// exactly one Rust module under `lib/renderer/src/effect/fx/`.
///
/// The renderer crate's `make_custom()` matches on this enum to construct
/// the concrete `CustomEffect` for an `EffectSpec::Custom` entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomFamily {
    Aura,
    GroundRing,
    CastCircle,
    SpikeRow,
    Wall,
    CylinderPillar,
    CrossBeam,
    SplineProjectile,
    RadialBurst,
    ScreenFlash,
    /// Truly bespoke effect — `EffectId` distinguishes which one.
    Bespoke(EffectId),
}
