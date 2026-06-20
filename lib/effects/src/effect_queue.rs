//! Game-to-renderer effect spawn channel.
//!
//! Game-side triggers (skill packets, level-up, refining, item use, map
//! ambients, status changes) push [`SpawnRequest`]s into this queue. The
//! renderer's `EffectHolder` drains the queue once per frame, looks each
//! id up in `effect_spec()`, and constructs the actual effect.
//!
//! Keeping spawns one-way via a queue means the game crate never needs to
//! know about renderer types (wgpu, etc).

use models::enums::effect_id::EffectId;
use super::spec::Attach;

/// One request to spawn a single effect.
#[derive(Clone, Debug)]
pub struct SpawnRequest {
    pub effect_id: EffectId,
    pub attach: Attach,
    /// Override the duration from `effect_spec()` (e.g. server-driven
    /// Ice Wall lifetime). `None` means use the default.
    pub override_duration_ms: Option<u32>,
    /// Number of hits for multi-bolt skills (Soul Strike, Fire Bolt, …).
    /// Passed through to the factory so the effect can spawn the right
    /// number of projectiles.
    pub hit_count: Option<u8>,
    /// Target sprite size in world units `[width, height]`. Effects that
    /// size themselves to the targeted actor (lock-on reticle) read this;
    /// `None` falls back to a fixed size.
    pub target_size: Option<[f32; 2]>,
    /// Caller-chosen owner key for despawn-by-key (a ground unit's `aid`, or a
    /// buffed entity's `gid`). `None` = fire-and-forget; the effect can only
    /// end by its own duration. Set it for persistent effects the game must be
    /// able to cancel *before* their duration: ground units removed by
    /// `ZC_SKILL_DISAPPEAR`, buffs cleared by a status-off packet. See
    /// [`EffectQueue::despawn`].
    pub key: Option<u32>,
    /// Per-spawn size multiplier applied on top of the spec's own size
    /// (Spr/SprBurst). `None` = use the spec size unchanged. RSW ambient
    /// emitters carry a per-emitter `param[0]` scale through here.
    pub size_scale: Option<f32>,
}

impl SpawnRequest {
    pub fn new(effect_id: EffectId, attach: Attach) -> Self {
        Self {
            effect_id,
            attach,
            override_duration_ms: None,
            hit_count: None,
            target_size: None,
            key: None,
            size_scale: None,
        }
    }
}

#[derive(Default)]
pub struct EffectQueue {
    pub pending: Vec<SpawnRequest>,
    /// Owner keys whose live effects should be despawned this frame. The
    /// holder drains this alongside `pending` and drops every effect spawned
    /// with a matching [`SpawnRequest::key`].
    pub despawns: Vec<u32>,
}

impl EffectQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, request: SpawnRequest) {
        self.pending.push(request);
    }

    pub fn spawn_at(&mut self, effect_id: EffectId, world_pos: [f32; 3]) {
        self.push(SpawnRequest::new(effect_id, Attach::WorldPos(world_pos)));
    }

    /// Spawn a point effect with a count (e.g. Chookgi's 1–5 spheres,
    /// carried by `hit_count`).
    pub fn spawn_at_with_count(&mut self, effect_id: EffectId, world_pos: [f32; 3], hit_count: u8) {
        self.push(SpawnRequest {
            hit_count: Some(hit_count),
            ..SpawnRequest::new(effect_id, Attach::WorldPos(world_pos))
        });
    }

    /// Spawn a point effect sized to the target sprite (lock-on reticle).
    /// `target_size` is the targeted actor's `[width, height]` in world units.
    pub fn spawn_at_with_size(
        &mut self,
        effect_id: EffectId,
        world_pos: [f32; 3],
        target_size: [f32; 2],
    ) {
        self.push(SpawnRequest {
            target_size: Some(target_size),
            ..SpawnRequest::new(effect_id, Attach::WorldPos(world_pos))
        });
    }

    pub fn spawn_on(&mut self, effect_id: EffectId, entity_id: u32) {
        self.push(SpawnRequest::new(effect_id, Attach::Entity(entity_id)));
    }

    /// Spawn an entity-attached effect whose lifetime is fixed to
    /// `duration_ms` instead of the spec default — the begin-spell cast circle
    /// uses this so its duration tracks the skill's cast time.
    pub fn spawn_on_for(&mut self, effect_id: EffectId, entity_id: u32, duration_ms: u32) {
        self.push(SpawnRequest {
            override_duration_ms: Some(duration_ms),
            ..SpawnRequest::new(effect_id, Attach::Entity(entity_id))
        });
    }

    /// Spawn an entity-attached effect with a count (the `ZC_NOTIFY_EFFECT3`
    /// extra datum maps onto `hit_count`).
    pub fn spawn_on_with_count(&mut self, effect_id: EffectId, entity_id: u32, hit_count: u8) {
        self.push(SpawnRequest {
            hit_count: Some(hit_count),
            ..SpawnRequest::new(effect_id, Attach::Entity(entity_id))
        });
    }

    /// Spawn a persistent, entity-attached effect tagged with an owner `key`
    /// so it can later be removed with [`EffectQueue::despawn`]. The canonical
    /// caller is a status buff keyed by the bearer's `gid` (cleared when the
    /// status-off packet arrives or its duration expires).
    pub fn spawn_on_keyed(&mut self, effect_id: EffectId, entity_id: u32, key: u32) {
        self.push(SpawnRequest {
            key: Some(key),
            ..SpawnRequest::new(effect_id, Attach::Entity(entity_id))
        });
    }

    /// Spawn a persistent, fixed-position effect tagged with an owner `key`.
    /// The canonical caller is a ground-skill unit keyed by its `aid`
    /// (removed when its `ZC_SKILL_DISAPPEAR` arrives).
    pub fn spawn_at_keyed(&mut self, effect_id: EffectId, world_pos: [f32; 3], key: u32) {
        self.push(SpawnRequest {
            key: Some(key),
            ..SpawnRequest::new(effect_id, Attach::WorldPos(world_pos))
        });
    }

    /// Keyed fixed-position spawn carrying a per-spawn size multiplier — the
    /// canonical caller is an RSW ambient emitter passing its `param[0]` scale.
    pub fn spawn_at_keyed_scaled(
        &mut self,
        effect_id: EffectId,
        world_pos: [f32; 3],
        key: u32,
        size_scale: f32,
    ) {
        self.push(SpawnRequest {
            key: Some(key),
            size_scale: Some(size_scale),
            ..SpawnRequest::new(effect_id, Attach::WorldPos(world_pos))
        });
    }

    /// Spawn a projectile-trail effect whose primitives lay between two
    /// pre-resolved world positions (caster → target). Frost Diver is
    /// the canonical caller; future arrow-shower style effects route
    /// through here as well.
    pub fn spawn_trail(
        &mut self,
        effect_id: EffectId,
        from: [f32; 3],
        to: [f32; 3],
    ) {
        self.push(SpawnRequest::new(effect_id, Attach::Trail { from, to }));
    }

    /// Spawn a projectile-trail effect with a hit count (multi-bolt skills).
    pub fn spawn_trail_with_count(
        &mut self,
        effect_id: EffectId,
        from: [f32; 3],
        to: [f32; 3],
        hit_count: u8,
    ) {
        self.push(SpawnRequest {
            hit_count: Some(hit_count),
            ..SpawnRequest::new(effect_id, Attach::Trail { from, to })
        });
    }

    /// Spawn a persistent link tether between two entities.
    /// Both account ids are re-resolved to world positions every frame by the
    /// renderer holder, so the ribbon follows the linked actor as it moves.
    pub fn spawn_link(&mut self, effect_id: EffectId, caster: u32, target: u32) {
        self.push(SpawnRequest::new(effect_id, Attach::Link { caster, target }));
    }

    /// Request that every live effect spawned with `key` be despawned this
    /// frame (the holder drops them when it drains the queue). Safe to call for
    /// a key with no live effects — it's a no-op.
    pub fn despawn(&mut self, key: u32) {
        self.despawns.push(key);
    }

    /// Caller takes ownership of the pending spawn list; the queue is left
    /// empty. The renderer's holder calls this each frame.
    pub fn drain(&mut self) -> Vec<SpawnRequest> {
        std::mem::take(&mut self.pending)
    }

    /// Caller takes ownership of the pending despawn keys; the list is left
    /// empty. The holder drains this alongside [`EffectQueue::drain`].
    pub fn drain_despawns(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.despawns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_spawn_and_despawn_channel_round_trip() {
        // B1.1c game-side API: a persistent effect is spawned tagged with an
        // owner key, and a later despawn request rides a separate channel the
        // holder drains alongside spawns.
        let mut q = EffectQueue::new();
        q.spawn_on_keyed(EffectId::Blessing, 42, 7);

        let pending = q.drain();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].attach, Attach::Entity(42));
        assert_eq!(pending[0].key, Some(7), "the owner key rides the spawn request");
        assert!(q.drain().is_empty(), "drain emptied the pending list");

        q.despawn(7);
        assert_eq!(q.drain_despawns(), vec![7]);
        assert!(q.drain_despawns().is_empty(), "despawn channel emptied");
    }
}

/// `true` if `id`'s custom-effect impl reads both endpoints of an
/// `Attach::Trail`. Callers spawning trail-shaped effects (Frost
/// Diver and any future projectile families) should route through
/// [`EffectQueue::spawn_trail`]; ones that don't will see the
/// cluster-mode fallback. Used by the effect viewer to construct
/// a demo trail for IDs that need one.
/// Effects that attach to the target's body (shake / tint / scale / spin the
/// actor sprite, optionally rendering primitives anchored on it). Tooling
/// spawns these with `spawn_on` so the actor pass can apply their
/// `body_shake` / `body_tint` / `body_scale` / `body_yaw`.
pub fn body_attached(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Quakebody
            | EffectId::Quakebody2
            | EffectId::Quakebody3
            | EffectId::Quakebody4
            | EffectId::Twohandquicken
            | EffectId::Spearquicken
            | EffectId::Lkconcentration
            | EffectId::Giantbody
            | EffectId::Giantbody2
            | EffectId::Babybody
            | EffectId::Babybody2
            | EffectId::BabybodyBack
            | EffectId::Jumpkick
            | EffectId::Jumpbody
            | EffectId::Landbody
            | EffectId::Spinedbody
            | EffectId::Spinedbody2
            | EffectId::Asurabody
            | EffectId::TaeReady
            | EffectId::Ef4waybody
            | EffectId::Hitline2
            | EffectId::Stormkick
            | EffectId::Stormkick1
            | EffectId::Stormkick2
            | EffectId::Stormkick3
            | EffectId::Stormkick6
            | EffectId::Stormkick7
            | EffectId::Redbody
            | EffectId::Transbluebody
            | EffectId::Pinkbody
            | EffectId::Linklight
            | EffectId::Magiccrasher
            | EffectId::Magiccrasher2
            | EffectId::Hitbody
            | EffectId::Falconassault
            | EffectId::Chemicalbody
            | EffectId::Piercebody
            | EffectId::Memorize
            | EffectId::Doublecastbody
            | EffectId::Greenbody
            | EffectId::Shrink
            | EffectId::Bluebody
            | EffectId::Redlightbody
            | EffectId::RedHit
            | EffectId::BlueHit
            | EffectId::Bunsinjyutsu
            | EffectId::MadnessBlue
            | EffectId::MadnessRed
            | EffectId::Undeadbody
            | EffectId::Pressedbody
            | EffectId::Kickedbody
            | EffectId::Reflectbody
            | EffectId::Assumptio
            | EffectId::Lightblade
            | EffectId::Damage1
            | EffectId::Damage12
            | EffectId::Damage13
            | EffectId::GreenNumber
            | EffectId::BlueNumber
            | EffectId::RedNumber
            | EffectId::PurpleNumber
            | EffectId::BlackNumber
            | EffectId::WhiteNumber
            | EffectId::YellowNumber
            | EffectId::PinkNumber
    )
}

/// Point (non-trail) effects whose `hit_count` carries a count the impl reads
/// (Chookgi's 1–5 celebration spheres). Tooling spawns these via
/// [`EffectQueue::spawn_at_with_count`].
pub fn is_count_point_effect(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Chookgi | EffectId::Chookgi2 | EffectId::Chookgi3
        // Cold Bolt / Fire Bolt rain a count of bolts onto the target; the
        // bolt count rides on `hit_count`.
        | EffectId::Icearrow | EffectId::Firearrow
    )
}

pub fn is_trail_effect(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Frostdiver
            | EffectId::Grimtooth
            | EffectId::Icewall
            | EffectId::Fireball
            | EffectId::Soulstrike
            | EffectId::Soulstrike2
            | EffectId::Soulbreaker
            | EffectId::Yufitel
            | EffectId::Pierce
            // Hit family — flared cone aims along the caster→target heading.
            | EffectId::Hit1
            | EffectId::Hit3
            | EffectId::Hit4
            | EffectId::Sonicblowhit
            | EffectId::Waterball
            | EffectId::Fireivy
            | EffectId::Foot
            | EffectId::Foot2
            | EffectId::Foot3
            | EffectId::Foot4
            | EffectId::Foot5
            | EffectId::Foot6
            | EffectId::Bowlingbash
            | EffectId::Dragonsmoke
            | EffectId::Throwitem
            | EffectId::Throwitem2
            | EffectId::Throwitem3
            | EffectId::Throwitem4
            | EffectId::Throwitem5
            | EffectId::Throwitem6
            | EffectId::Throwitem7
            | EffectId::Throwitem8
            | EffectId::Throwitem9
            | EffectId::Throwitem10
            // Chemical streak family aims along caster→target.
            | EffectId::Chemical2
            | EffectId::Chemical2dash
            | EffectId::Chemical3
            | EffectId::Chemical4
            | EffectId::Smatk1
            | EffectId::Smatk2
            | EffectId::Smatk3
            | EffectId::Smatk4
            // STIN/SMA wind streaks that travel/home along the caster→target
            // heading. Stin/Stin5 fly straight toward the target; Stin2/Stin4
            // launch perpendicular then seek the target's position and cross it.
            | EffectId::Stin
            | EffectId::Stin2
            | EffectId::Stin3
            | EffectId::Stin4
            | EffectId::Stin5
            | EffectId::Sma
            // Teihit2/Backstap dart sprays erupt from the target along the
            // caster→target heading.
            | EffectId::Teihit2
            | EffectId::Backstap
            // TANJI spirit-sphere projectiles fly along caster→target.
            | EffectId::Tanji
            | EffectId::Tanji2
            | EffectId::Alattack1
            | EffectId::Alattack2
            | EffectId::Alattack3
            | EffectId::Alattack4
            // Shield boomerangs: ranged attacks. 249/494 fly out and home back;
            // 520 bursts at the target. All need the caster→target endpoints.
            | EffectId::Shieldboomerang
            | EffectId::Shieldboomerang2
            | EffectId::Shieldboomerang3
            // Slim potion throws and Pressure land on the target (ranged); need
            // the target endpoint as the impact point.
            | EffectId::Slim
            | EffectId::Slim2
            | EffectId::Slim3
            | EffectId::Pressure
            // TripleAttack streaks fly from the caster toward the target.
            | EffectId::Tripleattack
            | EffectId::Tripleattack2
            | EffectId::Tripleattack3
            // Spear Boomerang spears + Waterball2 spline both fly caster→target.
            | EffectId::Spearbmr
            | EffectId::Waterball2
            // Wink / Fvoice emotes pick their fly-off action from the
            // caster→target heading, so they need the trail endpoints (they
            // collapse to a point and camera-only direction when self-cast).
            | EffectId::Wink
            | EffectId::Fvoice
    )
}

/// `true` for the Soul Linker tether family: a persistent
/// ribbon between the caster and a second, independently-moving actor. In game
/// these spawn via [`EffectQueue::spawn_link`] so both endpoints track live;
/// the effect viewer treats them like trail effects, anchoring the partner end
/// to the green-cross fake entity.
pub fn is_link_effect(id: EffectId) -> bool {
    matches!(id, EffectId::Linelink | EffectId::Linelink2 | EffectId::Linelink3)
}
