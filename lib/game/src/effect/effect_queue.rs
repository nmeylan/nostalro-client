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
}

#[derive(Default)]
pub struct EffectQueue {
    pub pending: Vec<SpawnRequest>,
}

impl EffectQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, request: SpawnRequest) {
        self.pending.push(request);
    }

    pub fn spawn_at(&mut self, effect_id: EffectId, world_pos: [f32; 3]) {
        self.pending.push(SpawnRequest {
            effect_id,
            attach: Attach::WorldPos(world_pos),
            override_duration_ms: None,
            hit_count: None,
            target_size: None,
        });
    }

    /// Spawn a point effect with a count (e.g. Chookgi's 1–5 spheres,
    /// carried by `hit_count`).
    pub fn spawn_at_with_count(&mut self, effect_id: EffectId, world_pos: [f32; 3], hit_count: u8) {
        self.pending.push(SpawnRequest {
            effect_id,
            attach: Attach::WorldPos(world_pos),
            override_duration_ms: None,
            hit_count: Some(hit_count),
            target_size: None,
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
        self.pending.push(SpawnRequest {
            effect_id,
            attach: Attach::WorldPos(world_pos),
            override_duration_ms: None,
            hit_count: None,
            target_size: Some(target_size),
        });
    }

    pub fn spawn_on(&mut self, effect_id: EffectId, entity_id: u32) {
        self.pending.push(SpawnRequest {
            effect_id,
            attach: Attach::Entity(entity_id),
            override_duration_ms: None,
            hit_count: None,
            target_size: None,
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
        self.pending.push(SpawnRequest {
            effect_id,
            attach: Attach::Trail { from, to },
            override_duration_ms: None,
            hit_count: None,
            target_size: None,
        });
    }

    /// Spawn a projectile-trail effect with a hit count (multi-bolt skills).
    pub fn spawn_trail_with_count(
        &mut self,
        effect_id: EffectId,
        from: [f32; 3],
        to: [f32; 3],
        hit_count: u8,
    ) {
        self.pending.push(SpawnRequest {
            effect_id,
            attach: Attach::Trail { from, to },
            override_duration_ms: None,
            hit_count: Some(hit_count),
            target_size: None,
        });
    }

    /// Spawn a persistent link tether between two entities.
    /// Both account ids are re-resolved to world positions every frame by the
    /// renderer holder, so the ribbon follows the linked actor as it moves.
    pub fn spawn_link(&mut self, effect_id: EffectId, caster: u32, target: u32) {
        self.pending.push(SpawnRequest {
            effect_id,
            attach: Attach::Link { caster, target },
            override_duration_ms: None,
            hit_count: None,
            target_size: None,
        });
    }

    /// Caller takes ownership of the pending list; the queue is left empty.
    /// The renderer's holder calls this each frame.
    pub fn drain(&mut self) -> Vec<SpawnRequest> {
        std::mem::take(&mut self.pending)
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
            // §1 body-render: tints / flash / spin (body_tint.rs)
            | EffectId::Redbody
            | EffectId::Transbluebody
            | EffectId::Pinkbody
            | EffectId::Linklight
            | EffectId::Magiccrasher
            | EffectId::Magiccrasher2
            | EffectId::Hitbody
            | EffectId::Falconassault
            // §1b geometry: squash / lift (squarebody.rs)
            | EffectId::Pressedbody
            | EffectId::Kickedbody
            // §1c multi-render: ghosts / halo / glow (multibody.rs)
            | EffectId::Reflectbody
            | EffectId::Assumptio
            | EffectId::Lightblade
    )
}

/// Point (non-trail) effects whose `hit_count` carries a count the impl reads
/// (Chookgi's 1–5 celebration spheres). Tooling spawns these via
/// [`EffectQueue::spawn_at_with_count`].
pub fn is_count_point_effect(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Chookgi | EffectId::Chookgi2 | EffectId::Chookgi3
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
            // heading. Stin (547) and Stin5 (624) are in-place ring swirls —
            // a point anchor keeps their spinning trail concentric.
            | EffectId::Stin2
            | EffectId::Stin3
            | EffectId::Stin4
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
            // Ice Arrow cross-texture shards stream from the caster toward the
            // target; the arrival ring lands on the target endpoint.
            | EffectId::Icearrow
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
