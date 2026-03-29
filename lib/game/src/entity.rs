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
    Attacking,
    Hurt,
    Dead,
    Pickup,
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
    pub name: Option<String>,
    pub name_requested: bool,
    pub direction: u8,
    pub head_dir: u8,
    pub speed: u16,
    pub state: EntityState,
    pub state_timer: f32,
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
            name: None,
            name_requested: false,
            direction, head_dir: direction, speed,
            state: EntityState::Standing,
            state_timer: 0.0,
            movement,
            animation: SpriteAnimationState::new(direction),
        }
    }

    pub fn new_player(id: u32, job: u16, sex: u8, head: u16, hair_color: u16, weapon: u16, head_top: u16, head_mid: u16, head_bottom: u16, shield: u16, x: u16, y: u16, direction: u8) -> Self {
        Self::new(id, EntityType::Player, job, sex, head, hair_color, weapon, head_top, head_mid, head_bottom, shield, x, y, direction, 150)
    }

    pub fn update_state(&mut self, dt: f32) {
        if self.state == EntityState::Dead {
            return;
        }
        if self.state_timer > 0.0 {
            self.state_timer -= dt;
            if self.state_timer <= 0.0 {
                self.state_timer = 0.0;
                self.state = EntityState::Standing;
            }
            return;
        }
        if self.state == EntityState::Sitting {
            return;
        }
        self.state = if self.movement.is_moving() {
            EntityState::Moving
        } else {
            EntityState::Standing
        };
    }

    pub fn enter_hurt(&mut self, duration_secs: f32) {
        if self.state == EntityState::Dead || self.state == EntityState::Attacking {
            return;
        }
        self.movement.stop();
        self.state = EntityState::Hurt;
        self.state_timer = duration_secs;
    }

    pub fn enter_attack(&mut self, duration_secs: f32) {
        if self.state == EntityState::Dead {
            return;
        }
        self.state = EntityState::Attacking;
        self.state_timer = duration_secs;
    }

    pub fn enter_dead(&mut self) {
        self.state = EntityState::Dead;
        self.state_timer = 0.0;
        self.movement.stop();
    }

    pub fn enter_pickup(&mut self, duration_secs: f32) {
        if self.state == EntityState::Dead {
            return;
        }
        self.state = EntityState::Pickup;
        self.state_timer = duration_secs;
    }

    pub fn action_index(&self) -> usize {
        match self.entity_type {
            EntityType::Player => match self.state {
                EntityState::Standing => 0,
                EntityState::Moving => 1,
                EntityState::Sitting => 2,
                EntityState::Pickup => 3,
                EntityState::Attacking => 5,
                EntityState::Hurt => 6,
                EntityState::Dead => 8,
            },
            EntityType::Monster | EntityType::Npc => match self.state {
                EntityState::Standing | EntityState::Sitting | EntityState::Pickup => 0,
                EntityState::Moving => 1,
                EntityState::Attacking => 2,
                EntityState::Hurt => 3,
                EntityState::Dead => 4,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathNode;

    fn make_entity() -> Entity {
        Entity::new_player(1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 100, 100, 0)
    }

    fn make_path_node(x: u16, y: u16, is_diagonal: bool) -> PathNode {
        PathNode { id: 0, parent_id: 0, x, y, g_cost: 0, f_cost: 0, is_open: false, is_diagonal }
    }

    #[test]
    fn entity_starts_without_name() {
        let e = make_entity();
        assert!(e.name.is_none());
        assert!(!e.name_requested);
    }

    #[test]
    fn action_index_maps_states_to_player_sprite_actions() {
        let mut e = make_entity();
        assert_eq!(e.action_index(), 0);
        e.state = EntityState::Moving;
        assert_eq!(e.action_index(), 1);
        e.state = EntityState::Sitting;
        assert_eq!(e.action_index(), 2);
        e.state = EntityState::Pickup;
        assert_eq!(e.action_index(), 3);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 5);
        e.state = EntityState::Hurt;
        assert_eq!(e.action_index(), 6);
        e.state = EntityState::Dead;
        assert_eq!(e.action_index(), 8);
    }

    #[test]
    fn action_index_maps_states_to_monster_sprite_actions() {
        let mut e = Entity::new(2, EntityType::Monster, 1002, 0, 0, 0, 0, 0, 0, 0, 0, 100, 100, 0, 200);
        assert_eq!(e.action_index(), 0);
        e.state = EntityState::Moving;
        assert_eq!(e.action_index(), 1);
        e.state = EntityState::Attacking;
        assert_eq!(e.action_index(), 2);
        e.state = EntityState::Hurt;
        assert_eq!(e.action_index(), 3);
        e.state = EntityState::Dead;
        assert_eq!(e.action_index(), 4);
    }

    #[test]
    fn update_state_preserves_sitting() {
        let mut e = make_entity();
        e.state = EntityState::Sitting;
        e.update_state(0.016);
        assert_eq!(e.state, EntityState::Sitting);
    }

    #[test]
    fn hurt_cancels_movement_and_recovers_to_standing() {
        let mut e = make_entity();
        let path = vec![make_path_node(101, 100, false), make_path_node(102, 100, false)];
        e.movement.start_move(path, 0.0);
        assert!(e.movement.is_moving());

        e.enter_hurt(0.5);
        assert_eq!(e.state, EntityState::Hurt);
        assert!(!e.movement.is_moving());

        // Still in hurt state after partial tick
        e.update_state(0.3);
        assert_eq!(e.state, EntityState::Hurt);

        // Timer expires, returns to standing
        e.update_state(0.3);
        assert_eq!(e.state, EntityState::Standing);
    }

    #[test]
    fn dead_blocks_all_transitions() {
        let mut e = make_entity();
        e.enter_dead();
        assert_eq!(e.state, EntityState::Dead);

        e.enter_hurt(1.0);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_attack(1.0);
        assert_eq!(e.state, EntityState::Dead);

        e.enter_pickup(1.0);
        assert_eq!(e.state, EntityState::Dead);

        e.update_state(1.0);
        assert_eq!(e.state, EntityState::Dead);
    }

    #[test]
    fn attacking_blocks_hurt() {
        let mut e = make_entity();
        e.enter_attack(1.0);
        assert_eq!(e.state, EntityState::Attacking);

        e.enter_hurt(0.5);
        assert_eq!(e.state, EntityState::Attacking);
    }
}
