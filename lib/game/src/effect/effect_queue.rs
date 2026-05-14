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
        });
    }

    pub fn spawn_on(&mut self, effect_id: EffectId, entity_id: u32) {
        self.pending.push(SpawnRequest {
            effect_id,
            attach: Attach::Entity(entity_id),
            override_duration_ms: None,
        });
    }

    /// Caller takes ownership of the pending list; the queue is left empty.
    /// The renderer's holder calls this each frame.
    pub fn drain(&mut self) -> Vec<SpawnRequest> {
        std::mem::take(&mut self.pending)
    }
}
