use crate::App;
use ragnarok_game::cursor::PendingSkillTarget;
use ragnarok_game::entity::{EntityState, EntityType};
use ragnarok_game::path::try_move_to;
use ragnarok_game::targeting::{can_attack, skill_target_allowed, skill_target_class, TargetClass};
use ragnarok_network::{
    build_contact_npc_packet, build_pickup_item_packet, build_request_move_packet,
    build_use_skill_packet, build_use_skill_to_ground_packet,
};

impl App {
    pub(crate) fn handle_left_click(&mut self) {
        if self.game.npc_dialog.dialog.is_open() || self.game.npc_shop.shop.is_open() {
            return;
        }
        if let Some(entity) = self.game.entities.player()
            && matches!(entity.state, EntityState::Casting | EntityState::SkillExec)
        {
            return;
        }
        // Incapacitated (stun/freeze/stone/sleep): the server rejects the action,
        // so swallow the click instead of mis-predicting movement client-side.
        if self.is_local_player_incapacitated() {
            return;
        }
        // Skill targeting mode: consume pending skill on click. While a (global
        // or per-skill) cooldown is running the cursor keeps showing the skill
        // ring, but the click casts nothing — leave the pending target armed so
        // the player can cast once the cooldown clears.
        if let Some(pending) = self.game.pending_skill_target {
            if self.skill_on_cooldown(pending.skill_id()) {
                return;
            }
            self.game.pending_skill_target = None;
            let mut skill_cast = false;
            match pending {
                PendingSkillTarget::Entity { skill_id, level } => {
                    let class = self
                        .game
                        .character
                        .skills
                        .get_skill(skill_id)
                        .map(|s| skill_target_class(s.skill_target_type))
                        .unwrap_or(TargetClass::Offensive);
                    let player_id = self.game.entities.player_id();
                    let valid_target = self.game.hovered_entity_id.filter(|&id| {
                        self.game.entities.get(id).is_some_and(|e| {
                            skill_target_allowed(class, e, &self.game.map_properties, player_id)
                        })
                    });
                    if let Some(entity_id) = valid_target {
                        let target_pos = self
                            .game
                            .entities
                            .get(entity_id)
                            .map(|e| e.movement.cell_position())
                            .unwrap_or((0, 0));
                        let (px, py) = self
                            .game
                            .entities
                            .player()
                            .map(|e| e.movement.cell_position())
                            .unwrap_or((0, 0));
                        let skill_range = self
                            .game
                            .character
                            .skills
                            .get_skill(skill_id)
                            .map(|s| s.attack_range as i32)
                            .unwrap_or(1);
                        let dx = (px as i32 - target_pos.0 as i32).abs();
                        let dy = (py as i32 - target_pos.1 as i32).abs();
                        let dist = dx.max(dy);
                        if dist <= skill_range {
                            self.channel.send_packet(build_use_skill_packet(
                                skill_id,
                                level,
                                entity_id,
                                self.config.packetver,
                            ));
                            skill_cast = true;
                        } else {
                            let dest_x = target_pos.0 as i32;
                            let dest_y = target_pos.1 as i32;
                            if self.try_move_toward(dest_x, dest_y, px, py, skill_range) {
                                self.game.pending_skill_id = Some(skill_id);
                                self.game.pending_skill_level = Some(level);
                                self.game.attack_target_id = Some(entity_id);
                            }
                        }
                    }
                }
                PendingSkillTarget::Ground { skill_id, level } => {
                    if let Some((cx, cy)) = self.hovered_cell() {
                        self.channel.send_packet(build_use_skill_to_ground_packet(
                            skill_id,
                            level,
                            cx as i16,
                            cy as i16,
                            self.config.packetver,
                        ));
                    }
                    skill_cast = true;
                }
            }
            if skill_cast {
                self.game.attack_target_id = None;
            }
            return;
        }
        // Click on floor item to pick up
        if let Some(item_id) = self.game.hovered_floor_item_id {
            self.game.attack_target_id = None;
            self.game.pending_pickup_item_id = None;
            if let Some(floor_item) = self.game.floor_items.get(&item_id) {
                let (px, py) = self
                    .game
                    .entities
                    .player()
                    .map(|e| e.movement.cell_position())
                    .unwrap_or((0, 0));
                let dx = (px as i32 - floor_item.x as i32).unsigned_abs();
                let dy = (py as i32 - floor_item.y as i32).unsigned_abs();
                if dx <= 1 && dy <= 1 {
                    self.channel
                        .send_packet(build_pickup_item_packet(item_id, self.config.packetver));
                    if let Some(entity) = self.game.entities.player_mut() {
                        entity.enter_pickup(0.5);
                    }
                } else if let Some(gat) = &self.game.gat {
                    let dest_x = floor_item.x as i32;
                    let dest_y = floor_item.y as i32;
                    if let Some(move_action) = try_move_to(gat, px, py, dest_x, dest_y) {
                        self.channel.send_packet(build_request_move_packet(
                            move_action.dest_x,
                            move_action.dest_y,
                            self.config.packetver,
                        ));
                        let elapsed = self.start_time.elapsed().as_secs_f32();
                        if let Some(entity) = self.game.entities.player_mut() {
                            entity.movement.start_move(move_action.path, elapsed);
                        }
                        self.game.pending_pickup_item_id = Some(item_id);
                    }
                }
            }
            return;
        }
        // Click on NPC to talk
        if let Some(entity_id) = self.game.hovered_entity_id
            && let Some(entity) = self.game.entities.get(entity_id)
            && entity.entity_type == EntityType::Npc
            && entity.job != 45
        {
            self.channel
                .send_packet(build_contact_npc_packet(entity_id, self.config.packetver));
            return;
        }
        // Click on monster to attack (always), or player (shift or noshift mode)
        if let Some(entity_id) = self.game.hovered_entity_id
            && let Some(entity) = self.game.entities.get(entity_id)
        {
            let player_id = self.game.entities.player_id();
            let should_attack = match entity.entity_type {
                EntityType::Monster => !self.input.shift_pressed,
                EntityType::Player => {
                    can_attack(entity, &self.game.map_properties, player_id)
                        && (!self.game.map_properties.no_lockon()
                            || self.input.shift_pressed
                            || self.game.noshift_mode)
                }
                _ => false,
            };
            if should_attack {
                self.initiate_attack(entity_id);
                return;
            }
        }
        self.game.attack_target_id = None;
        self.game.pending_pickup_item_id = None;
        let (dest_x, dest_y) = match self.hovered_cell() {
            Some(c) => c,
            None => return,
        };
        let gat = match &self.game.gat {
            Some(g) => g,
            None => return,
        };

        let (src_x, src_y) = self
            .game
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let move_action = match try_move_to(gat, src_x, src_y, dest_x, dest_y) {
            Some(a) => a,
            None => return,
        };

        self.channel.send_packet(build_request_move_packet(
            move_action.dest_x,
            move_action.dest_y,
            self.config.packetver,
        ));

        let elapsed = self.start_time.elapsed().as_secs_f32();
        if let Some(entity) = self.game.entities.player_mut() {
            entity.movement.start_move(move_action.path, elapsed);
        }
    }
}
