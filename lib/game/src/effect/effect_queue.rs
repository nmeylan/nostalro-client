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
        });
    }

    pub fn spawn_on(&mut self, effect_id: EffectId, entity_id: u32) {
        self.pending.push(SpawnRequest {
            effect_id,
            attach: Attach::Entity(entity_id),
            override_duration_ms: None,
            hit_count: None,
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
/// Effects that attach to the caster's body (shake / tint the actor sprite
/// rather than rendering at a world point). Tooling spawns these with
/// `spawn_on` so the actor pass can apply their `body_shake` / `body_tint`.
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
    )
}
