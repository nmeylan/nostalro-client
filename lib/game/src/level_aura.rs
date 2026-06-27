use crate::entity::EntityType;
use crate::sprite_path::is_hidden;

pub const LEVEL_AURA_THRESHOLD: i16 = 99;

pub fn level_aura_visible(entity_type: EntityType, base_level: i16, effect_state: i32) -> bool {
    entity_type == EntityType::Player
        && base_level >= LEVEL_AURA_THRESHOLD
        && !is_hidden(effect_state)
}

pub fn boss_aura_visible(entity_type: EntityType, is_boss: bool, effect_state: i32) -> bool {
    entity_type == EntityType::Monster && is_boss && !is_hidden(effect_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite_path::OPTION_CLOAK;

    #[test]
    fn aura_is_player_only_at_threshold_and_hidden_when_cloaked() {
        assert!(level_aura_visible(EntityType::Player, 99, 0));
        assert!(level_aura_visible(EntityType::Player, 150, 0));
        assert!(!level_aura_visible(EntityType::Player, 98, 0));
        assert!(!level_aura_visible(EntityType::Monster, 99, 0));
        assert!(!level_aura_visible(EntityType::Npc, 99, 0));
        assert!(!level_aura_visible(EntityType::Player, 99, OPTION_CLOAK));
    }

    #[test]
    fn boss_aura_is_monster_only_and_hidden_when_cloaked() {
        assert!(boss_aura_visible(EntityType::Monster, true, 0));
        assert!(!boss_aura_visible(EntityType::Monster, false, 0));
        assert!(!boss_aura_visible(EntityType::Player, true, 0));
        assert!(!boss_aura_visible(EntityType::Npc, true, 0));
        assert!(!boss_aura_visible(EntityType::Monster, true, OPTION_CLOAK));
    }
}
