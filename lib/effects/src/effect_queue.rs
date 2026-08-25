use super::spec::Attach;
use models::enums::effect_id::EffectId;

#[derive(Clone, Debug)]
pub struct SpawnRequest {
    pub effect_id: EffectId,
    pub attach: Attach,
    pub override_duration_ms: Option<u32>,
    pub hit_count: Option<u8>,
    pub target_size: Option<[f32; 2]>,
    pub key: Option<u32>,
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

pub struct EffectQueue {
    pub pending: Vec<SpawnRequest>,
    pub despawns: Vec<u32>,
    enabled: bool,
}

impl Default for EffectQueue {
    fn default() -> Self {
        Self {
            pending: Vec::new(),
            despawns: Vec::new(),
            enabled: true,
        }
    }
}

impl EffectQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// The `/effect` toggle: when disabled, one-shot requests (no key) are
    /// dropped while keyed requests (auras, status buffs, ailment overlays)
    /// keep flowing so their key-map lifecycles never observe a lost spawn.
    pub fn set_effects_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn push(&mut self, request: SpawnRequest) {
        if !self.enabled && request.key.is_none() {
            return;
        }
        self.pending.push(request);
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.despawns.clear();
    }

    pub fn spawn_at(&mut self, effect_id: EffectId, world_pos: [f32; 3]) {
        self.push(SpawnRequest::new(effect_id, Attach::WorldPos(world_pos)));
    }

    pub fn spawn_at_with_count(&mut self, effect_id: EffectId, world_pos: [f32; 3], hit_count: u8) {
        self.push(SpawnRequest {
            hit_count: Some(hit_count),
            ..SpawnRequest::new(effect_id, Attach::WorldPos(world_pos))
        });
    }

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

    pub fn spawn_on_for(&mut self, effect_id: EffectId, entity_id: u32, duration_ms: u32) {
        self.push(SpawnRequest {
            override_duration_ms: Some(duration_ms),
            ..SpawnRequest::new(effect_id, Attach::Entity(entity_id))
        });
    }

    pub fn spawn_on_with_count(&mut self, effect_id: EffectId, entity_id: u32, hit_count: u8) {
        self.push(SpawnRequest {
            hit_count: Some(hit_count),
            ..SpawnRequest::new(effect_id, Attach::Entity(entity_id))
        });
    }

    pub fn spawn_on_keyed(&mut self, effect_id: EffectId, entity_id: u32, key: u32) {
        self.push(SpawnRequest {
            key: Some(key),
            ..SpawnRequest::new(effect_id, Attach::Entity(entity_id))
        });
    }

    pub fn spawn_on_keyed_with_count(
        &mut self,
        effect_id: EffectId,
        entity_id: u32,
        key: u32,
        hit_count: u8,
    ) {
        self.push(SpawnRequest {
            key: Some(key),
            hit_count: Some(hit_count),
            ..SpawnRequest::new(effect_id, Attach::Entity(entity_id))
        });
    }

    pub fn spawn_on_keyed_for(
        &mut self,
        effect_id: EffectId,
        entity_id: u32,
        key: u32,
        duration_ms: u32,
    ) {
        self.spawn_on_keyed_for_with_count(effect_id, entity_id, key, duration_ms, None);
    }

    pub fn spawn_on_keyed_for_with_count(
        &mut self,
        effect_id: EffectId,
        entity_id: u32,
        key: u32,
        duration_ms: u32,
        hit_count: Option<u8>,
    ) {
        let duration_ms = if duration_ms == 0 {
            u32::MAX
        } else {
            duration_ms
        };
        self.push(SpawnRequest {
            key: Some(key),
            override_duration_ms: Some(duration_ms),
            hit_count,
            ..SpawnRequest::new(effect_id, Attach::Entity(entity_id))
        });
    }

    pub fn spawn_at_keyed(&mut self, effect_id: EffectId, world_pos: [f32; 3], key: u32) {
        self.push(SpawnRequest {
            key: Some(key),
            ..SpawnRequest::new(effect_id, Attach::WorldPos(world_pos))
        });
    }

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

    pub fn spawn_trail(&mut self, effect_id: EffectId, from: [f32; 3], to: [f32; 3]) {
        self.push(SpawnRequest::new(effect_id, Attach::Trail { from, to }));
    }

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

    pub fn spawn_link(&mut self, effect_id: EffectId, caster: u32, target: u32) {
        self.push(SpawnRequest::new(
            effect_id,
            Attach::Link { caster, target },
        ));
    }

    pub fn despawn(&mut self, key: u32) {
        self.pending.retain(|r| r.key != Some(key));
        self.despawns.push(key);
    }

    pub fn drain(&mut self) -> Vec<SpawnRequest> {
        std::mem::take(&mut self.pending)
    }

    pub fn drain_despawns(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.despawns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_spawn_and_despawn_channel_round_trip() {
        let mut q = EffectQueue::new();
        q.spawn_on_keyed(EffectId::Blessing, 42, 7);

        let pending = q.drain();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].attach, Attach::Entity(42));
        assert_eq!(pending[0].key, Some(7));
        assert!(q.drain().is_empty());

        q.despawn(7);
        assert_eq!(q.drain_despawns(), vec![7]);
        assert!(q.drain_despawns().is_empty());
    }

    #[test]
    fn despawn_cancels_a_same_frame_pending_spawn_of_that_key() {
        // Spirit spheres ramping 1→2→3 in one frame: each step despawns the
        // previous key then queues the next. Only the last spawn must survive,
        // and the superseded keys must not leak into the holder.
        let mut q = EffectQueue::new();
        q.spawn_on_keyed_with_count(EffectId::Chookgi2, 42, 1, 1);
        q.despawn(1);
        q.spawn_on_keyed_with_count(EffectId::Chookgi2, 42, 2, 2);
        q.despawn(2);
        q.spawn_on_keyed_with_count(EffectId::Chookgi2, 42, 3, 3);

        let pending = q.drain();
        assert_eq!(pending.len(), 1, "only the final spawn survives");
        assert_eq!(pending[0].key, Some(3));
        assert_eq!(pending[0].hit_count, Some(3));

        // Dropping to 0 spheres in one frame must cancel the pending spawn too.
        let mut q = EffectQueue::new();
        q.spawn_on_keyed_with_count(EffectId::Chookgi2, 42, 5, 5);
        q.despawn(5);
        assert!(q.drain().is_empty(), "count→0 leaves no pending spheres");
    }

    #[test]
    fn clear_drops_pending_spawns_and_despawns() {
        let mut q = EffectQueue::new();
        q.spawn_on_keyed_with_count(EffectId::Chookgi2, 42, 1, 1);
        q.despawn(9);
        q.clear();
        assert!(q.drain().is_empty());
        assert!(q.drain_despawns().is_empty());
    }

    #[test]
    fn disabling_effects_drops_one_shots_but_keeps_keyed_spawns() {
        let mut q = EffectQueue::new();
        q.set_effects_enabled(false);
        q.spawn_at(EffectId::Firehit, [0.0, 0.0, 0.0]);
        q.spawn_on(EffectId::Blessing, 42);
        q.spawn_on_keyed(EffectId::Blessing, 42, 7);

        let pending = q.drain();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].key, Some(7));

        q.set_effects_enabled(true);
        q.spawn_at(EffectId::Firehit, [0.0, 0.0, 0.0]);
        assert_eq!(q.drain().len(), 1);
    }

    #[test]
    fn keyed_timed_spawn_carries_key_and_duration() {
        let mut q = EffectQueue::new();
        q.spawn_on_keyed_for(EffectId::Redbody, 42, 0x8000_0001, 60_000);
        q.spawn_on_keyed_for(EffectId::Pinkbody, 42, 0x8000_0002, 0);

        let pending = q.drain();
        assert_eq!(pending[0].key, Some(0x8000_0001));
        assert_eq!(pending[0].override_duration_ms, Some(60_000));
        assert_eq!(pending[1].override_duration_ms, Some(u32::MAX));
    }
}

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

pub fn is_count_point_effect(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Chookgi
            | EffectId::Chookgi2
            | EffectId::Chookgi3
            | EffectId::Icearrow
            | EffectId::Firearrow
    )
}

pub fn is_caster_link_effect(id: EffectId) -> bool {
    matches!(id, EffectId::Soulbreaker | EffectId::Energydrain2)
}

pub fn is_trail_effect(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Frostdiver
            | EffectId::Blooddrain
            | EffectId::Energydrain
            | EffectId::Grimtooth
            | EffectId::Icewall
            | EffectId::Fireball
            | EffectId::Soulstrike
            | EffectId::Soulstrike2
            | EffectId::Soulbreaker
            | EffectId::Yufitel
            | EffectId::Pierce
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
            | EffectId::Chemical2
            | EffectId::Chemical2dash
            | EffectId::Chemical3
            | EffectId::Chemical4
            | EffectId::Smatk1
            | EffectId::Smatk2
            | EffectId::Smatk3
            | EffectId::Smatk4
            | EffectId::Stin
            | EffectId::Stin2
            | EffectId::Stin3
            | EffectId::Stin4
            | EffectId::Stin5
            | EffectId::Sma
            | EffectId::Teihit2
            | EffectId::Backstap
            | EffectId::Tanji
            | EffectId::Tanji2
            | EffectId::Alattack1
            | EffectId::Alattack2
            | EffectId::Alattack3
            | EffectId::Alattack4
            | EffectId::Shieldboomerang
            | EffectId::Shieldboomerang2
            | EffectId::Shieldboomerang3
            | EffectId::Slim
            | EffectId::Slim2
            | EffectId::Slim3
            | EffectId::Pressure
            | EffectId::Tripleattack
            | EffectId::Tripleattack2
            | EffectId::Tripleattack3
            | EffectId::Spearbmr
            | EffectId::Waterball2
            | EffectId::Wink
            | EffectId::Fvoice
    )
}

#[derive(Clone, Copy, Debug)]
pub enum ProjectileFlight {
    FixedFrames(f32),
    ConstantSpeed {
        delay_frames: f32,
        units_per_frame: f32,
    },
    AtTarget,
}

impl ProjectileFlight {
    const FPS: f32 = 60.0;

    pub fn reach_secs(self, distance_units: f32) -> f32 {
        let frames = match self {
            ProjectileFlight::FixedFrames(f) => f,
            ProjectileFlight::ConstantSpeed {
                delay_frames,
                units_per_frame,
            } => delay_frames + distance_units / units_per_frame.max(1e-3),
            ProjectileFlight::AtTarget => 0.0,
        };
        frames / Self::FPS
    }
}

pub fn trail_arrival_secs(id: EffectId, distance_units: f32) -> Option<f32> {
    use crate::effects;
    let flight = match id {
        EffectId::Fireball => effects::fireball::PROJECTILE_FLIGHT,
        EffectId::Waterball2 => effects::waterball2::PROJECTILE_FLIGHT,
        EffectId::Yufitel => effects::yupitel::PROJECTILE_FLIGHT,
        EffectId::Spearbmr => effects::spearbmr::PROJECTILE_FLIGHT,
        EffectId::Soulstrike | EffectId::Soulstrike2 => effects::soul_strike::PROJECTILE_FLIGHT,
        EffectId::Frostdiver | EffectId::Grimtooth => effects::frost_diver::PROJECTILE_FLIGHT,
        EffectId::Throwitem
        | EffectId::Throwitem2
        | EffectId::Throwitem3
        | EffectId::Throwitem4
        | EffectId::Throwitem5
        | EffectId::Throwitem6
        | EffectId::Throwitem7
        | EffectId::Throwitem8
        | EffectId::Throwitem9
        | EffectId::Throwitem10 => effects::throw_item::projectile_flight(id),
        EffectId::Pressure => effects::pressure::PROJECTILE_FLIGHT,
        EffectId::Tripleattack | EffectId::Tripleattack2 | EffectId::Tripleattack3 => {
            effects::tripleattack::PROJECTILE_FLIGHT
        }
        EffectId::Teihit2 | EffectId::Backstap => effects::teihit::PROJECTILE_FLIGHT,
        EffectId::Soulbreaker => effects::soul_breaker::PROJECTILE_FLIGHT,
        EffectId::Stin | EffectId::Stin2 | EffectId::Stin3 | EffectId::Stin4 | EffectId::Stin5 => {
            effects::stin::PROJECTILE_FLIGHT
        }
        EffectId::Chemical2
        | EffectId::Chemical2dash
        | EffectId::Chemical3
        | EffectId::Chemical4 => effects::chemical::PROJECTILE_FLIGHT,
        EffectId::Tanji
        | EffectId::Tanji2
        | EffectId::Shieldboomerang
        | EffectId::Shieldboomerang2
        | EffectId::Shieldboomerang3 => effects::cloud_projectile::PROJECTILE_FLIGHT,
        _ => return None,
    };
    Some(flight.reach_secs(distance_units))
}

pub fn is_link_effect(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Linelink | EffectId::Linelink2 | EffectId::Linelink3
    )
}
