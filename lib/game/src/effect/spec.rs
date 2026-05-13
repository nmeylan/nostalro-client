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
    /// Family-dispatched custom effect (Aura, GroundRing, SpikeRow, ...).
    Custom {
        family: CustomFamily,
        params: CustomFamilyParams,
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

/// Per-family parameter bundle. One variant per `CustomFamily` that has a
/// data-driven primitive renderer; families without per-effect parameters use
/// `Default` and the family stub falls back to its own constants.
#[derive(Clone, Debug)]
pub enum CustomFamilyParams {
    GroundRing(GroundRingParams),
    Default,
}

/// Blend mode for a custom-effect primitive. Mirrors the renderer's
/// `BlendKind` for the two variants we use from game-side tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectBlend {
    Alpha,
    Additive,
}

/// Per-effect parameters for the `GroundRing` family.
#[derive(Clone, Copy, Debug)]
pub struct GroundRingParams {
    /// GRF path under `data/texture/effect/`. Empty string = fallback white.
    pub texture: &'static str,
    pub radius: f32,
    /// `>= radius` → filled disc; otherwise the inner radius is `radius - thickness`.
    pub thickness: f32,
    pub rotation_deg_per_sec: f32,
    pub color: [f32; 4],
    pub blend: EffectBlend,
    pub fade_in_ms: u16,
    pub fade_out_ms: u16,
}

impl GroundRingParams {
    pub const DEFAULT: Self = Self {
        texture: "",
        radius: 14.0,
        thickness: 14.0,
        rotation_deg_per_sec: 30.0,
        color: [0.6, 0.85, 1.0, 0.55],
        blend: EffectBlend::Additive,
        fade_in_ms: 0,
        fade_out_ms: 0,
    };
}
