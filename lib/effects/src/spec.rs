/// Resolved spawn anchor passed to effect constructors via the factory.
/// The renderer's `EffectHolder` resolves an `Attach` into one of these
/// variants before invoking [`super::factory::make_effect`], so individual
/// effects don't have to match on `Attach` themselves.
///
/// * `Point` covers `Attach::WorldPos` directly, and `Attach::Entity` /
///   `Attach::Projectile` once the entity-to-world resolver has run. The
///   overwhelming majority of effects only need this.
/// * `Trail` carries both endpoints of a projectile-style effect (Frost
///   Diver and the future arrow-shower family). Effects that don't care
///   about the trail call [`EffectAnchor::point`] to collapse it back
///   to the caster-side anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectAnchor {
    /// Single resolved world position.
    Point([f32; 3]),
    /// Pre-resolved endpoints. **Canonical convention, identical at every spawn
    /// site:** `from` = the source (caster/attacker), `to` = the target (struck
    /// entity / destination cell). Projectiles originate at `from`; impact rings
    /// and directional sparks anchor on `to` and aim back toward `from`.
    Trail { from: [f32; 3], to: [f32; 3] },
}

impl EffectAnchor {
    /// Collapse to the caster-side anchor. Used by single-point effects
    /// that don't care about the trail variant — `Point(p)` is `p`, and
    /// `Trail { from, .. }` is `from`.
    pub fn point(self) -> [f32; 3] {
        match self {
            EffectAnchor::Point(p) | EffectAnchor::Trail { from: p, .. } => p,
        }
    }

    /// Expand to `(from, to)` endpoints. `Point(p)` collapses to `(p, p)`
    /// (no direction), the same shape every trail-effect factory arm builds.
    pub fn trail(self) -> ([f32; 3], [f32; 3]) {
        match self {
            EffectAnchor::Point(p) => (p, p),
            EffectAnchor::Trail { from, to } => (from, to),
        }
    }
}

/// How an effect should be positioned in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Attach {
    /// Follow the entity each frame (e.g. Aura on a player).
    Entity(u32),
    /// Fixed world position (e.g. Ice Wall, ground rings).
    WorldPos([f32; 3]),
    /// Projectile from one entity to another. Entity IDs — the renderer
    /// holder resolves `from` to a world position each frame for any
    /// effect that anchors on the caster.
    Projectile { from: u32, to: u32 },
    /// Pre-resolved trail between two world points. Captured once at
    /// spawn; effects that emit shards along a line (Frost Diver, future
    /// arrow-shower style effects) read both endpoints from here. Spawn
    /// callers translate `Projectile { from_entity, to_entity }` to
    /// `Trail { from_world, to_world }` when the trail geometry needs
    /// to live for longer than one frame. **Convention:** `from` = source
    /// (caster/attacker), `to` = target (struck entity); see [`EffectAnchor::Trail`].
    Trail { from: [f32; 3], to: [f32; 3] },
    /// Persistent link between two entities, both re-resolved to world
    /// positions *every frame* (Soul Linker tether). Unlike
    /// `Trail`, neither endpoint is snapshotted: the holder resolves `caster`
    /// and `target` each frame and threads them into the effect via
    /// [`super::effect_trait::EffectUpdateCtx::link_endpoints`]. Carries the
    /// caster and partner account ids.
    Link { caster: u32, target: u32 },
}

/// What "kind" of effect this is - selects which subsystem renders it.
#[derive(Clone, Debug)]
pub enum EffectSpec {
    /// Single STR file played once.
    ///
    /// STR effects anchor at the resolved world position (entity feet / ground).
    /// Actor buff STRs that should appear at the head (Soul Burn / Soul Change,
    /// …) bake that offset into their own authored layer positions — no runtime
    /// lift is applied.
    Str {
        file: &'static str,
        duration_ms: u32,
        /// When `true` the STR animation loops for the effect's whole lifetime
        /// instead of playing once and stopping. Persistent ground units
        /// (Firewall) set this so the cell keeps burning until its disappear
        /// packet, matching the original game's per-cell looped effect.
        repeat: bool,
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
    /// rather than looping; `tint` is an RGBA multiplier (`[1.0; 4]` = no
    /// tint, uses the sprite's own colour). DarkBreath uses `[1.0, 0.0, 0.0, 1.0]`
    /// because the original game shows it pure red (green / blue zeroed).
    Spr {
        sprite: &'static str,
        duration_ms: u32,
        size_scale: f32,
        anim_speed: f32,
        repeat: bool,
        tint: [f32; 4],
        pos_y: f32,
        /// ACT action index to play.
        action_index: usize,
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
        /// Optional caster body recolor played over the burst (the original's
        /// hybrid effects that emit particles *and* recolor the body — e.g.
        /// Enchant Deadly Poison's magenta flicker). `None` for the ambient
        /// bursts that leave the body alone.
        body_recolor: Option<SprBodyRecolor>,
    },
    /// Effect with no rendering — original game has neither a sprintf STR
    /// load nor primitive dispatch for this id (pass-through / data-only
    /// effects: status markers, screen messages, no-op packet hooks).
    /// Holder skips the spawn; viewers exclude it from listings.
    Noop,
}

/// Caster body recolor played alongside a [`EffectSpec::SprBurst`] — the
/// original's hybrid effects that emit a particle burst *and* recolor the body
/// (Enchant Deadly Poison flickers the assassin magenta). The recolor is a
/// per-frame multiply tint, strobed on even frames within the window (the
/// original's `colour ↔ white` alternation).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SprBodyRecolor {
    /// Inclusive effect-age frame window (60 fps) the recolor is active.
    pub window_frames: (u32, u32),
    /// 8-bit multiply tint applied on the flicker's coloured frames.
    pub rgb: [u8; 3],
}

/// Tunables for `EffectSpec::SprBurst` — a rising/drifting particle burst
/// (chimney smoke, poison clouds, …) drawn through the renderer's
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
    /// Constant Y acceleration in world units / sec² applied each frame to
    /// the particle's velocity. Positive = particles fall (Y grows) in
    /// native RO coords. Mirrors the original game's gravity integration
    /// on the falling-particle emitters (Steal). Default
    /// 0 disables gravity.
    pub gravity_world_per_sec2: f32,
    /// When set, particles spawn with a 3D-cone initial velocity instead of
    /// the default pure-Y axis. `(min_lat_deg, max_lat_deg)` are clamped
    /// latitudes from the horizontal plane: `(40, 140)` matches Steal's
    /// "mostly upward but spread" hemisphere. Longitude is always random
    /// 0..360°. The cone speed magnitude is drawn from `speed_range`.
    pub cone_latitude_deg: Option<(f32, f32)>,
    /// When `true`, the rendered sprite size lerps linearly from `size` to
    /// 0 over the particle's lifetime. Matches the shrinking
    /// Steal-style emitters in the original game.
    pub size_shrink: bool,
    /// When `true`, alpha oscillates around the linear fade envelope
    /// instead of monotonically fading. Approximation of the twinkle
    /// behaviour for emitters that don't supply [`Self::alpha_keyframes`] —
    /// reproduces the visible pulsing with a sin² envelope. When
    /// `alpha_keyframes` is non-empty the renderer uses the per-particle
    /// sawtooth driven by those keyframes instead, matching the
    /// original game's exact behaviour.
    pub twinkle: bool,
    /// Optional curved-path parameters. When `Some`, each particle
    /// re-randomizes its heading (longitude/latitude perturbed by
    /// `angle_jitter_deg`) and optionally its speed at a periodic
    /// interval drawn from `subsequent_period_frames`. Mirrors the
    /// original game's curved-path particle drift.
    pub curve: Option<CurveParams>,
    /// Optional twinkle alpha keyframes matching the original game's
    /// per-particle alpha-change schedule. Each entry's
    /// `at_frame` is in 60 fps ticks; when the particle reaches that
    /// age, its alpha and alpha_max are reset to the values in the
    /// entry, then the sawtooth oscillation continues from there.
    /// Empty `&[]` = no keyframe schedule; the renderer falls back
    /// to the linear fade envelope (plus sin² pulse when `twinkle`).
    pub alpha_keyframes: &'static [AlphaKeyframe],
}

/// Parameters for the periodic heading-jitter (curved-path) behaviour.
#[derive(Clone, Copy, Debug)]
pub struct CurveParams {
    /// Frames before the first re-randomization, inclusive range
    /// (5..=30 matches firefly's initial random delay of 25 + 5).
    pub initial_period_frames: (u32, u32),
    /// Frames between subsequent re-randomizations, inclusive range
    /// (5..=15 matches firefly's random delay of 10 + 5).
    pub subsequent_period_frames: (u32, u32),
    /// Maximum random perturbation applied to longitude and latitude
    /// each curve tick. ±40° matches the original game's firefly.
    pub angle_jitter_deg: f32,
    /// When `true`, draw a fresh `speed` from the emitter's
    /// `speed_range` at every curve tick (firefly's behaviour). When
    /// `false`, keep the spawn-time speed.
    pub speed_resample: bool,
}

/// One keyframe in a twinkle alpha schedule. At `at_frame` (60 fps
/// ticks since particle spawn), the particle's instantaneous alpha is
/// reset to `alpha_init` and its oscillation ceiling is reset to
/// `alpha_max`. Values are 0..1; the renderer multiplies by 255 if
/// needed.
#[derive(Clone, Copy, Debug)]
pub struct AlphaKeyframe {
    pub at_frame: u32,
    pub alpha_init: f32,
    pub alpha_max: f32,
}
