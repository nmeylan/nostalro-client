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
pub fn is_trail_effect(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Frostdiver
            | EffectId::Grimtooth
            | EffectId::Icewall
            | EffectId::Fireball
            | EffectId::Soulstrike
            | EffectId::Yufitel
            | EffectId::Pierce
            | EffectId::Sonicblowhit
            | EffectId::Waterball
            | EffectId::Fireivy
    )
}
