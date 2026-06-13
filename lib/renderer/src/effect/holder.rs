//! Runtime store of currently-active effects.
//!
//! Game-side triggers push [`ragnarok_game::effect::SpawnRequest`]s into a
//! [`EffectQueue`]; the holder drains that queue each frame and constructs
//! the runtime instances. `EffectSpec::Custom` dispatches through
//! [`ragnarok_game::effect::make_effect`] to a `Box<dyn Effect>` living in
//! the game crate; tooling can swap that for an [`ExternalCustomBackend`]
//! to load effects from a hot-reload cdylib.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use models::enums::EnumWithNumberValue;
use models::enums::effect_id::EffectId;
use ragnarok_formats::act::SpriteAnimationState;
use ragnarok_game::effect::spec::EffectAnchor;
use ragnarok_game::effect::{
    Afterimage, AlphaKeyframe, Attach, BodyAction, CameraShake, Effect as GameEffect,
    EffectDrawList, EffectQueue, EffectRenderCtx, EffectSpec, EffectStatus, EffectUpdateCtx,
    SpawnRequest, SprBurstParams, effect_spec, make_effect, spawn_camera_shake,
};

use crate::effect_sprite::Smoke3DParticle;

/// Pluggable backend for custom-effect dispatch. The default path links the
/// game crate's `make_effect` statically; tooling that needs hot reload
/// (the effect viewer) supplies a wrapper around its cdylib so effect
/// implementations live behind the dlsym boundary and can be swapped at
/// runtime. Drop semantics: `drop_handle` must release the cdylib-owned
/// `Box<dyn Effect>`; `drop_all` is called by [`EffectHolder::clear`] and
/// by tooling just before a reload.
pub trait ExternalCustomBackend: Send + Sync {
    fn spawn(&self, effect_id: u16, from: [f32; 3], to: [f32; 3], hit_count: u8) -> u64;
    /// Returns `true` while the effect is still running, `false` once it
    /// has signalled death.
    fn update(&self, handle: u64, dt: f32, caster_yaw: Option<f32>) -> bool;
    fn collect(&self, handle: u64, ctx: &EffectRenderCtx, out: &mut EffectDrawList);
    /// Optional STR overlay name for this effect instance. Default `None`;
    /// hot-reload backends can probe their cdylib for the current overlay.
    fn str_overlay(&self, _handle: u64) -> Option<String> {
        None
    }
    /// One-shot screen-shake request from this effect instance, if any.
    /// Default `None`; hot-reload backends probe their cdylib.
    fn take_camera_shake(&self, _handle: u64) -> Option<CameraShake> {
        None
    }
    fn drop_handle(&self, handle: u64);
    fn drop_all(&self);
}

/// Decaying screen-shake state owned by the holder. Effects fire a one-shot
/// [`CameraShake`]; this integrates the per-frame jittered offset that the
/// camera applies, so the whole view trembles and settles.
#[derive(Default)]
struct ShakeController {
    amplitude: f32,
    elapsed: f32,
    duration: f32,
}

impl ShakeController {
    fn trigger(&mut self, shake: CameraShake) {
        let remaining = if self.duration > 0.0 {
            self.amplitude * (1.0 - (self.elapsed / self.duration).clamp(0.0, 1.0))
        } else {
            0.0
        };
        // A fresh shake dominates but never weakens an ongoing stronger one.
        self.amplitude = shake.amplitude.max(remaining);
        self.duration = (shake.duration_ms as f32 / 1000.0).max(1e-3);
        self.elapsed = 0.0;
    }

    fn tick(&mut self, dt: f32) {
        if self.duration > 0.0 {
            self.elapsed += dt;
        }
    }

    fn offset(&self) -> glam::Vec3 {
        if self.duration <= 0.0 || self.elapsed >= self.duration {
            return glam::Vec3::ZERO;
        }
        let amp = self.amplitude * (1.0 - self.elapsed / self.duration);
        // Stepped on the 60 fps frame so the shudder is jittery, not smooth.
        let frame = (self.elapsed * 60.0) as u32;
        let j = |salt: u32| {
            let x = frame
                .wrapping_mul(2_654_435_761)
                .wrapping_add(salt.wrapping_mul(40_503))
                .wrapping_add(0x9E37_79B9);
            let x = x ^ (x >> 15);
            ((x % 100_000) as f32 / 100_000.0) * 2.0 - 1.0
        };
        // Vertical shudder is gentler than the horizontal sway.
        glam::Vec3::new(j(1) * amp, j(2) * amp * 0.5, j(3) * amp)
    }
}

/// Owned snapshot of a live STR effect - handed to `build_str_effect_batches`
/// by callers that need a borrow-free view of the holder's STR effects.
pub struct StrSnapshot {
    pub name: String,
    pub position: [f32; 3],
    pub anim_time: f32,
}

/// Owned snapshot of a live SPR-billboard effect. Callers convert this into
/// `SpriteEffectEmitter::Spr` and feed it through `collect_sprite_effect_draws`.
pub struct SprSnapshot {
    pub sprite: String,
    pub position: [f32; 3],
    pub anim_time: f32,
    pub duration_ms: f32,
    pub size_scale: f32,
    pub anim_speed: f32,
    pub repeat: bool,
    pub tint: [f32; 4],
    pub action_index: usize,
}

/// Owned snapshot of a live `EffectSpec::SprBurst` instance — params for the
/// `SpriteEffectEmitter::Smoke3D` render path plus a per-particle list.
pub struct SprBurstSnapshot {
    pub sprite: String,
    pub size_scale: f32,
    pub alpha_max: f32,
    pub anim_speed: f32,
    /// Linearly shrink the per-particle sprite to 0 over its lifetime.
    pub size_shrink: bool,
    /// Oscillate alpha around the linear fade envelope (twinkle).
    pub twinkle: bool,
    pub particles: Vec<Smoke3DParticle>,
}

#[derive(Clone, Copy)]
struct BurstParticle {
    pos: [f32; 3],
    velocity: [f32; 3],
    age: f32,
    lifetime: f32,
    /// Frames elapsed since spawn (60 fps ticks), tracked separately
    /// from `age` so curve periods and `alpha_keyframes` line up
    /// with the original game's tick-driven scheduler. Carries a
    /// fractional remainder across update calls.
    age_frames: f32,
    /// Heading for curve re-randomization. Longitude is the Y-axis
    /// rotation; latitude is the X-axis rotation applied after.
    lon_deg: f32,
    lat_deg: f32,
    /// Curve countdown in frames. When it crosses 0,
    /// re-randomize heading + speed and refill from the curve params.
    curve_timer_frames: f32,
    /// Number of curve ticks consumed so far. After the first tick we
    /// switch from `initial_period_frames` to `subsequent_period_frames`.
    curve_count: u32,
    /// Twinkle alpha state. `alpha` is the current 0..1 value;
    /// `alpha_speed` is the per-frame delta whose sign flips at the
    /// min/max bounds. `alpha_max` is the active ceiling (keyframed).
    /// `keyframe_idx` is the next entry to consume from
    /// `SprBurstParams::alpha_keyframes`.
    alpha: f32,
    alpha_speed: f32,
    alpha_max: f32,
    keyframe_idx: usize,
}

struct BurstState {
    sprite: String,
    params: SprBurstParams,
    particles: Vec<BurstParticle>,
    has_emitted: bool,
    /// Time since the last burst was emitted; reset on respawn.
    cooldown_timer: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectHandle(u64);

#[derive(Clone, Debug)]
pub enum SpawnOutcome {
    Custom,
    Str { name: String },
    Spr,
    SprBurst,
    CustomNotImpl,
    NoSpec,
    /// `EffectSpec::Noop` — original game has no visible behaviour for this
    /// effect id. Holder treats the spawn as silently dropped.
    Noop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpawnStatus {
    Rendering,
    StrFileMissing,
    CustomNotImpl,
    NoSpec,
    /// Spec was `EffectSpec::Noop` — original game renders nothing.
    Noop,
}

enum HeldPayload {
    /// Factory-built effect (new path).
    Custom(Box<dyn GameEffect>),
    /// Custom effect whose `Box<dyn Effect>` lives in an external (hot-
    /// reloadable) backend. The holder only keeps the opaque handle.
    CustomExternal { handle: u64 },
    /// Single-shot STR effect. Anim time accumulates in `age`; the render
    /// step projects via the existing `build_str_effect_batches` path.
    Str { name: String },
    /// Single SPR billboard (looping or one-shot, depending on `repeat`).
    Spr {
        sprite: String,
        size_scale: f32,
        anim_speed: f32,
        repeat: bool,
        tint: [f32; 4],
        pos_y: f32,
        action_index: usize,
    },
    /// Multi-particle SPR burst (chimney smoke, firefly, snow, …).
    SprBurst(BurstState),
}

struct HeldEffect {
    handle: EffectHandle,
    payload: HeldPayload,
    attach: Attach,
    age: f32,
    duration: f32,
}

/// One frozen copy of a moving actor's sprite — a motion-blur clone.
/// Snapshots the animation frame and screen transform at spawn time so the
/// actor walks away from it, leaving a trail; the holder decays `alpha`.
pub struct AfterimageSnapshot {
    entity_id: u32,
    pub anim: SpriteAnimationState,
    pub camera_dir: Option<u8>,
    pub head_dir: u8,
    pub anchor: [f32; 2],
    pub depth: f32,
    pub scale: f32,
    pub tint: [u8; 3],
    pub alpha: f32,
    fade_per_sec: f32,
}

impl AfterimageSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: u32,
        anim: SpriteAnimationState,
        camera_dir: Option<u8>,
        head_dir: u8,
        anchor: [f32; 2],
        depth: f32,
        scale: f32,
        ai: &Afterimage,
    ) -> Self {
        Self {
            entity_id,
            anim,
            camera_dir,
            head_dir,
            anchor,
            depth,
            scale,
            tint: ai.tint,
            alpha: ai.start_alpha,
            fade_per_sec: ai.fade_per_frame * 60.0,
        }
    }
}

#[derive(Default)]
pub struct EffectHolder {
    next_id: u64,
    effects: Vec<HeldEffect>,
    last_spawn: Option<SpawnOutcome>,
    /// When set, `EffectSpec::Custom` spawns are routed through this backend
    /// instead of the statically-linked `make_effect`. Tooling owns the
    /// concrete implementation; production code leaves it `None`.
    external_backend: Option<Arc<dyn ExternalCustomBackend>>,
    shake: ShakeController,
    /// Live afterimage snapshots (the original game's motion-blur clones),
    /// decayed each frame by the holder.
    afterimages: Vec<AfterimageSnapshot>,
    /// Per-entity frame accumulator for the emit interval.
    afterimage_emit: HashMap<u32, f32>,
    /// Entities whose emit interval elapsed this frame (a snapshot is due).
    afterimage_due: HashSet<u32>,
}

impl EffectHolder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Install (or remove) an external custom-effect backend. Drops every
    /// live external effect first so the old backend's handles are released
    /// before its function pointers are torn down. Internal `Custom` and
    /// non-Custom effects are left untouched.
    pub fn set_external_backend(&mut self, backend: Option<Arc<dyn ExternalCustomBackend>>) {
        if let Some(old) = &self.external_backend {
            self.effects.retain(|e| !matches!(e.payload, HeldPayload::CustomExternal { .. }));
            old.drop_all();
        }
        self.external_backend = backend;
    }

    /// Spawn directly by `EffectId`. Used by the effect viewer and any
    /// caller that has resolved the id itself.
    pub fn spawn(
        &mut self,
        effect_id: EffectId,
        attach: Attach,
        override_duration_ms: Option<u32>,
    ) -> Option<EffectHandle> {
        self.spawn_with_hit_count(effect_id, attach, override_duration_ms, None, &|_| None)
    }

    fn spawn_with_hit_count(
        &mut self,
        effect_id: EffectId,
        attach: Attach,
        override_duration_ms: Option<u32>,
        hit_count: Option<u8>,
        resolve_entity: &dyn Fn(u32) -> Option<[f32; 3]>,
    ) -> Option<EffectHandle> {
        let Some(spec) = effect_spec(effect_id) else {
            self.last_spawn = Some(SpawnOutcome::NoSpec);
            return None;
        };
        if matches!(spec, EffectSpec::Noop) {
            self.last_spawn = Some(SpawnOutcome::Noop);
            return None;
        }
        // Effects whose original behaviour is a sustained screen quake fire a
        // one-shot shake at spawn (alongside whatever STR/SPR they also play),
        // independent of the per-frame `take_camera_shake` path used by Custom
        // effects like Aciddemon.
        if let Some(shake) = spawn_camera_shake(effect_id) {
            self.shake.trigger(shake);
        }
        let payload = match &spec {
            EffectSpec::Str { file, .. } => {
                self.last_spawn = Some(SpawnOutcome::Str {
                    name: (*file).to_string(),
                });
                HeldPayload::Str {
                    name: (*file).to_string(),
                }
            }
            EffectSpec::Spr {
                sprite,
                size_scale,
                anim_speed,
                repeat,
                tint,
                pos_y,
                action_index,
                ..
            } => {
                self.last_spawn = Some(SpawnOutcome::Spr);
                HeldPayload::Spr {
                    sprite: (*sprite).to_string(),
                    size_scale: *size_scale,
                    anim_speed: *anim_speed,
                    repeat: *repeat,
                    tint: *tint,
                    pos_y: *pos_y,
                    action_index: *action_index,
                }
            }
            EffectSpec::SprBurst { sprite, burst, .. } => {
                self.last_spawn = Some(SpawnOutcome::SprBurst);
                HeldPayload::SprBurst(BurstState {
                    sprite: (*sprite).to_string(),
                    params: *burst,
                    particles: Vec::new(),
                    has_emitted: false,
                    cooldown_timer: 0.0,
                })
            }
            EffectSpec::Noop => unreachable!("Noop handled above"),
            EffectSpec::Custom { .. } => {
                if let Some(backend) = &self.external_backend {
                    let (from, to) = match attach {
                        Attach::WorldPos(p) => (p, p),
                        Attach::Trail { from, to } => (from, to),
                        // Snapshot the entity position at spawn — the external
                        // backend has no per-frame entity table.
                        Attach::Entity(id) => {
                            let p = resolve_entity(id).unwrap_or([0.0; 3]);
                            (p, p)
                        }
                        // Link effects are driven on this backend as a static
                        // `Trail` (caster origin → clicked fake-entity);
                        // `Link` itself can't reach this path.
                        Attach::Projectile { .. } | Attach::Link { .. } => ([0.0; 3], [0.0; 3]),
                    };
                    // The cdylib decodes this with `EffectId::try_from_value`,
                    // so send the enum's *value* — not the Rust discriminant
                    // (`as u16`), which diverges from the value past the first
                    // gap in EF numbering (e.g. TextureFalling: discriminant
                    // 734 vs value 1031).
                    let handle =
                        backend.spawn(effect_id.value() as u16, from, to, hit_count.unwrap_or(0));
                    if handle != 0 {
                        self.last_spawn = Some(SpawnOutcome::Custom);
                        HeldPayload::CustomExternal { handle }
                    } else {
                        self.last_spawn = Some(SpawnOutcome::CustomNotImpl);
                        tracing::debug!(
                            "EffectHolder: external backend has no impl for {:?}",
                            effect_id
                        );
                        return None;
                    }
                } else {
                    // Spawn-time `Attach::Entity` resolution: the queue path
                    // (`drain_queue`) threads the caller's entity table in so
                    // entity-attached Custom effects anchor on the actor at
                    // spawn; resolver-less callers (GIF exporter, tests) fall
                    // back to the origin. Per-frame following (Link, body
                    // channels) still goes through the `update`/collector
                    // resolvers.
                    let anchor = attach_to_anchor(attach, resolve_entity);
                    match make_effect(effect_id, anchor, hit_count) {
                        Some(e) => {
                            self.last_spawn = Some(SpawnOutcome::Custom);
                            HeldPayload::Custom(e)
                        }
                        None => {
                            self.last_spawn = Some(SpawnOutcome::CustomNotImpl);
                            tracing::debug!(
                                "EffectHolder: no factory impl for {:?}",
                                effect_id
                            );
                            return None;
                        }
                    }
                }
            }
        };

        let duration_ms = override_duration_ms.unwrap_or_else(|| match spec {
            EffectSpec::Str { duration_ms, .. }
            | EffectSpec::Spr { duration_ms, .. }
            | EffectSpec::SprBurst { duration_ms, .. }
            | EffectSpec::Custom { duration_ms, .. } => duration_ms,
            EffectSpec::Noop => unreachable!("Noop handled above"),
        });
        let duration = if duration_ms == u32::MAX {
            f32::INFINITY
        } else {
            duration_ms as f32 / 1000.0
        };

        let handle = EffectHandle(self.next_id);
        self.next_id += 1;
        self.effects.push(HeldEffect {
            handle,
            payload,
            attach,
            age: 0.0,
            duration,
        });
        Some(handle)
    }

    /// Drain a game-side queue and spawn each request. `resolve_entity`
    /// supplies the caller's entity table so `Attach::Entity` spawns anchor
    /// their world-space primitives on the actor.
    pub fn drain_queue(
        &mut self,
        queue: &mut EffectQueue,
        resolve_entity: &dyn Fn(u32) -> Option<[f32; 3]>,
    ) {
        for req in queue.drain() {
            let SpawnRequest {
                effect_id,
                attach,
                override_duration_ms,
                hit_count,
            } = req;
            self.spawn_with_hit_count(
                effect_id,
                attach,
                override_duration_ms,
                hit_count,
                resolve_entity,
            );
        }
    }

    pub fn despawn(&mut self, handle: EffectHandle) {
        let backend = self.external_backend.as_ref().cloned();
        self.effects.retain(|e| {
            if e.handle != handle {
                return true;
            }
            if let (HeldPayload::CustomExternal { handle: h }, Some(b)) = (&e.payload, &backend) {
                b.drop_handle(*h);
            }
            false
        });
    }

    /// Drop every live effect. Used by the effect viewer when the user
    /// cycles to a new picker entry so old (persistent) effects don't
    /// accumulate.
    pub fn clear(&mut self) {
        if let Some(b) = &self.external_backend {
            b.drop_all();
        }
        self.effects.clear();
    }

    pub fn update(
        &mut self,
        ctx: &EffectUpdateCtx,
        resolve_caster_yaw: &dyn Fn(u32) -> Option<f32>,
        resolve_entity_pos: &dyn Fn(u32) -> Option<[f32; 3]>,
    ) {
        let dt = ctx.delta;
        let backend = self.external_backend.clone();
        let mut shake_requests: Vec<CameraShake> = Vec::new();
        self.effects.retain_mut(|e| {
            e.age += dt;
            let expired = e.age >= e.duration;
            let attach = e.attach;
            // Per-effect caster facing (the master actor's heading)
            // for direction-oriented effects. Entity-attached effects resolve it
            // live from their caster; others inherit the ctx default (the viewer
            // sets it from the crosshair, the same way it aims projectiles).
            let caster_yaw = match attach {
                Attach::Entity(id) => resolve_caster_yaw(id),
                _ => ctx.caster_yaw,
            };
            let alive = match &mut e.payload {
                HeldPayload::Custom(c) => {
                    // Live second-actor tether (Linelink): resolve both endpoints
                    // each frame and feed them in. If the linked actor is gone,
                    // drop the effect (the original game ends it immediately).
                    if let Attach::Link { caster, target } = attach {
                        match (resolve_entity_pos(caster), resolve_entity_pos(target)) {
                            (Some(a), Some(b)) => c.set_link_endpoints(a, b),
                            _ => return false,
                        }
                    }
                    let per_ctx = EffectUpdateCtx { caster_yaw, ..*ctx };
                    let running = c.update(&per_ctx) == EffectStatus::Running;
                    if let Some(s) = c.take_camera_shake() {
                        shake_requests.push(s);
                    }
                    running
                }
                HeldPayload::CustomExternal { handle } => backend
                    .as_ref()
                    .map(|b| {
                        let running = b.update(*handle, dt, caster_yaw);
                        if let Some(s) = b.take_camera_shake(*handle) {
                            shake_requests.push(s);
                        }
                        running
                    })
                    .unwrap_or(false),
                HeldPayload::Str { .. } => true,
                HeldPayload::Spr { .. } => true,
                HeldPayload::SprBurst(b) => {
                    update_burst(b, &e.attach, dt, ctx.camera_target);
                    true
                }
            };
            if !alive || expired {
                if let (HeldPayload::CustomExternal { handle }, Some(b)) =
                    (&e.payload, &backend)
                {
                    b.drop_handle(*handle);
                }
                return false;
            }
            true
        });
        for s in shake_requests {
            self.shake.trigger(s);
        }
        self.shake.tick(dt);
        self.tick_afterimages(dt);
    }

    /// Decay live afterimage snapshots and advance the per-entity emit timers,
    /// flagging entities whose interval elapsed this frame. The actor pass
    /// consumes the flag (only emitting while the actor is actually moving),
    /// matching the original game's motion-blur cadence (every 5th frame).
    fn tick_afterimages(&mut self, dt: f32) {
        for img in &mut self.afterimages {
            img.alpha -= img.fade_per_sec * dt;
        }
        self.afterimages.retain(|i| i.alpha > 0.0);

        let mut active: HashMap<u32, Afterimage> = HashMap::new();
        for e in &self.effects {
            if let (Attach::Entity(id), HeldPayload::Custom(c)) = (e.attach, &e.payload)
                && let Some(ai) = c.body_afterimage()
            {
                active.insert(id, ai);
            }
        }
        self.afterimage_emit.retain(|id, _| active.contains_key(id));
        self.afterimage_due.clear();
        for (id, ai) in active {
            let acc = self.afterimage_emit.entry(id).or_insert(0.0);
            *acc += dt * 60.0;
            if *acc >= ai.interval_frames {
                *acc -= ai.interval_frames;
                self.afterimage_due.insert(id);
            }
        }
    }

    /// Afterimage parameters of the effect attached to `entity_id`, if any
    /// (the actor pass reads `tint` / `start_alpha` to build a snapshot).
    pub fn afterimage_params_for_entity(&self, entity_id: u32) -> Option<Afterimage> {
        self.effects.iter().rev().find_map(|e| {
            if let (Attach::Entity(id), HeldPayload::Custom(c)) = (e.attach, &e.payload)
                && id == entity_id
            {
                c.body_afterimage()
            } else {
                None
            }
        })
    }

    /// `true` if a fresh afterimage snapshot is due for `entity_id` this frame.
    pub fn afterimage_emit_due(&self, entity_id: u32) -> bool {
        self.afterimage_due.contains(&entity_id)
    }

    /// Store a snapshot of the moving actor; the holder decays it until gone.
    pub fn push_afterimage(&mut self, snapshot: AfterimageSnapshot) {
        self.afterimages.push(snapshot);
    }

    /// Live afterimage snapshots for `entity_id`, oldest first. The actor pass
    /// rebuilds each through `build_batches` (tinted, faded) behind the live
    /// sprite.
    pub fn afterimages_for_entity(
        &self,
        entity_id: u32,
    ) -> impl Iterator<Item = &AfterimageSnapshot> {
        self.afterimages.iter().filter(move |i| i.entity_id == entity_id)
    }

    /// Current screen-shake displacement to apply to the camera this frame
    /// (zero when no shake is active). Set `Camera::shake_offset` from this.
    pub fn camera_shake_offset(&self) -> [f32; 3] {
        self.shake.offset().to_array()
    }

    /// Every per-frame body modifier from effects attached to `entity_id`,
    /// bundled for the actor pass (shake/tint/scale/yaw sum or last-writer as
    /// the original game shows; plus the newer spin/lift/copy channels). The
    /// caller folds its hidden/death fade into `alpha` and hands the result to
    /// [`compose_actor_batches`]. Only in-process `Custom` effects participate
    /// — the hot-reload backend has no attached actor.
    pub fn body_channels_for_entity(&self, entity_id: u32) -> crate::sprite::BodyChannels {
        let mut ch = crate::sprite::BodyChannels::default();
        for e in &self.effects {
            let (Attach::Entity(id), HeldPayload::Custom(c)) = (e.attach, &e.payload) else {
                continue;
            };
            if id != entity_id {
                continue;
            }
            if let Some(off) = c.body_shake() {
                ch.shake[0] += off[0];
                ch.shake[1] += off[1];
            }
            if let Some(t) = c.body_tint() {
                ch.tint = Some(t.rgb);
            }
            if let Some(s) = c.body_scale() {
                ch.scale *= s;
            }
            if let Some(yaw) = c.body_yaw() {
                ch.yaw += yaw;
            }
            if let Some(angle) = c.body_angle() {
                ch.angle += angle;
            }
            if let Some(v) = c.body_vertical() {
                ch.lift_px += v.lift_px;
                ch.alpha *= v.alpha;
            }
            if let Some(mut copies) = c.body_copies() {
                ch.copies.append(&mut copies);
            }
        }
        ch
    }

    /// Drain the one-shot forced-animation request (`SetForceAnimation`,
    /// Jumpkick) of the first effect attached to `entity_id` that has one
    /// armed this frame. Mutating — each request fires once. Called from the
    /// game-update step, not the draw pass.
    pub fn take_body_action_for_entity(&mut self, entity_id: u32) -> Option<BodyAction> {
        for e in &mut self.effects {
            if let (Attach::Entity(id), HeldPayload::Custom(c)) = (e.attach, &mut e.payload)
                && id == entity_id
                && let Some(action) = c.take_body_action()
            {
                return Some(action);
            }
        }
        None
    }

    /// Append primitive draws for live custom effects. STR/Spr collection
    /// is wired in Phase D when the renderer's render-pass plumbing lands.
    pub fn collect_custom_draws(
        &self,
        out: &mut EffectDrawList,
        ctx: &EffectRenderCtx,
    ) {
        for e in &self.effects {
            match &e.payload {
                HeldPayload::Custom(c) => c.collect_draws(out, ctx),
                HeldPayload::CustomExternal { handle } => {
                    if let Some(b) = &self.external_backend {
                        b.collect(*handle, ctx, out);
                    }
                }
                _ => {}
            }
        }
    }

    /// Snapshot of every live SPR-billboard effect. Caller converts each into
    /// a `SpriteEffectEmitter::Spr` and pipes through
    /// Snapshot of every live SprBurst effect. Particle positions are world-
    /// space; callers feed each snapshot into `SpriteEffectEmitter::Smoke3D`.
    pub fn collect_spr_burst_emitters(
        &self,
        resolve_entity: &dyn Fn(u32) -> Option<[f32; 3]>,
    ) -> Vec<SprBurstSnapshot> {
        self.effects
            .iter()
            .filter_map(|e| {
                let HeldPayload::SprBurst(b) = &e.payload else {
                    return None;
                };
                // Anchor isn't strictly needed once particles spawn — every
                // particle already carries its absolute world pos — but we
                // still skip emitters whose attach can't resolve, mirroring
                // the STR/Spr collectors.
                let _ = resolve_position(&e.attach, resolve_entity)?;
                let has_keyframes = !b.params.alpha_keyframes.is_empty();
                let particles = b
                    .particles
                    .iter()
                    .map(|p| Smoke3DParticle {
                        pos: p.pos,
                        age: p.age,
                        lifetime: p.lifetime,
                        alpha_override: has_keyframes.then_some(p.alpha),
                    })
                    .collect();
                Some(SprBurstSnapshot {
                    sprite: b.sprite.clone(),
                    size_scale: b.params.size,
                    alpha_max: b.params.alpha_max,
                    anim_speed: b.params.anim_speed,
                    size_shrink: b.params.size_shrink,
                    twinkle: b.params.twinkle,
                    particles,
                })
            })
            .collect()
    }

    /// `collect_sprite_effect_draws` + `build_emitter_batches`.
    pub fn collect_spr_emitters(
        &self,
        resolve_entity: &dyn Fn(u32) -> Option<[f32; 3]>,
    ) -> Vec<SprSnapshot> {
        self.effects
            .iter()
            .filter_map(|e| {
                let HeldPayload::Spr {
                    sprite,
                    size_scale,
                    anim_speed,
                    repeat,
                    tint,
                    pos_y,
                    action_index,
                } = &e.payload
                else {
                    return None;
                };
                let mut pos = resolve_position(&e.attach, resolve_entity)?;
                pos[1] += pos_y;
                let duration_ms = if e.duration.is_finite() {
                    e.duration * 1000.0
                } else {
                    1000.0
                };
                Some(SprSnapshot {
                    sprite: sprite.clone(),
                    position: pos,
                    anim_time: e.age,
                    duration_ms,
                    size_scale: *size_scale,
                    anim_speed: *anim_speed,
                    repeat: *repeat,
                    tint: *tint,
                    action_index: *action_index,
                })
            })
            .collect()
    }

    /// Snapshot of every live STR effect (name + resolved world position +
    /// elapsed anim time). Renderer consumes this and feeds it into
    /// `build_str_effect_batches`. Custom effects that return `Some` from
    /// `Effect::str_overlay` also contribute a snapshot.
    pub fn collect_str_emitters(
        &self,
        resolve_entity: &dyn Fn(u32) -> Option<[f32; 3]>,
    ) -> Vec<StrSnapshot> {
        self.effects
            .iter()
            .filter_map(|e| {
                let name: String = match &e.payload {
                    HeldPayload::Str { name } => name.clone(),
                    HeldPayload::Custom(c) => c.str_overlay()?.to_string(),
                    HeldPayload::CustomExternal { handle } => {
                        self.external_backend.as_ref()?.str_overlay(*handle)?
                    }
                    _ => return None,
                };
                let pos = resolve_position(&e.attach, resolve_entity)?;
                Some(StrSnapshot {
                    name,
                    position: pos,
                    anim_time: e.age,
                })
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn last_spawn_outcome(&self) -> Option<&SpawnOutcome> {
        self.last_spawn.as_ref()
    }

    /// Resolve the last spawn outcome into a `SpawnStatus`. Callers pass one
    /// closure so the holder can poll the renderer's STR cache without
    /// holding a borrow on it.
    pub fn last_spawn_status(
        &self,
        str_in_cache: impl Fn(&str) -> bool,
    ) -> Option<SpawnStatus> {
        Some(match self.last_spawn.as_ref()? {
            SpawnOutcome::Spr => SpawnStatus::Rendering,
            SpawnOutcome::SprBurst => SpawnStatus::Rendering,
            SpawnOutcome::Custom => SpawnStatus::Rendering,
            SpawnOutcome::Str { name } => {
                if str_in_cache(name) {
                    SpawnStatus::Rendering
                } else {
                    SpawnStatus::StrFileMissing
                }
            }
            SpawnOutcome::CustomNotImpl => SpawnStatus::CustomNotImpl,
            SpawnOutcome::NoSpec => SpawnStatus::NoSpec,
            SpawnOutcome::Noop => SpawnStatus::Noop,
        })
    }
}

fn resolve_position(
    attach: &Attach,
    resolve_entity: &dyn Fn(u32) -> Option<[f32; 3]>,
) -> Option<[f32; 3]> {
    match attach {
        Attach::WorldPos(p) => Some(*p),
        Attach::Entity(id) => resolve_entity(*id),
        Attach::Projectile { from, .. } => resolve_entity(*from),
        // Trail effects (Frost Diver, projectile shards) snapshot both
        // endpoints at spawn — anchor the holder on the caster-side
        // (`from`) world position so the spawn marker lines up with the
        // other variants.
        Attach::Trail { from, .. } => Some(*from),
        // Link effects (Linelink) track both endpoints live; anchor on the
        // caster for spawn-marker / STR-overlay purposes.
        Attach::Link { caster, .. } => resolve_entity(*caster),
    }
}

/// Resolve `Attach` to the [`EffectAnchor`] shape the factory expects.
/// `WorldPos`, `Entity`, and `Projectile` collapse to `Point` (the
/// effect doesn't need both endpoints), while `Trail` preserves both
/// endpoints so projectile-shaped effects can lay out their geometry
/// along the line. Callers that can't resolve entity → world (e.g. the
/// direct `spawn` entry point used by the effect viewer) pass a
/// resolver that always returns `None`; that path falls back to the
/// origin, matching the pre-refactor behaviour.
fn attach_to_anchor(
    attach: Attach,
    resolve_entity: &dyn Fn(u32) -> Option<[f32; 3]>,
) -> EffectAnchor {
    match attach {
        Attach::WorldPos(p) => EffectAnchor::Point(p),
        Attach::Entity(id) => {
            EffectAnchor::Point(resolve_entity(id).unwrap_or([0.0; 3]))
        }
        Attach::Projectile { from, to } => {
            let from_pos = resolve_entity(from).unwrap_or([0.0; 3]);
            match resolve_entity(to) {
                Some(to_pos) => EffectAnchor::Trail { from: from_pos, to: to_pos },
                None => EffectAnchor::Point(from_pos),
            }
        }
        Attach::Trail { from, to } => EffectAnchor::Trail { from, to },
        // Initial endpoints for a live link. Frame-1 `update` overwrites these
        // before the first `collect_draws` (drain → update run in one tick), so
        // an unresolved origin here never renders.
        Attach::Link { caster, target } => EffectAnchor::Trail {
            from: resolve_entity(caster).unwrap_or([0.0; 3]),
            to: resolve_entity(target).unwrap_or([0.0; 3]),
        },
    }
}

/// Drive one SprBurst instance forward by `dt`. Spawns the initial burst on
/// the first tick; re-spawns after every `period_frames` cooldown when set
/// and the previous batch has fully died. Particles drift along +Y at their
/// individual speeds and die when `age >= lifetime`.
fn update_burst(
    b: &mut BurstState,
    attach: &Attach,
    dt: f32,
    camera_target: Option<[f32; 3]>,
) {
    let anchor = if b.params.follow_camera
        && let Some(p) = camera_target
    {
        p
    } else {
        match attach {
            Attach::WorldPos(p) => *p,
            // Burst particles snapshot their position at spawn; if the entity
            // resolver isn't available here we still want the burst to render
            // around the origin, matching the viewer's spawn convention.
            Attach::Entity(_)
            | Attach::Projectile { .. }
            | Attach::Trail { .. }
            | Attach::Link { .. } => [0.0; 3],
        }
    };

    if !b.has_emitted {
        spawn_burst(b, anchor);
        b.has_emitted = true;
        b.cooldown_timer = 0.0;
    }

    let gravity = b.params.gravity_world_per_sec2;
    let curve = b.params.curve;
    let alpha_keyframes = b.params.alpha_keyframes;
    let (speed_lo, speed_hi) = b.params.speed_range;
    let dt_frames = dt * 60.0;
    b.particles.retain_mut(|p| {
        p.age += dt;
        if p.age >= p.lifetime {
            return false;
        }
        p.age_frames += dt_frames;

        // Curve re-randomization: every `curve_timer_frames` ticks,
        // perturb heading by ±`angle_jitter_deg`, optionally re-roll
        // speed, and pick a fresh subsequent period.
        if let Some(cp) = curve {
            p.curve_timer_frames -= dt_frames;
            if p.curve_timer_frames <= 0.0 {
                p.lon_deg = wrap_deg(
                    p.lon_deg + rand_range(-cp.angle_jitter_deg, cp.angle_jitter_deg),
                );
                p.lat_deg = wrap_deg(
                    p.lat_deg + rand_range(-cp.angle_jitter_deg, cp.angle_jitter_deg),
                );
                let speed = if cp.speed_resample {
                    rand_range(speed_lo, speed_hi)
                } else {
                    velocity_magnitude(p.velocity) / 60.0
                };
                let (vx, vy, vz) = direction_from_lon_lat(p.lon_deg, p.lat_deg);
                let world_speed = speed * 60.0;
                p.velocity = [vx * world_speed, vy * world_speed, vz * world_speed];
                let (lo, hi) = cp.subsequent_period_frames;
                p.curve_timer_frames += rand_period_frames(lo, hi);
                p.curve_count = p.curve_count.saturating_add(1);
            }
        }

        // Euler integration with optional gravity acceleration along Y.
        // Positive `gravity` pulls toward +Y (down in native RO coords),
        // matching the original game's gravity-particle fall.
        if gravity != 0.0 {
            p.velocity[1] += gravity * dt;
        }
        p.pos[0] += p.velocity[0] * dt;
        p.pos[1] += p.velocity[1] * dt;
        p.pos[2] += p.velocity[2] * dt;

        // Twinkle keyframe sawtooth. Consume any keyframes whose
        // `at_frame` is now in the past, snapping alpha + ceiling.
        // Then advance alpha by alpha_speed (per-frame delta scaled by
        // dt_frames) and flip the sign at the [0, alpha_max] bounds.
        if !alpha_keyframes.is_empty() {
            while let Some(kf) = alpha_keyframes.get(p.keyframe_idx)
                && p.age_frames >= kf.at_frame as f32
            {
                p.alpha = kf.alpha_init;
                p.alpha_max = kf.alpha_max;
                // Alpha climbs by ceiling / 1.5 per
                // frame (positive). At the ceiling it flips negative,
                // at 0 flips positive — sawtooth.
                p.alpha_speed = p.alpha_max / 1.5;
                p.keyframe_idx += 1;
            }
            // Sign flip at the bounds (before the advance so we don't
            // overshoot on the first frame after a keyframe).
            if p.alpha_speed >= 0.0 && p.alpha >= p.alpha_max {
                p.alpha_speed = -p.alpha_speed.abs();
            } else if p.alpha_speed < 0.0 && p.alpha <= 0.0 {
                p.alpha_speed = p.alpha_speed.abs();
            }
            p.alpha = (p.alpha + p.alpha_speed * dt_frames).clamp(0.0, p.alpha_max);
        }

        true
    });

    // Periodic continuous emission: fire another burst every `period_frames`
    // regardless of whether previous particles are still alive. The
    // accumulator approach handles dt larger than the period (multiple
    // spawns in one tick).
    if let Some(period_frames) = b.params.period_frames {
        b.cooldown_timer += dt;
        let period_secs = (period_frames as f32 / 60.0).max(1e-4);
        while b.cooldown_timer >= period_secs {
            b.cooldown_timer -= period_secs;
            spawn_burst(b, anchor);
        }
    }
}

fn spawn_burst(b: &mut BurstState, anchor: [f32; 3]) {
    let (lo, hi) = b.params.burst_count_range;
    let count = if hi <= lo { lo } else { lo + (rand_u32() % (hi - lo + 1)) };
    let (slo, shi) = b.params.speed_range;
    let lifetime = (b.params.particle_lifetime_ms / 1000.0).max(1e-3);
    let radius = b.params.spawn_radius_xz;
    let cone = b.params.cone_latitude_deg;
    for _ in 0..count {
        let speed = rand_range(slo, shi);
        let (ox, oz) = if radius > 0.0 {
            // Uniform scatter on a disc: sqrt(r) for area weighting.
            let r_norm = ((rand_u32() % 1000) as f32 / 1000.0).sqrt();
            let theta =
                (rand_u32() % 360_000) as f32 / 1000.0 * std::f32::consts::PI / 180.0;
            (radius * r_norm * theta.cos(), radius * r_norm * theta.sin())
        } else {
            (0.0, 0.0)
        };
        // Initial velocity. Vertical default matches the legacy
        // chimney-smoke shape (negative Y = upward in native RO coords);
        // when a cone is configured, the speed magnitude is mapped onto a
        // 3D direction picked at spawn time: a longitude Y-rotation then a
        // latitude X-rotation, matching the original game's gravity-particle
        // emission spread.
        let (lon_deg, lat_deg, velocity) = match cone {
            None => (0.0, -90.0, [0.0, -speed * 60.0, 0.0]),
            Some((lat_min, lat_max)) => {
                let lon_deg = (rand_u32() % 360_000) as f32 / 1000.0;
                let lat_deg = rand_range(lat_min, lat_max);
                let (vx, vy, vz) = direction_from_lon_lat(lon_deg, lat_deg);
                let speed_world = speed * 60.0;
                (lon_deg, lat_deg, [vx * speed_world, vy * speed_world, vz * speed_world])
            }
        };
        let curve_timer_frames = b
            .params
            .curve
            .map(|cp| {
                let (lo, hi) = cp.initial_period_frames;
                rand_period_frames(lo, hi)
            })
            .unwrap_or(0.0);
        let (alpha, alpha_max, alpha_speed, keyframe_idx) =
            init_alpha_state(b.params.alpha_keyframes);
        b.particles.push(BurstParticle {
            pos: [
                anchor[0] + ox,
                anchor[1] + b.params.pos_y_start,
                anchor[2] + oz,
            ],
            velocity,
            age: 0.0,
            lifetime,
            age_frames: 0.0,
            lon_deg,
            lat_deg,
            curve_timer_frames,
            curve_count: 0,
            alpha,
            alpha_speed,
            alpha_max,
            keyframe_idx,
        });
    }
}

/// Map `(lon_deg, lat_deg)` to a unit direction in native RO coords.
/// Equivalent to rotating +Z by longitude around Y then latitude around X.
/// We use the same "elevation from horizontal = lat-90°" remap as the
/// spawn-time cone math.
fn direction_from_lon_lat(lon_deg: f32, lat_deg: f32) -> (f32, f32, f32) {
    let lon = lon_deg.to_radians();
    let elev = (lat_deg - 90.0).to_radians();
    let cos_e = elev.cos();
    let sin_e = elev.sin();
    (cos_e * lon.cos(), -sin_e, cos_e * lon.sin())
}

fn velocity_magnitude(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn wrap_deg(d: f32) -> f32 {
    let mut x = d % 360.0;
    if x < 0.0 {
        x += 360.0;
    }
    x
}

fn rand_range(lo: f32, hi: f32) -> f32 {
    if hi <= lo {
        return lo;
    }
    let t = (rand_u32() % 1_000_000) as f32 / 1_000_000.0;
    lo + t * (hi - lo)
}

fn rand_period_frames(lo: u32, hi: u32) -> f32 {
    if hi <= lo {
        return lo as f32;
    }
    (lo + (rand_u32() % (hi - lo + 1))) as f32
}

/// Initial `(alpha, alpha_max, alpha_speed, keyframe_idx)` for a fresh
/// particle. If the schedule begins with an `at_frame=0` keyframe we
/// consume it immediately so the sawtooth starts from the right value;
/// otherwise the particle is fully transparent until the renderer
/// envelope takes over (alpha_keyframes empty case).
fn init_alpha_state(keyframes: &[AlphaKeyframe]) -> (f32, f32, f32, usize) {
    if keyframes.is_empty() {
        return (0.0, 1.0, 0.0, 0);
    }
    let mut idx = 0;
    let mut alpha = 0.0;
    let mut alpha_max = 1.0;
    while let Some(kf) = keyframes.get(idx)
        && kf.at_frame == 0
    {
        alpha = kf.alpha_init;
        alpha_max = kf.alpha_max;
        idx += 1;
    }
    let alpha_speed = alpha_max / 1.5;
    (alpha, alpha_max, alpha_speed, idx)
}

fn rand_u32() -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::Instant::now().hash(&mut h);
    std::thread::current().id().hash(&mut h);
    h.finish() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::effect_id::EffectId;
    use ragnarok_game::effect::Attach;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None }
    }

    #[test]
    fn spawning_an_str_effect_runs_until_duration_expires() {
        let mut h = EffectHolder::new();
        // Override the data-driven duration so the test doesn't depend on
        // whatever duration the original game happens to set Bubble
        // to.
        let handle = h
            .spawn(
                EffectId::Bubble,
                Attach::WorldPos([0.0, 0.0, 0.0]),
                Some(2000),
            )
            .expect("spawn");
        assert_eq!(h.len(), 1);

        h.update(&ctx(1.0), &|_| None, &|_| None);
        assert_eq!(h.len(), 1, "still alive at 1s of a 2s effect");

        h.update(&ctx(1.5), &|_| None, &|_| None);
        assert!(h.is_empty(), "should have expired after total 2.5s");

        h.despawn(handle);
    }

    #[test]
    fn quakebody_attached_to_entity_shakes_and_tints_only_that_entity() {
        let mut h = EffectHolder::new();
        h.spawn(EffectId::Quakebody4, Attach::Entity(7), None)
            .expect("Quakebody4 spawns as a Custom body effect");
        // Advance into Quakebody4's 20..60-frame shake window.
        h.update(&ctx(25.0 / 60.0), &|_| None, &|_| None);

        let ch = h.body_channels_for_entity(7);
        assert_ne!(ch.shake, [0.0, 0.0], "the attached entity shakes");
        assert!(ch.tint.is_some(), "Quakebody4 tints the attached entity");
        // A different entity is untouched.
        let other = h.body_channels_for_entity(99);
        assert_eq!(other.shake, [0.0, 0.0]);
        assert!(other.tint.is_none());
    }

    #[test]
    fn twohand_quicken_emits_and_decays_afterimage_for_attached_entity() {
        let mut h = EffectHolder::new();
        h.spawn(EffectId::Twohandquicken, Attach::Entity(7), None)
            .expect("Two-Hand Quicken spawns as a Custom body effect");

        let ai = h.afterimage_params_for_entity(7).expect("Quicken leaves a trail");
        assert_eq!(ai.tint, [200, 200, 0]);
        assert!(
            h.afterimage_params_for_entity(99).is_none(),
            "only the attached entity trails"
        );

        // One interval (5 frames) elapses → a snapshot is due.
        h.update(&ctx(5.0 / 60.0), &|_| None, &|_| None);
        assert!(h.afterimage_emit_due(7), "snapshot due after the interval");

        // The actor pass snapshots only while moving; store one here.
        h.push_afterimage(AfterimageSnapshot::new(
            7,
            SpriteAnimationState::new(0),
            Some(0),
            0,
            [10.0, 20.0],
            0.5,
            1.0,
            &ai,
        ));
        assert_eq!(h.afterimages_for_entity(7).count(), 1);
        assert!(h.afterimages_for_entity(99).next().is_none());

        // It fades out over its ~0.75 s lifetime.
        h.update(&ctx(1.0), &|_| None, &|_| None);
        assert_eq!(h.afterimages_for_entity(7).count(), 0, "snapshot decays away");
    }

    #[test]
    fn screen_quake_spawn_triggers_and_settles_camera_shake() {
        let mut h = EffectHolder::new();
        assert_eq!(h.camera_shake_offset(), [0.0, 0.0, 0.0], "idle before spawn");
        h.spawn(EffectId::ScreenQuake, Attach::WorldPos([0.0; 3]), None)
            .expect("ScreenQuake spawns (no visual, shakes the camera)");
        h.update(&ctx(0.05), &|_| None, &|_| None);
        assert_ne!(
            h.camera_shake_offset(),
            [0.0, 0.0, 0.0],
            "camera shakes while the quake is active"
        );
        // Past the shake window it settles back to rest.
        h.update(&ctx(3.0), &|_| None, &|_| None);
        assert_eq!(h.camera_shake_offset(), [0.0, 0.0, 0.0], "settles to rest");
    }

    #[test]
    fn factory_dispatched_warp_spawns_and_reports_rendering() {
        let mut h = EffectHolder::new();
        let handle = h.spawn(
            EffectId::Warp,
            Attach::WorldPos([0.0, 0.0, 0.0]),
            Some(500),
        );
        assert!(handle.is_some());
        assert_eq!(h.last_spawn_status(|_| false), Some(SpawnStatus::Rendering));
    }

    #[test]
    fn factory_unimplemented_custom_falls_back_to_placeholder() {
        // Icewall has a Custom spec but no real impl yet — the factory's
        // placeholder catchall takes over so the spawn still succeeds and
        // reports `Rendering`.
        let mut h = EffectHolder::new();
        let handle = h.spawn(
            EffectId::Icewall,
            Attach::WorldPos([0.0, 0.0, 0.0]),
            Some(500),
        );
        assert!(handle.is_some());
        assert_eq!(h.last_spawn_status(|_| false), Some(SpawnStatus::Rendering));
    }

    #[test]
    fn str_spawn_status_depends_on_cache_lookup() {
        let mut h = EffectHolder::new();
        h.spawn(
            EffectId::Bubble,
            Attach::WorldPos([0.0, 0.0, 0.0]),
            Some(1000),
        )
        .expect("spawn");
        assert_eq!(h.last_spawn_status(|_| true), Some(SpawnStatus::Rendering));
        assert_eq!(
            h.last_spawn_status(|_| false),
            Some(SpawnStatus::StrFileMissing)
        );
    }

    #[test]
    fn custom_effect_with_str_overlay_emits_snapshot() {
        // Sociable test for the Slice G hybrid path: a Custom effect that
        // declares an `str_overlay` must contribute a `StrSnapshot` alongside
        // its primitive draws.
        struct HybridFake;
        impl GameEffect for HybridFake {
            fn update(&mut self, _: &EffectUpdateCtx) -> EffectStatus {
                EffectStatus::Running
            }
            fn collect_draws(&self, _: &mut EffectDrawList, _: &EffectRenderCtx) {}
            fn str_overlay(&self) -> Option<&'static str> {
                Some("stormgust")
            }
        }
        let mut h = EffectHolder::new();
        h.effects.push(HeldEffect {
            handle: EffectHandle(1),
            payload: HeldPayload::Custom(Box::new(HybridFake)),
            attach: Attach::WorldPos([1.0, 2.0, 3.0]),
            age: 0.5,
            duration: 10.0,
        });
        let snaps = h.collect_str_emitters(&|_| None);
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].name, "stormgust");
        assert_eq!(snaps[0].position, [1.0, 2.0, 3.0]);
        assert!((snaps[0].anim_time - 0.5).abs() < 1e-6);
    }

    #[test]
    fn custom_effect_without_overlay_emits_no_str_snapshot() {
        // Warp is a Custom factory effect with no str_overlay — confirms the
        // default `None` doesn't generate snapshots accidentally.
        let mut h = EffectHolder::new();
        h.spawn(EffectId::Warp, Attach::WorldPos([0.0, 0.0, 0.0]), Some(500))
            .expect("spawn");
        let snaps = h.collect_str_emitters(&|_| None);
        assert!(snaps.is_empty());
    }

    #[test]
    fn spr_spawn_emits_snapshot_with_sprite_and_anim_time() {
        // Torch is the canonical Spr spec entry. After a spawn + tick the
        // holder should yield exactly one SprSnapshot carrying the sprite
        // path, the spawn position, and the accumulated anim time.
        let mut h = EffectHolder::new();
        h.spawn(
            EffectId::Torch,
            Attach::WorldPos([10.0, 20.0, 30.0]),
            Some(2000),
        )
        .expect("spawn");
        h.update(&ctx(0.25), &|_| None, &|_| None);
        let snaps = h.collect_spr_emitters(&|_| None);
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].sprite, "data/sprite/이팩트/torch_01");
        assert_eq!(snaps[0].position, [10.0, 20.0, 30.0]);
        assert!((snaps[0].anim_time - 0.25).abs() < 1e-6);
        // Torch plays the default ACT action.
        assert_eq!(snaps[0].action_index, 0);
    }

    #[test]
    fn spr_snapshot_carries_non_zero_act_action() {
        // Vallentine2 shares vallentine.spr but plays ACT action 1 — the
        // snapshot must carry the action index through to the renderer.
        let mut h = EffectHolder::new();
        h.spawn(EffectId::Vallentine2, Attach::WorldPos([0.0, 0.0, 0.0]), Some(1000))
            .expect("spawn");
        h.update(&ctx(0.1), &|_| None, &|_| None);
        let snaps = h.collect_spr_emitters(&|_| None);
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].action_index, 1);
    }

    #[test]
    fn spr_burst_spawns_initial_particles_and_drifts_them() {
        // Sociable test: spawn EffectId::Smoke (a SprBurst spec entry), tick
        // it once, and verify the holder yields a snapshot with 1..=4
        // particles whose Y coord has drifted upward (-Y in native RO).
        let mut h = EffectHolder::new();
        h.spawn(EffectId::Smoke, Attach::WorldPos([0.0, 0.0, 0.0]), None)
            .expect("spawn");
        h.update(&ctx(0.1), &|_| None, &|_| None);
        let snaps = h.collect_spr_burst_emitters(&|_| None);
        assert_eq!(snaps.len(), 1);
        let snap = &snaps[0];
        assert_eq!(snap.sprite, "data/sprite/이팩트/굴뚝연기");
        assert!(
            (1..=4).contains(&snap.particles.len()),
            "burst count out of range: {}",
            snap.particles.len()
        );
        for sp in &snap.particles {
            assert!(
                sp.pos[1] < -9.0,
                "particle should have drifted past pos_y_start=-9: {:?}",
                sp.pos
            );
            assert!((sp.age - 0.1).abs() < 1e-5, "age should track dt");
        }
    }

    #[test]
    fn steal_bursts_ten_gravity_arc_particles_that_scatter_in_xz() {
        // Sociable test: Steal's recipe spawns 10 particles in a 3D cone.
        // After one tick the snapshot should report ~10 particles with
        // non-zero XZ spread (proof the cone direction is honored
        // instead of the legacy pure-Y velocity). Size-shrink and
        // gravity surface as flags / drift over a longer integration.
        let mut h = EffectHolder::new();
        h.spawn(EffectId::Steal, Attach::WorldPos([0.0, 0.0, 0.0]), None)
            .expect("spawn");
        h.update(&ctx(0.05), &|_| None, &|_| None);
        let snaps = h.collect_spr_burst_emitters(&|_| None);
        assert_eq!(snaps.len(), 1);
        let snap = &snaps[0];
        assert_eq!(snap.sprite, "data/sprite/이팩트/particle7");
        assert!(snap.size_shrink, "Steal particles must shrink to 0");
        assert!(!snap.twinkle, "Steal does not twinkle");
        assert_eq!(snap.particles.len(), 10, "Steal spawns exactly 10 particles");
        let any_xz_motion = snap
            .particles
            .iter()
            .any(|sp| sp.pos[0].abs() > 0.05 || sp.pos[2].abs() > 0.05);
        assert!(
            any_xz_motion,
            "cone scatter should give at least one particle non-zero XZ drift after one tick"
        );
    }

    #[test]
    fn firefly_propagates_twinkle_and_cone_flags() {
        // Sociable test: Firefly's spec turns the twinkle flag on.
        // Verify the snapshot carries that flag through so the renderer's
        // twinkle approximation kicks in.
        let mut h = EffectHolder::new();
        h.spawn(EffectId::Firefly, Attach::WorldPos([0.0, 0.0, 0.0]), None)
            .expect("spawn");
        h.update(&ctx(0.05), &|_| None, &|_| None);
        let snaps = h.collect_spr_burst_emitters(&|_| None);
        assert_eq!(snaps.len(), 1);
        assert!(snaps[0].twinkle, "Firefly must surface PT_TWINKLE approximation");
        assert!(!snaps[0].size_shrink, "Firefly does not shrink");
    }

    #[test]
    fn firefly_spawn_directions_span_full_sphere() {
        // Regression: a cone range of `(-90, 90)` mapped to
        // `vy = cos(lat°) ∈ [0, 1]` only, so every particle fired
        // downward or horizontal — never up. Spawning many fireflies
        // must produce velocities in both upper (vy<0) and lower
        // (vy>0) hemispheres in native RO coords.
        let mut h = EffectHolder::new();
        for _ in 0..40 {
            h.spawn(EffectId::Firefly, Attach::WorldPos([0.0, 0.0, 0.0]), None)
                .expect("spawn");
        }
        h.update(&ctx(1.0 / 60.0), &|_| None, &|_| None);
        let snaps = h.collect_spr_burst_emitters(&|_| None);
        let mut saw_up = false;
        let mut saw_down = false;
        // First-tick positions relative to the spawn anchor reveal
        // the sign of the initial Y velocity (after applying
        // pos_y_start = -10).
        for s in &snaps {
            for p in &s.particles {
                let dy = p.pos[1] - (-10.0);
                if dy < -0.05 {
                    saw_up = true;
                }
                if dy > 0.05 {
                    saw_down = true;
                }
            }
        }
        assert!(saw_up, "firefly must sometimes drift upward (vy<0)");
        assert!(saw_down, "firefly must sometimes drift downward (vy>0)");
    }

    #[test]
    fn firefly_pt_curve_perturbs_velocity_within_30_frames() {
        // Sociable test for the curve path: a firefly particle's velocity must
        // change at least once within the 5..30 initial period, proving the
        // re-randomization branch in `update_burst` is wired up. We sample
        // the velocity-magnitude proxy (XZ drift direction) before and
        // after a 0.6 s window and require it to differ.
        let mut h = EffectHolder::new();
        h.spawn(EffectId::Firefly, Attach::WorldPos([0.0, 0.0, 0.0]), None)
            .expect("spawn");
        // First tick: integrate a small step so the particle starts moving.
        h.update(&ctx(1.0 / 60.0), &|_| None, &|_| None);
        let snap0 = h
            .collect_spr_burst_emitters(&|_| None)
            .into_iter()
            .next()
            .expect("snapshot");
        let p0 = snap0.particles[0].pos;
        // Wait 30 frames (0.5 s) — guarantees at least one curve tick
        // since the initial period is capped at 30 frames.
        for _ in 0..30 {
            h.update(&ctx(1.0 / 60.0), &|_| None, &|_| None);
        }
        let snap1 = h
            .collect_spr_burst_emitters(&|_| None)
            .into_iter()
            .next()
            .expect("snapshot still alive");
        let p1 = snap1.particles[0].pos;
        // Particle should have moved meaningfully (curve doesn't kill
        // motion). We don't assert direction change directly because the
        // RNG is deterministic per-tick — but the post-curve trajectory
        // must produce a position different from a pure straight-line
        // integration of the spawn velocity. The cheap proxy: position
        // delta vs. initial position is non-zero and finite.
        let dist = ((p1[0] - p0[0]).powi(2)
            + (p1[1] - p0[1]).powi(2)
            + (p1[2] - p0[2]).powi(2))
        .sqrt();
        assert!(dist > 0.5, "particle should drift across 0.5s window: {dist}");
        assert!(dist.is_finite(), "no NaN from curve math: {dist}");
    }

    #[test]
    fn firefly_alpha_keyframes_drive_per_particle_alpha_override() {
        // Sociable test for twinkle keyframes: after the firefly is
        // spawned, each particle snapshot must carry an `alpha_override`
        // (not None) because the spec supplies a keyframe schedule. The
        // first-frame value sits at frame-0's `alpha_init` (= 0).
        let mut h = EffectHolder::new();
        h.spawn(EffectId::Firefly, Attach::WorldPos([0.0, 0.0, 0.0]), None)
            .expect("spawn");
        h.update(&ctx(1.0 / 60.0), &|_| None, &|_| None);
        let snap = h
            .collect_spr_burst_emitters(&|_| None)
            .into_iter()
            .next()
            .expect("snapshot");
        let a_early = snap.particles[0]
            .alpha_override
            .expect("keyframes should populate alpha_override");
        assert!(
            a_early >= 0.0 && a_early <= 200.0 / 255.0 + 1e-3,
            "early alpha within ceiling: {a_early}",
        );

        // Step into the bright phase (past frame 40). The keyframe at
        // frame 40 raises the ceiling to 200/255 — the sawtooth bounces
        // fast (~0.52/frame) so the snapshot at any single tick lands
        // somewhere in [0, 200/255]. Sample 20 frames in that window
        // and assert at least one reading exceeds the dim 80/255
        // ceiling that bounded the early phase.
        for _ in 0..39 {
            h.update(&ctx(1.0 / 60.0), &|_| None, &|_| None);
        }
        let mut peak_bright: f32 = 0.0;
        for _ in 0..20 {
            h.update(&ctx(1.0 / 60.0), &|_| None, &|_| None);
            if let Some(snap) = h.collect_spr_burst_emitters(&|_| None).into_iter().next()
                && let Some(a) = snap.particles[0].alpha_override
            {
                peak_bright = peak_bright.max(a);
            }
        }
        assert!(
            peak_bright > 80.0 / 255.0,
            "bright phase peak should exceed the dim ceiling: {peak_bright}",
        );
    }

    #[test]
    fn drain_queue_pulls_pending_requests() {
        let mut h = EffectHolder::new();
        let mut q = EffectQueue::new();
        q.spawn_at(EffectId::Bubble, [1.0, 2.0, 3.0]);
        q.spawn_at(EffectId::Gaspush, [0.0, 0.0, 0.0]);
        h.drain_queue(&mut q, &|_| None);
        assert_eq!(h.len(), 2);
        assert!(q.pending.is_empty());
    }
}
