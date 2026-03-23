use models::enums::weapon::WeaponType;
use ragnarok_formats::act::SpriteAnimationState;

use crate::movement::MovementState;
use crate::sprite_path::weapon_view_id_to_type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Player,
    Npc,
    Monster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityState {
    Standing,
    Moving,
    Sitting,
}

pub struct Entity {
    pub id: u32,
    pub entity_type: EntityType,
    pub job: u16,
    pub sex: u8,
    pub head: u16,
    pub hair_color: u16,
    pub weapon: Option<WeaponType>,
    pub head_top: u16,
    pub head_mid: u16,
    pub head_bottom: u16,
    pub shield: u16,
    pub direction: u8,
    pub head_dir: u8,
    pub speed: u16,
    pub state: EntityState,
    pub movement: MovementState,
    pub animation: SpriteAnimationState,
}

impl Entity {
    pub fn new(
        id: u32, entity_type: EntityType, job: u16, sex: u8, head: u16,
        hair_color: u16, weapon: u16, head_top: u16, head_mid: u16,
        head_bottom: u16, shield: u16, x: u16, y: u16, direction: u8, speed: u16,
    ) -> Self {
        let weapon_type = if entity_type == EntityType::Player {
            weapon_view_id_to_type(weapon)
        } else {
            None
        };
        let mut movement = MovementState::new(x, y);
        movement.set_speed(speed);
        Self {
            id, entity_type, job, sex, head, hair_color,
            weapon: weapon_type,
            head_top, head_mid, head_bottom, shield,
            direction, head_dir: direction, speed,
            state: EntityState::Standing,
            movement,
            animation: SpriteAnimationState::new(direction),
        }
    }

    pub fn new_player(id: u32, job: u16, sex: u8, head: u16, hair_color: u16, weapon: u16, head_top: u16, head_mid: u16, head_bottom: u16, shield: u16, x: u16, y: u16, direction: u8) -> Self {
        Self::new(id, EntityType::Player, job, sex, head, hair_color, weapon, head_top, head_mid, head_bottom, shield, x, y, direction, 150)
    }

    pub fn update_state(&mut self) {
        if self.state == EntityState::Sitting {
            return;
        }
        self.state = if self.movement.is_moving() {
            EntityState::Moving
        } else {
            EntityState::Standing
        };
    }

    pub fn action_index(&self) -> usize {
        match self.state {
            EntityState::Standing => 0,
            EntityState::Moving => 1,
            EntityState::Sitting => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity() -> Entity {
        Entity::new_player(1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 100, 100, 0)
    }

    #[test]
    fn action_index_maps_states_to_sprite_actions() {
        let mut e = make_entity();
        assert_eq!(e.action_index(), 0);
        e.state = EntityState::Moving;
        assert_eq!(e.action_index(), 1);
        e.state = EntityState::Sitting;
        assert_eq!(e.action_index(), 2);
    }

    #[test]
    fn update_state_preserves_sitting() {
        let mut e = make_entity();
        e.state = EntityState::Sitting;
        e.update_state();
        assert_eq!(e.state, EntityState::Sitting);
    }
}
