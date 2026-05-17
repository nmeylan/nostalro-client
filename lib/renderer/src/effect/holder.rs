//! Runtime store of currently-active effects.
//!
//! Game-side triggers push [`ragnarok_game::effect::SpawnRequest`]s into a
//! [`EffectQueue`]; the holder drains that queue each frame and constructs
//! the runtime instances. `EffectSpec::Custom` dispatches through
//! [`ragnarok_game::effect::make_effect`] to a `Box<dyn Effect>` living in
//! the game crate; tooling can swap that for an [`ExternalCustomBackend`]
//! to load effects from a hot-reload cdylib.

use std::sync::Arc;

use models::enums::effect_id::EffectId;
use ragnarok_game::effect::{
    Attach, Effect as GameEffect, EffectDrawList, EffectQueue, EffectRenderCtx,
    EffectSpec, EffectStatus, EffectUpdateCtx, SpawnRequest, effect_spec, make_effect,
};

/// Pluggable backend for custom-effect dispatch. The default path links the
/// game crate's `make_effect` statically; tooling that needs hot reload
/// (the effect viewer) supplies a wrapper around its cdylib so effect
/// implementations live behind the dlsym boundary and can be swapped at
/// runtime. Drop semantics: `drop_handle` must release the cdylib-owned
/// `Box<dyn Effect>`; `drop_all` is called by [`EffectHolder::clear`] and
/// by tooling just before a reload.
pub trait ExternalCustomBackend: Send + Sync {
    fn spawn(&self, effect_id: u16, world_pos: [f32; 3]) -> u64;
    /// Returns `true` while the effect is still running, `false` once it
    /// has signalled death.
    fn update(&self, handle: u64, dt: f32) -> bool;
    fn collect(&self, handle: u64, ctx: &EffectRenderCtx, out: &mut EffectDrawList);
    /// Optional STR overlay name for this effect instance. Default `None`;
    /// hot-reload backends can probe their cdylib for the current overlay.
    fn str_overlay(&self, _handle: u64) -> Option<String> {
        None
    }
    fn drop_handle(&self, handle: u64);
    fn drop_all(&self);
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectHandle(u64);

#[derive(Clone, Debug)]
pub enum SpawnOutcome {
    Custom,
    Str { name: String },
    Spr,
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
    /// Looping single SPR billboard (torches, simple ambient).
    Spr { sprite: String },
}

struct HeldEffect {
    handle: EffectHandle,
    payload: HeldPayload,
    attach: Attach,
    age: f32,
    duration: f32,
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
        let Some(spec) = effect_spec(effect_id) else {
            self.last_spawn = Some(SpawnOutcome::NoSpec);
            return None;
        };
        if matches!(spec, EffectSpec::Noop) {
            self.last_spawn = Some(SpawnOutcome::Noop);
            return None;
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
            EffectSpec::Spr { sprite, .. } => {
                self.last_spawn = Some(SpawnOutcome::Spr);
                HeldPayload::Spr {
                    sprite: (*sprite).to_string(),
                }
            }
            EffectSpec::Noop => unreachable!("Noop handled above"),
            EffectSpec::Custom { .. } => {
                if let Some(backend) = &self.external_backend {
                    let world_pos = match attach {
                        Attach::WorldPos(p) => p,
                        Attach::Entity(_) | Attach::Projectile { .. } => [0.0; 3],
                    };
                    let handle = backend.spawn(effect_id as u16, world_pos);
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
                    match make_effect(effect_id, attach) {
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

    /// Drain a game-side queue and spawn each request.
    pub fn drain_queue(&mut self, queue: &mut EffectQueue) {
        for req in queue.drain() {
            let SpawnRequest {
                effect_id,
                attach,
                override_duration_ms,
            } = req;
            self.spawn(effect_id, attach, override_duration_ms);
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

    pub fn update(&mut self, ctx: &EffectUpdateCtx) {
        let dt = ctx.delta;
        let backend = self.external_backend.clone();
        self.effects.retain_mut(|e| {
            e.age += dt;
            let expired = e.age >= e.duration;
            let alive = match &mut e.payload {
                HeldPayload::Custom(c) => c.update(ctx) == EffectStatus::Running,
                HeldPayload::CustomExternal { handle } => backend
                    .as_ref()
                    .map(|b| b.update(*handle, dt))
                    .unwrap_or(false),
                HeldPayload::Str { .. } => true,
                HeldPayload::Spr { .. } => true,
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
    /// `collect_sprite_effect_draws` + `build_emitter_batches`.
    pub fn collect_spr_emitters(
        &self,
        resolve_entity: &dyn Fn(u32) -> Option<[f32; 3]>,
    ) -> Vec<SprSnapshot> {
        self.effects
            .iter()
            .filter_map(|e| {
                let HeldPayload::Spr { sprite } = &e.payload else {
                    return None;
                };
                let pos = resolve_position(&e.attach, resolve_entity)?;
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::effect_id::EffectId;
    use ragnarok_game::effect::Attach;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt }
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

        h.update(&ctx(1.0));
        assert_eq!(h.len(), 1, "still alive at 1s of a 2s effect");

        h.update(&ctx(1.5));
        assert!(h.is_empty(), "should have expired after total 2.5s");

        h.despawn(handle);
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
        h.update(&ctx(0.25));
        let snaps = h.collect_spr_emitters(&|_| None);
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].sprite, "data/sprite/이팩트/torch_01");
        assert_eq!(snaps[0].position, [10.0, 20.0, 30.0]);
        assert!((snaps[0].anim_time - 0.25).abs() < 1e-6);
    }

    #[test]
    fn drain_queue_pulls_pending_requests() {
        let mut h = EffectHolder::new();
        let mut q = EffectQueue::new();
        q.spawn_at(EffectId::Bubble, [1.0, 2.0, 3.0]);
        q.spawn_at(EffectId::Gaspush, [0.0, 0.0, 0.0]);
        h.drain_queue(&mut q);
        assert_eq!(h.len(), 2);
        assert!(q.pending.is_empty());
    }
}
