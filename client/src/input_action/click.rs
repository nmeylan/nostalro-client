use crate::App;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::autocounter;
use ragnarok_game::companion::OwnerCommand;
use ragnarok_game::cursor::PendingSkillTarget;
use ragnarok_game::entity::{EntityState, EntityType};
use ragnarok_game::path::try_move_to;
use ragnarok_game::sprite_path::hide_allows_skill;
use ragnarok_game::targeting::{TargetClass, can_attack, skill_target_allowed, skill_target_class};
use ragnarok_network::{
    build_contact_npc_packet, build_pickup_item_packet, build_req_buy_frommc_packet,
    build_req_enter_room_packet, build_request_move_packet, build_use_skill_packet,
    build_use_skill_to_ground_packet,
};

impl App {
    pub(crate) fn handle_left_click(&mut self) {
        if self.try_pet_modal_click() {
            return;
        }
        if ragnarok_profiling::debug::trace_input() {
            tracing::info!(
                "handle_left_click: pending_companion={:?} hovered_entity={:?}",
                self.game.pending_casts.pending_companion_skill.is_some(),
                self.game.hover.hovered_entity_id
            );
        }
        if self.windows.npc_dialog.dialog.is_open() || self.windows.npc_shop.shop.is_open() {
            return;
        }
        if autocounter::player_in_autocounter(&self.game.world.entities) {
            self.dispel_autocounter();
            return;
        }
        if let Some(entity) = self.game.world.entities.player()
            && matches!(entity.state, EntityState::Casting | EntityState::SkillExec)
        {
            return;
        }
        if let Some(entity_id) = self.game.hover.target_id()
            && Some(entity_id) == self.game.world.entities.player_id()
            && self
                .game
                .world
                .entities
                .get(entity_id)
                .is_some_and(|e| e.vending_board.is_some())
        {
            self.close_own_shop();
            return;
        }
        if let Some(room_id) = self.game.hover.hovered_chat_room {
            self.channel
                .send_packet(build_req_enter_room_packet(room_id, self.active_packetver));
            return;
        }
        if let Some(pending) = self.game.pending_casts.pending_companion_skill.take() {
            let reserved = self.input.shift_pressed;
            if pending.is_ground {
                if let Some((cx, cy)) = self.hovered_cell() {
                    self.push_owner_command_to(
                        pending.is_mercenary,
                        OwnerCommand::skill_area(pending.skill_id, pending.level as u8, cx, cy),
                        reserved,
                    );
                }
            } else if let Some(target) = self.game.hover.hovered_entity_id {
                self.push_owner_command_to(
                    pending.is_mercenary,
                    OwnerCommand::skill_object(pending.skill_id, pending.level as u8, target),
                    reserved,
                );
            }
            return;
        }
        if self.is_local_player_incapacitated() {
            return;
        }
        if let Some(pending) = self.game.pending_casts.pending_skill_target {
            if self.player_hidden() && !hide_allows_skill(pending.skill_id()) {
                self.game.pending_casts.pending_skill_target = None;
                return;
            }
            if self.skill_on_cooldown(pending.skill_id()) {
                return;
            }
            self.game.pending_casts.pending_skill_target = None;
            let mut skill_cast = false;
            match pending {
                PendingSkillTarget::Entity { skill_id, level } => {
                    let class = self
                        .game
                        .resolve_cast_skill(skill_id)
                        .map(|(target_type, _)| skill_target_class(target_type))
                        .unwrap_or(TargetClass::Offensive);
                    let player_id = self.game.world.entities.player_id();
                    let potion_pitcher = skill_id == SkillEnum::AmPotionpitcher.id() as u16;
                    let valid_target = self.game.hover.hovered_entity_id.filter(|&id| {
                        self.game.world.entities.get(id).is_some_and(|e| {
                            skill_target_allowed(class, e, &self.game.session.map_properties, player_id)
                                || (potion_pitcher
                                    && matches!(
                                        e.entity_type,
                                        EntityType::Homunculus | EntityType::Mercenary
                                    ))
                        })
                    });
                    if let Some(entity_id) = valid_target {
                        let target_pos = self
                            .game
                            .world
                            .entities
                            .get(entity_id)
                            .map(|e| e.movement.cell_position())
                            .unwrap_or((0, 0));
                        let (px, py) = self
                            .game
                            .world
                            .entities
                            .player()
                            .map(|e| e.movement.cell_position())
                            .unwrap_or((0, 0));
                        let skill_range = self
                            .game
                            .resolve_cast_skill(skill_id)
                            .map(|(_, range)| range as i32)
                            .unwrap_or(1);
                        let dx = (px as i32 - target_pos.0 as i32).abs();
                        let dy = (py as i32 - target_pos.1 as i32).abs();
                        let dist = dx.max(dy);
                        if dist <= skill_range {
                            self.channel.send_packet(build_use_skill_packet(
                                skill_id,
                                level,
                                entity_id,
                                self.active_packetver,
                            ));
                            if skill_id == SkillEnum::BsRepairweapon.id() as u16 {
                                self.game.pending_casts.pending_repair_target = Some(entity_id);
                            }
                            skill_cast = true;
                        } else {
                            let dest_x = target_pos.0 as i32;
                            let dest_y = target_pos.1 as i32;
                            if self.try_move_toward(dest_x, dest_y, px, py, skill_range) {
                                self.game.pending_casts.pending_skill_id = Some(skill_id);
                                self.game.pending_casts.pending_skill_level = Some(level);
                                self.game.combat.attack_target_id = Some(entity_id);
                            }
                        }
                    }
                }
                PendingSkillTarget::Ground { skill_id, level } => {
                    if let Some((cx, cy)) = self.hovered_cell() {
                        let (px, py) = self
                            .game
                            .world
                            .entities
                            .player()
                            .map(|e| e.movement.cell_position())
                            .unwrap_or((0, 0));
                        let skill_range = self
                            .game
                            .resolve_cast_skill(skill_id)
                            .map(|(_, range)| range as i32)
                            .unwrap_or(1);
                        let dx = (px as i32 - cx as i32).abs();
                        let dy = (py as i32 - cy as i32).abs();
                        if dx.max(dy) <= skill_range {
                            self.channel.send_packet(build_use_skill_to_ground_packet(
                                skill_id,
                                level,
                                cx as i16,
                                cy as i16,
                                self.active_packetver,
                            ));
                        } else {
                            self.game.pending_casts.pending_ground_cast =
                                Some((skill_id, level, cx as i16, cy as i16));
                            self.try_move_toward(cx as i32, cy as i32, px, py, skill_range);
                        }
                    }
                    skill_cast = true;
                }
            }
            if skill_cast {
                self.game.combat.attack_target_id = None;
            }
            return;
        }
        if let Some(item_id) = self.game.hover.hovered_floor_item_id {
            if self.player_hidden() {
                return;
            }
            self.game.combat.attack_target_id = None;
            self.game.pending_casts.pending_pickup_item_id = None;
            if let Some(floor_item) = self.game.world.floor_items.get(&item_id) {
                let (px, py) = self
                    .game
                    .world
                    .entities
                    .player()
                    .map(|e| e.movement.cell_position())
                    .unwrap_or((0, 0));
                let dx = (px as i32 - floor_item.x as i32).unsigned_abs();
                let dy = (py as i32 - floor_item.y as i32).unsigned_abs();
                if dx <= 1 && dy <= 1 {
                    self.channel
                        .send_packet(build_pickup_item_packet(item_id, self.active_packetver));
                    if let Some(entity) = self.game.world.entities.player_mut() {
                        entity.enter_pickup(0.5);
                    }
                } else if let Some(gat) = &self.game.session.gat {
                    let dest_x = floor_item.x as i32;
                    let dest_y = floor_item.y as i32;
                    if let Some(move_action) = try_move_to(gat, px, py, dest_x, dest_y) {
                        self.channel.send_packet(build_request_move_packet(
                            move_action.dest_x,
                            move_action.dest_y,
                            self.active_packetver,
                        ));
                        self.game.pending_casts.pending_pickup_item_id = Some(item_id);
                    }
                }
            }
            return;
        }
        if let Some(entity_id) = self.game.hover.target_id()
            && Some(entity_id) != self.game.world.entities.player_id()
            && self
                .game
                .world
                .entities
                .get(entity_id)
                .is_some_and(|e| e.vending_board.is_some())
        {
            self.channel
                .send_packet(build_req_buy_frommc_packet(entity_id, self.active_packetver));
            return;
        }
        if let Some(entity_id) = self.game.hover.hovered_entity_id
            && let Some(entity) = self.game.world.entities.get(entity_id)
            && entity.entity_type == EntityType::Npc
            && entity.job != 45
        {
            self.channel
                .send_packet(build_contact_npc_packet(entity_id, self.active_packetver));
            return;
        }
        if let Some(entity_id) = self.game.hover.hovered_entity_id
            && let Some(entity) = self.game.world.entities.get(entity_id)
        {
            let player_id = self.game.world.entities.player_id();
            let should_attack = match entity.entity_type {
                EntityType::Monster => !self.input.shift_pressed && !entity.is_pet,
                EntityType::Player => {
                    can_attack(entity, &self.game.session.map_properties, player_id)
                        && (!self.game.session.map_properties.no_lockon()
                            || self.input.shift_pressed
                            || self.game.prefs.noshift_mode)
                }
                _ => false,
            };
            if should_attack {
                self.initiate_attack(entity_id);
                return;
            }
        }
        self.game.combat.attack_target_id = None;
        self.game.pending_casts.pending_pickup_item_id = None;
        self.game.pending_casts.pending_ground_cast = None;
        // While running, the server auto-moves the character in a straight line and
        // rejects any client-issued move, which snaps the character back. Suppress
        // click-to-move (and the continuous-walk that routes through here) entirely.
        if self
            .game
            .world
            .entities
            .player()
            .is_some_and(|e| e.is_running)
        {
            return;
        }
        if self.game.world.entities.player().is_some_and(|e| e.is_move_locked()) {
            return;
        }
        if self.player_hide_move_blocked() {
            return;
        }
        let (dest_x, dest_y) = match self.hovered_cell() {
            Some(c) => c,
            None => return,
        };
        let gat = match &self.game.session.gat {
            Some(g) => g,
            None => return,
        };

        let (src_x, src_y) = self
            .game
            .world
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
            self.active_packetver,
        ));
    }
}
