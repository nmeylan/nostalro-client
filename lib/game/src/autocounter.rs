use crate::character::Character;
use crate::entity::EntityState;
use crate::entity_collection::EntityCollection;
use models::enums::skill_enums::SkillEnum;

const AUTOCOUNTER_SECS_PER_LEVEL: f32 = 0.4;

pub struct ChannelParams {
    pub duration: f32,
    pub face: Option<u32>,
}

pub fn is_kn_autocounter(skill_id: u16) -> bool {
    skill_id == SkillEnum::KnAutocounter.id() as u16
}

pub fn player_in_autocounter(entities: &EntityCollection) -> bool {
    entities.player().is_some_and(|e| {
        e.state == EntityState::Casting && e.active_skill_id.is_some_and(is_kn_autocounter)
    })
}

pub fn channel_params(
    character: &Character,
    is_player: bool,
    last_attacked_enemy: Option<u32>,
    attack_target: Option<u32>,
) -> ChannelParams {
    let skill_id = SkillEnum::KnAutocounter.id() as u16;
    let level = character
        .skills
        .get_skill(skill_id)
        .map(|s| s.level.max(1))
        .unwrap_or(1);
    let duration = level as f32 * AUTOCOUNTER_SECS_PER_LEVEL;
    let face = if is_player {
        last_attacked_enemy.or(attack_target)
    } else {
        None
    };
    ChannelParams { duration, face }
}
