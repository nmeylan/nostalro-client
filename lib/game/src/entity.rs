use models::enums::weapon::WeaponType;

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
    pub state: EntityState,
    pub movement: MovementState,
}

impl Entity {
    pub fn new_player(id: u32, job: u16, sex: u8, head: u16, hair_color: u16, weapon: u16, head_top: u16, head_mid: u16, head_bottom: u16, shield: u16, x: u16, y: u16, direction: u8) -> Self {
        Self {
            id,
            entity_type: EntityType::Player,
            job,
            sex,
            head,
            hair_color,
            weapon: weapon_view_id_to_type(weapon),
            head_top,
            head_mid,
            head_bottom,
            shield,
            direction,
            head_dir: direction,
            state: EntityState::Standing,
            movement: MovementState::new(x, y),
        }
    }

    pub fn update_state(&mut self) {
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
        }
    }
}
