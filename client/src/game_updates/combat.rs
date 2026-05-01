use crate::App;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::entity::EntityState;
use ragnarok_game::movement::direction_from_positions;
use ragnarok_game::scheduled_hit::{DamageMessage, ScheduledHit};
use ragnarok_network::{build_pickup_item_packet, build_use_skill_packet};

impl App {
    pub(crate) fn check_pending_attack(&mut self, delta: f32) {
        self.game.attack_request_cooldown = (self.game.attack_request_cooldown - delta).max(0.0);

        let target_id = match self.game.attack_target_id {
            Some(id) => id,
            None => return,
        };

        let target_alive = self.game.entities.get(target_id)
            .is_some_and(|e| e.state != EntityState::Dead && !e.is_fading());
        if !target_alive {
            self.game.attack_target_id = None;
            return;
        }

        if let Some(player) = self.game.entities.player() {
            if matches!(player.state,
                EntityState::Casting | EntityState::SkillExec | EntityState::Dead | EntityState::Sitting
            ) {
                self.game.attack_target_id = None;
                return;
            }
        }

        if !self.game.attack_is_locked && !self.input.left_mouse_down {
            self.game.attack_target_id = None;
            return;
        }

        if self.game.attack_request_cooldown > 0.0 {
            return;
        }

        let target_pos = self.game.entities.get(target_id)
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));
        let (px, py) = self.game.entities.player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let range = self.game.attack_range as i32;
        let dx = (px as i32 - target_pos.0 as i32).abs();
        let dy = (py as i32 - target_pos.1 as i32).abs();
        let dist = dx.max(dy);

        if dist <= range {
            let player_state = self.game.entities.player()
                .map(|e| e.state)
                .unwrap_or(EntityState::Standing);
            if matches!(player_state, EntityState::Standing | EntityState::ReadyFight) {
                self.send_attack_packet(target_id);
                // TODO Having a doubt on this number should investigate how to do it properly
                self.game.attack_request_cooldown = 0.3;
            }
        } else if let Some(player) = self.game.entities.player() {
            if !matches!(player.state, EntityState::Casting | EntityState::SkillExec | EntityState::Dead | EntityState::Sitting) {
                // Call every frame to track a moving target (original game used to recalculates dest cell each tick)
                self.try_move_toward(target_pos.0 as i32, target_pos.1 as i32, px, py, range);
            }
        }
    }

    pub(crate) fn check_pending_skill(&mut self) {
        let (skill_id, level) = match (self.game.pending_skill_id, self.game.pending_skill_level) {
            (Some(sid), Some(lvl)) => (sid, lvl),
            _ => return,
        };

        let target_id = match self.game.attack_target_id {
            Some(id) => id,
            None => {
                self.game.pending_skill_id = None;
                self.game.pending_skill_level = None;
                return;
            }
        };

        let target_alive = self.game.entities.get(target_id)
            .is_some_and(|e| e.state != EntityState::Dead && !e.is_fading());
        if !target_alive {
            self.game.pending_skill_id = None;
            self.game.pending_skill_level = None;
            self.game.attack_target_id = None;
            return;
        }

        if let Some(player) = self.game.entities.player() {
            if matches!(player.state, EntityState::Casting | EntityState::SkillExec | EntityState::Dead | EntityState::Sitting) {
                return;
            }
        }

        let target_pos = self.game.entities.get(target_id)
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));
        let (px, py) = self.game.entities.player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let skill_range = self.game.character.skills.get_skill(skill_id)
            .map(|s| s.attack_range as i32)
            .unwrap_or(1);
        let dx = (px as i32 - target_pos.0 as i32).abs();
        let dy = (py as i32 - target_pos.1 as i32).abs();
        let dist = dx.max(dy);

        if dist <= skill_range {
            self.channel.send_packet(build_use_skill_packet(skill_id, level, target_id, self.config.packetver));
            self.game.pending_skill_id = None;
            self.game.pending_skill_level = None;
            self.game.attack_target_id = None;
        } else if let Some(player) = self.game.entities.player() {
            if !matches!(player.state, EntityState::Casting | EntityState::SkillExec | EntityState::Dead | EntityState::Sitting) {
                self.try_move_toward(target_pos.0 as i32, target_pos.1 as i32, px, py, skill_range);
            }
        }
    }

    pub(crate) fn check_pending_pickup(&mut self) {
        let item_id = match self.game.pending_pickup_item_id {
            Some(id) => id,
            None => return,
        };
        if !self.game.floor_items.contains_key(&item_id) {
            self.game.pending_pickup_item_id = None;
            return;
        }
        let (px, py) = self
            .game
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));
        let floor_item = &self.game.floor_items[&item_id];
        let dx = (px as i32 - floor_item.x as i32).unsigned_abs();
        let dy = (py as i32 - floor_item.y as i32).unsigned_abs();
        if dx <= 1 && dy <= 1 {
            self.channel.send_packet(build_pickup_item_packet(item_id, self.config.packetver));
            if let Some(entity) = self.game.entities.player_mut() {
                entity.movement.stop();
                entity.enter_pickup(0.5);
            }
            self.game.pending_pickup_item_id = None;
        }
    }

    pub(crate) fn process_scheduled_hits(&mut self) {
        let now = self.start_time.elapsed().as_secs_f32();
        let entity_ids: Vec<u32> = self.game.entities.iter().map(|e| e.id).collect();
        for entity_id in entity_ids {
            let ready = if let Some(entity) = self.game.entities.get_mut(entity_id) {
                entity.scheduled_hits.drain_ready(now)
            } else {
                continue;
            };
            for hit in ready {
                self.emit_damage_number(entity_id, &hit);

                if matches!(hit.message, DamageMessage::Attacked | DamageMessage::AttackedMultiHit { .. }) {
                    if hit.damage > 0 {
                        let is_sonic_or_chain = hit.skill_id == SkillEnum::AsSonicblow.id() as u16
                            || hit.skill_id == SkillEnum::MoChaincombo.id() as u16;
                        let attacker_pos = self.game.entities.get(hit.attacker_gid)
                            .map(|e| e.movement.cell_position());
                        if let Some(entity) = self.game.entities.get_mut(entity_id) {
                            if !is_sonic_or_chain {
                                if let Some(ap) = attacker_pos {
                                    let tp = entity.movement.cell_position();
                                    if let Some(dir) = direction_from_positions(tp.0, tp.1, ap.0, ap.1) {
                                        entity.direction = dir;
                                    }
                                }
                            }
                            entity.enter_hurt(hit.attacked_mt_secs.max(0.288));

                            if is_sonic_or_chain {
                                entity.direction = ((entity.direction as i32 + 2) % 8) as u8;
                            }
                        }
                    }
                }
            }

        }
    }

    pub(crate) fn process_caster_replays(&mut self) {
        let now = self.start_time.elapsed().as_secs_f32();
        for entity in self.game.entities.iter_mut() {
            let mut replay_skill_id = None;
            let before_count = entity.pending_attack_replays.len();
            entity.pending_attack_replays.retain(|&(fire_at, skill_id)| {
                if now >= fire_at {
                    replay_skill_id = Some(skill_id);
                    false
                } else {
                    true
                }
            });
            if let Some(skill_id) = replay_skill_id {
                let after_count = entity.pending_attack_replays.len();
                tracing::info!("Caster replay fired for entity {}: state={:?}, drained {} replays, {} remaining",
                    entity.id, entity.state, before_count - after_count, after_count);
                entity.enter_attack_replay(skill_id);
            }
        }
    }

    pub(crate) fn emit_damage_number(&mut self, entity_id: u32, hit: &ScheduledHit) {
        let is_multi_hit = matches!(hit.message, DamageMessage::AttackedMultiHit { .. });
        let is_miss = hit.damage == 0 && !is_multi_hit;

        let display_entity = if is_miss && hit.attacker_gid != 0 {
            hit.attacker_gid
        } else {
            entity_id
        };

        let target_pos = self.game.entities.get(entity_id).map(|e| e.movement.cell_position());
        let attacker_pos = self.game.entities.get(hit.attacker_gid).map(|e| e.movement.cell_position());
        let dir = match (attacker_pos, target_pos) {
            (Some(ap), Some(tp)) => direction_from_positions(ap.0, ap.1, tp.0, tp.1).unwrap_or(0),
            _ => self.game.entities.get(entity_id).map(|e| e.direction).unwrap_or(0),
        };
        let is_player_target = self.game.entities.get(entity_id)
            .map(|e| e.entity_type == ragnarok_game::entity::EntityType::Player)
            .unwrap_or(false);
        self.game.damage_numbers.emit(display_entity, dir, hit, is_player_target);
    }
}
