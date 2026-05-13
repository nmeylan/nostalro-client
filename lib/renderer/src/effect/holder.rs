//! Runtime store of currently-active effects.
//!
//! Lives in the renderer crate because the most complex variant
//! (`Box<dyn CustomEffect>`) is renderer-side. Game-side triggers push
//! [`ragnarok_game::effect::SpawnRequest`]s into a [`SpawnQueue`]; the holder
//! drains that queue each frame and constructs the runtime instances.

use ragnarok_game::effect::{
    Attach, CustomFamily, CustomFamilyParams, EffectId, EffectQueue, EffectSpec, SpawnRequest,
    effect_spec,
};

use super::custom_effect::{
    CustomEffect, CustomParams, EffectRenderCtx, EffectStatus, EffectUpdateCtx, make_custom,
};
use super::EffectDrawList;

/// Owned snapshot of a live STR effect - handed to `build_str_effect_batches`
/// by callers that need a borrow-free view of the holder's STR effects.
pub struct StrSnapshot {
    pub name: String,
    pub position: [f32; 3],
    pub anim_time: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectHandle(u64);

#[derive(Clone, Debug)]
pub enum SpawnOutcome {
    Custom {
        /// Per-family texture path the effect would like the renderer to use
        /// (may be empty if the family doesn't specify one). Used by
        /// `last_spawn_status` to surface missing-texture cases.
        texture: Option<String>,
    },
    Str {
        name: String,
    },
    Hybrid {
        name: String,
    },
    Spr,
    CustomNotImpl,
    NoSpec,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpawnStatus {
    Rendering,
    StrFileMissing,
    CustomTextureMissing,
    CustomNotImpl,
    NoSpec,
}

enum HeldPayload {
    Custom(Box<dyn CustomEffect>),
    /// Single-shot STR effect. Anim time accumulates in `age`; the render
    /// step projects via the existing `build_str_effect_batches` path.
    Str { name: String },
    /// STR animation with an additional custom-primitive overlay.
    Hybrid {
        name: String,
        custom: Box<dyn CustomEffect>,
    },
    /// Looping single SPR billboard (torches, simple ambient).
    Spr {
        #[allow(dead_code)] // wired in Phase D
        sprite: String,
    },
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
}

impl EffectHolder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn directly by `EffectId`. Used by the effect viewer and any
    /// caller that has resolved the id itself.
    pub fn spawn(
        &mut self,
        effect_id: EffectId,
        attach: Attach,
        override_duration_ms: Option<u32>,
        tint: Option<[f32; 4]>,
    ) -> Option<EffectHandle> {
        let Some(spec) = effect_spec(effect_id) else {
            self.last_spawn = Some(SpawnOutcome::NoSpec);
            return None;
        };
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
            EffectSpec::Custom { family, params: family_params, .. } => {
                let params = build_params(*family, &attach, tint);
                match make_custom(*family, &params, family_params) {
                    Some(c) => {
                        self.last_spawn = Some(SpawnOutcome::Custom {
                            texture: family_texture(family_params),
                        });
                        HeldPayload::Custom(c)
                    }
                    None => {
                        self.last_spawn = Some(SpawnOutcome::CustomNotImpl);
                        tracing::debug!(
                            "EffectHolder: no implementation for family {:?} (effect {:?})",
                            family,
                            effect_id
                        );
                        return None;
                    }
                }
            }
            EffectSpec::StrHybrid { file, family, .. } => {
                let params = build_params(*family, &attach, tint);
                match make_custom(*family, &params, &CustomFamilyParams::Default) {
                    Some(c) => {
                        self.last_spawn = Some(SpawnOutcome::Hybrid {
                            name: (*file).to_string(),
                        });
                        HeldPayload::Hybrid {
                            name: (*file).to_string(),
                            custom: c,
                        }
                    }
                    None => {
                        self.last_spawn = Some(SpawnOutcome::CustomNotImpl);
                        tracing::debug!(
                            "EffectHolder: no impl for hybrid family {:?} (effect {:?})",
                            family,
                            effect_id
                        );
                        return None;
                    }
                }
            }
        };

        let duration_ms = override_duration_ms.unwrap_or_else(|| match spec {
            EffectSpec::Str { duration_ms, .. }
            | EffectSpec::Spr { duration_ms, .. }
            | EffectSpec::Custom { duration_ms, .. }
            | EffectSpec::StrHybrid { duration_ms, .. } => duration_ms,
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
                tint,
            } = req;
            self.spawn(effect_id, attach, override_duration_ms, tint);
        }
    }

    pub fn despawn(&mut self, handle: EffectHandle) {
        self.effects.retain(|e| e.handle != handle);
    }

    /// Drop every live effect. Used by the effect viewer when the user
    /// cycles to a new picker entry so old (persistent) effects don't
    /// accumulate.
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    pub fn update(&mut self, ctx: &EffectUpdateCtx) {
        let dt = ctx.dt;
        self.effects.retain_mut(|e| {
            e.age += dt;
            if e.age >= e.duration {
                return false;
            }
            match &mut e.payload {
                HeldPayload::Custom(c) => c.update(ctx) == EffectStatus::Running,
                HeldPayload::Hybrid { custom, .. } => custom.update(ctx) == EffectStatus::Running,
                HeldPayload::Str { .. } => true,
                HeldPayload::Spr { .. } => true,
            }
        });
    }

    /// Append primitive draws for live custom and hybrid effects. STR/Spr
    /// collection is wired in Phase D when the renderer's render-pass
    /// plumbing lands.
    pub fn collect_custom_draws(
        &self,
        out: &mut EffectDrawList,
        ctx: &EffectRenderCtx,
    ) {
        for e in &self.effects {
            match &e.payload {
                HeldPayload::Custom(c) => c.collect_draws(out, ctx),
                HeldPayload::Hybrid { custom, .. } => custom.collect_draws(out, ctx),
                _ => {}
            }
        }
    }

    /// Snapshot of every live STR effect (name + resolved world position +
    /// elapsed anim time). Renderer consumes this and feeds it into
    /// `build_str_effect_batches`. Hybrid effects also emit a snapshot for
    /// their STR layer.
    pub fn collect_str_emitters(
        &self,
        resolve_entity: &dyn Fn(u32) -> Option<[f32; 3]>,
    ) -> Vec<StrSnapshot> {
        self.effects
            .iter()
            .filter_map(|e| {
                let name = match &e.payload {
                    HeldPayload::Str { name } => name,
                    HeldPayload::Hybrid { name, .. } => name,
                    _ => return None,
                };
                let pos = resolve_position(&e.attach, resolve_entity)?;
                Some(StrSnapshot {
                    name: name.clone(),
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

    /// Resolve the last spawn outcome into a `SpawnStatus`. Callers pass two
    /// closures so the holder can poll the renderer's caches without holding
    /// a borrow on them.
    pub fn last_spawn_status(
        &self,
        str_in_cache: impl Fn(&str) -> bool,
        texture_in_cache: impl Fn(&str) -> bool,
    ) -> Option<SpawnStatus> {
        Some(match self.last_spawn.as_ref()? {
            SpawnOutcome::Spr => SpawnStatus::Rendering,
            SpawnOutcome::Custom { texture } => match texture.as_deref() {
                Some(name) if !name.is_empty() && !texture_in_cache(name) => {
                    SpawnStatus::CustomTextureMissing
                }
                _ => SpawnStatus::Rendering,
            },
            SpawnOutcome::Str { name } | SpawnOutcome::Hybrid { name } => {
                if str_in_cache(name) {
                    SpawnStatus::Rendering
                } else {
                    SpawnStatus::StrFileMissing
                }
            }
            SpawnOutcome::CustomNotImpl => SpawnStatus::CustomNotImpl,
            SpawnOutcome::NoSpec => SpawnStatus::NoSpec,
        })
    }
}

/// GRF path the family params would like rendered (None if the family has no
/// per-effect texture). Mirrors `effect_texture_paths` on the game side.
fn family_texture(params: &CustomFamilyParams) -> Option<String> {
    match params {
        CustomFamilyParams::GroundRing(p) if !p.texture.is_empty() => {
            Some(format!("data/texture/effect/{}", p.texture))
        }
        _ => None,
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

fn build_params(
    _family: CustomFamily,
    attach: &Attach,
    tint: Option<[f32; 4]>,
) -> CustomParams {
    let (world_pos, target_pos) = match attach {
        Attach::WorldPos(p) => (*p, None),
        Attach::Entity(_) => ([0.0; 3], None),
        Attach::Projectile { .. } => ([0.0; 3], None),
    };
    CustomParams {
        world_pos,
        target_pos,
        texture: None,
        tint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::effect::{Attach, EffectId};

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { dt }
    }

    #[test]
    fn spawning_an_str_effect_runs_until_duration_expires() {
        let mut h = EffectHolder::new();
        // Override the data-driven duration so the test doesn't depend on
        // whatever the original's duration table happens to set Bubble to.
        let handle = h
            .spawn(
                EffectId::Bubble,
                Attach::WorldPos([0.0, 0.0, 0.0]),
                Some(2000),
                None,
            )
            .expect("spawn");
        assert_eq!(h.len(), 1);

        h.update(&ctx(1.0));
        assert_eq!(h.len(), 1, "still alive at 1s of a 2s effect");

        h.update(&ctx(1.5));
        assert!(h.is_empty(), "should have expired after total 2.5s");

        // Despawning a stale handle is harmless.
        h.despawn(handle);
    }

    #[test]
    fn custom_effect_with_impl_spawns_and_reports_rendering() {
        let mut h = EffectHolder::new();
        let handle = h.spawn(
            EffectId::Icewall,
            Attach::WorldPos([0.0, 0.0, 0.0]),
            Some(500),
            None,
        );
        assert!(handle.is_some());
        assert_eq!(
            h.last_spawn_status(|_| false, |_| false),
            Some(SpawnStatus::Rendering)
        );
    }

    #[test]
    fn str_spawn_status_depends_on_cache_lookup() {
        let mut h = EffectHolder::new();
        h.spawn(
            EffectId::Bubble,
            Attach::WorldPos([0.0, 0.0, 0.0]),
            Some(1000),
            None,
        )
        .expect("spawn");
        assert_eq!(
            h.last_spawn_status(|_| true, |_| false),
            Some(SpawnStatus::Rendering)
        );
        assert_eq!(
            h.last_spawn_status(|_| false, |_| false),
            Some(SpawnStatus::StrFileMissing)
        );
    }

    #[test]
    fn custom_with_texture_reports_missing_when_not_in_cache() {
        let mut h = EffectHolder::new();
        // EF_WARP has a hand-curated GroundRing texture override.
        h.spawn(
            EffectId::Warp,
            Attach::WorldPos([0.0, 0.0, 0.0]),
            Some(1000),
            None,
        )
        .expect("spawn");
        assert_eq!(
            h.last_spawn_status(|_| false, |_| false),
            Some(SpawnStatus::CustomTextureMissing)
        );
        assert_eq!(
            h.last_spawn_status(|_| false, |_| true),
            Some(SpawnStatus::Rendering)
        );
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
