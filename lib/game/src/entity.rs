use crate::movement::MovementState;

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
    pub direction: u8,
    pub state: EntityState,
    pub movement: MovementState,
}

impl Entity {
    pub fn new_player(id: u32, job: u16, sex: u8, head: u16, hair_color: u16, x: u16, y: u16, direction: u8) -> Self {
        Self {
            id,
            entity_type: EntityType::Player,
            job,
            sex,
            head,
            hair_color,
            direction,
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
