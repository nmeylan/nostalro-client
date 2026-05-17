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
    /// Behaviour dispatched by `EffectId` via
    /// [`super::factory::make_effect`]. Per-effect parameters live inside the
    /// effect struct, not here.
    Custom { duration_ms: u32 },
    /// Single SPR billboard (torches, simple ambient, one-shot impacts).
    ///
    /// `size_scale` is the sprite scale (1.0 = native sprite
    /// scale); `anim_speed` sets how fast the motion advances (every N
    /// ticks at 60 fps, so 2.0 = animation runs half-speed); `repeat`
    /// — when `false` the renderer clamps to the last motion
    /// rather than looping.
    Spr {
        sprite: &'static str,
        duration_ms: u32,
        size_scale: f32,
        anim_speed: f32,
        repeat: bool,
    },
    /// Burst of N animated SPR particles drifting along the Y axis with a
    /// fade-out tail. Used for chimney smoke, firefly puffs, and the rest of
    /// the multi-particle ambient family.
    SprBurst {
        sprite: &'static str,
        /// Lifetime of the parent emitter (when periodic) or single-shot
        /// total time (when one-shot).
        duration_ms: u32,
        burst: SprBurstParams,
    },
    /// Effect with no rendering — original game has neither a sprintf STR
    /// load nor primitive dispatch for this id (pass-through / data-only
    /// effects: status markers, screen messages, no-op packet hooks).
    /// Holder skips the spawn; viewers exclude it from listings.
    Noop,
}

/// Tunables for `EffectSpec::SprBurst`. Mirrors the existing RSW-side
/// `EffectKind::Smoke3D` shape so the renderer can drive both from the same
/// `SpriteEffectEmitter::Smoke3D` path.
#[derive(Clone, Copy, Debug)]
pub struct SprBurstParams {
    /// Per-particle lifetime in milliseconds.
    pub particle_lifetime_ms: f32,
    /// Per-particle sprite size multiplier.
    pub size: f32,
    /// Initial alpha (0..1) — fades linearly to 0 over the particle's life.
    pub alpha_max: f32,
    /// Random burst count, inclusive on both ends.
    pub burst_count_range: (u32, u32),
    /// Random per-particle vertical speed (world units / second / 60),
    /// matching the original game's per-tick particle speed.
    pub speed_range: (f32, f32),
    /// SPR animation speed multiplier (frames per .act delay tick).
    /// Default 4 matches the original game's chimney smoke cadence.
    pub anim_speed: f32,
    /// Particle spawn Y-offset relative to the emitter position. Negative
    /// = upward (native RO coords).
    pub pos_y_start: f32,
    /// Random horizontal scatter radius (XZ) around the anchor at spawn
    /// time. 0 = all particles spawn on the anchor axis. Snow / Detoxication
    /// use this to spread particles across a disc.
    pub spawn_radius_xz: f32,
    /// If `Some(n)`, spawn another burst every `n` frames at 60 fps,
    /// regardless of whether earlier particles are still alive. `None` =
    /// one-shot.
    pub period_frames: Option<u32>,
    /// When true, every burst spawn re-anchors to the current camera target
    /// (supplied via `EffectUpdateCtx.camera_target`) instead of the
    /// effect's `Attach`. Used for ambient weather that must blanket the
    /// player's view regardless of where the effect packet originated.
    /// Falls back silently to the spawn-time Attach if the ctx is missing.
    pub follow_camera: bool,
}
