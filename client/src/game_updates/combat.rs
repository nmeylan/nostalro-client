use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::class::JobName;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::effect::{derive_hit_effect, is_trail_effect};
use ragnarok_game::entity::EntityState;
use ragnarok_game::path::{in_attack_range, try_move_to};
use ragnarok_game::scheduled_hit::{DamageMessage, ScheduledHit};
use ragnarok_game::skill::skill_needs_talkbox;
use ragnarok_network::{
    build_pickup_item_packet, build_request_move_packet, build_use_skill_packet,
    build_use_skill_to_ground_packet,
};
use ragnarok_ui_component::game::skill_talkbox_dialog::SkillTalkboxDialog;

const EFST_KYRIE: i16 = 19;
const EFST_PARRYING: i16 = 104;

impl App {
    pub(crate) fn check_pending_attack(&mut self, delta: f32) {
        self.game.combat.attack_request_cooldown =
            (self.game.combat.attack_request_cooldown - delta).max(0.0);

        if self.game.pending_casts.pending_skill.is_some() {
            return;
        }

        let target_id = match self.game.combat.attack_target_id {
            Some(id) => id,
            None => return,
        };

        let target_alive = self
            .game
            .world
            .entities
            .get(target_id)
            .is_some_and(|e| e.state != EntityState::Dead && !e.is_fading())
            || self.game.world.trap_units.contains_key(&target_id);
        if !target_alive {
            self.game.combat.attack_target_id = None;
            return;
        }

        if self.game.casting_blocks_action()
            || self.game.world.entities.player().is_some_and(|player| {
                matches!(
                    player.state,
                    EntityState::SkillExec | EntityState::Dead | EntityState::Sitting
                )
            })
        {
            self.game.combat.attack_target_id = None;
            return;
        }

        if !self.game.combat.attack_is_locked && !self.input.left_mouse_down {
            self.stop_attacking();
            return;
        }

        if self.game.combat.attack_request_cooldown > 0.0 {
            return;
        }

        let target_pos = self.attack_target_cell(target_id).unwrap_or((0, 0));
        let (px, py) = self
            .game
            .world
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let range = self.game.combat.attack_range as i32;
        if in_attack_range(
            px as i32,
            py as i32,
            target_pos.0 as i32,
            target_pos.1 as i32,
            range,
        ) {
            let player_state = self
                .game
                .world
                .entities
                .player()
                .map(|e| e.state)
                .unwrap_or(EntityState::Standing);
            // The damage motion and the walk are animation states, not action
            // locks: the swing keeps being requested through both. Mid-swing
            // only a fresh target gets through, so switching enemies does not
            // cost a full attack motion.
            if matches!(
                player_state,
                EntityState::Standing
                    | EntityState::ReadyFight
                    | EntityState::Moving
                    | EntityState::Hurt
            ) || (player_state == EntityState::Casting && !self.game.casting_blocks_action())
                || (player_state == EntityState::Attacking
                    && self.game.combat.last_attacked_enemy != Some(target_id))
            {
                self.send_attack_packet(target_id);
                self.game.combat.attack_request_cooldown = 0.3;
            }
        } else if !self.game.casting_blocks_action()
            && let Some(player) = self.game.world.entities.player()
            && !matches!(
                player.state,
                EntityState::SkillExec | EntityState::Dead | EntityState::Sitting
            )
        {
            self.try_move_toward(target_pos.0 as i32, target_pos.1 as i32, px, py, range);
        }
    }

    /// Advances the server-driven progress bar and reports back when it empties.
    pub(crate) fn update_progress_bar(&mut self, delta: f32) {
        let Some(bar) = self.game.session.progress_bar.as_mut() else {
            return;
        };
        if bar.tick(delta) {
            self.finish_progress_bar();
        }
    }

    pub(crate) fn finish_progress_bar(&mut self) {
        if self.game.session.progress_bar.take().is_none() {
            return;
        }
        self.channel
            .send_packet(ragnarok_network::build_progress_done_packet(
                self.active_packetver,
            ));
    }

    pub(crate) fn check_pending_skill(&mut self) {
        let (skill, level) = match (
            self.game.pending_casts.pending_skill,
            self.game.pending_casts.pending_skill_level,
        ) {
            (Some(skill), Some(lvl)) => (skill, lvl),
            _ => return,
        };

        let target_id = match self.game.combat.attack_target_id {
            Some(id) => id,
            None => {
                self.game.pending_casts.pending_skill = None;
                self.game.pending_casts.pending_skill_level = None;
                return;
            }
        };

        let target_alive = self
            .game
            .world
            .entities
            .get(target_id)
            .is_some_and(|e| e.state != EntityState::Dead && !e.is_fading());
        if !target_alive {
            self.game.pending_casts.pending_skill = None;
            self.game.pending_casts.pending_skill_level = None;
            self.game.combat.attack_target_id = None;
            return;
        }

        if let Some(player) = self.game.world.entities.player()
            && matches!(
                player.state,
                EntityState::Casting
                    | EntityState::SkillExec
                    | EntityState::Dead
                    | EntityState::Sitting
            )
        {
            return;
        }

        let target_pos = self
            .game
            .world
            .entities
            .get(target_id)
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
            .resolve_cast_skill(skill)
            .map(|(_, range)| range as i32)
            .unwrap_or(1);
        let dx = (px as i32 - target_pos.0 as i32).abs();
        let dy = (py as i32 - target_pos.1 as i32).abs();
        let dist = dx.max(dy);

        let path_completed = self
            .game
            .world
            .entities
            .player()
            .is_some_and(|p| !p.movement.is_moving());

        if dist <= skill_range || path_completed {
            if let Some(player) = self.game.world.entities.player_mut() {
                player.movement.stop();
            }
            if self.skill_on_cooldown(skill) {
                return;
            }
            self.channel.send_packet(build_use_skill_packet(
                skill,
                level,
                target_id,
                self.active_packetver,
            ));
            self.game.pending_casts.pending_skill = None;
            self.game.pending_casts.pending_skill_level = None;
            self.game.combat.attack_target_id = None;
        } else {
            self.try_move_toward(
                target_pos.0 as i32,
                target_pos.1 as i32,
                px,
                py,
                skill_range,
            );
        }
    }

    pub(crate) fn check_pending_ground_skill(&mut self) {
        let (skill, level, x, y) = match self.game.pending_casts.pending_ground_cast {
            Some(v) => v,
            None => return,
        };

        if let Some(player) = self.game.world.entities.player()
            && matches!(
                player.state,
                EntityState::Casting
                    | EntityState::SkillExec
                    | EntityState::Dead
                    | EntityState::Sitting
            )
        {
            return;
        }

        let (px, py) = self
            .game
            .world
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let skill_range = self
            .game
            .resolve_cast_skill(skill)
            .map(|(_, range)| range as i32)
            .unwrap_or(1);

        let dx = (px as i32 - x as i32).abs();
        let dy = (py as i32 - y as i32).abs();
        let path_completed = self
            .game
            .world
            .entities
            .player()
            .is_some_and(|p| !p.movement.is_moving());

        if dx.max(dy) <= skill_range || path_completed {
            if let Some(player) = self.game.world.entities.player_mut() {
                player.movement.stop();
            }
            if self.skill_on_cooldown(skill) {
                return;
            }
            self.cast_on_ground(skill, level, x, y);
            self.game.pending_casts.pending_ground_cast = None;
        } else {
            self.try_move_toward(x as i32, y as i32, px, py, skill_range);
        }
    }

    pub(crate) fn check_pending_skill_unit_cast(&mut self) {
        let (skill, level, unit_id) = match self.game.pending_casts.pending_skill_unit_cast {
            Some(v) => v,
            None => return,
        };

        let Some(cell) = self.game.world.trap_units.get(&unit_id).map(|t| t.cell) else {
            self.game.pending_casts.pending_skill_unit_cast = None;
            return;
        };

        if let Some(player) = self.game.world.entities.player()
            && matches!(
                player.state,
                EntityState::Casting
                    | EntityState::SkillExec
                    | EntityState::Dead
                    | EntityState::Sitting
            )
        {
            return;
        }

        let (px, py) = self
            .game
            .world
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));

        let skill_range = self
            .game
            .resolve_cast_skill(skill)
            .map(|(_, range)| range as i32)
            .unwrap_or(1);

        let dx = (px as i32 - cell.0 as i32).abs();
        let dy = (py as i32 - cell.1 as i32).abs();
        let path_completed = self
            .game
            .world
            .entities
            .player()
            .is_some_and(|p| !p.movement.is_moving());

        if dx.max(dy) <= skill_range || path_completed {
            if let Some(player) = self.game.world.entities.player_mut() {
                player.movement.stop();
            }
            if self.skill_on_cooldown(skill) {
                return;
            }
            self.channel.send_packet(build_use_skill_packet(
                skill,
                level,
                unit_id,
                self.active_packetver,
            ));
            self.game.pending_casts.pending_skill_unit_cast = None;
        } else {
            self.try_move_toward(cell.0 as i32, cell.1 as i32, px, py, skill_range);
        }
    }

    /// Places a ground skill, first collecting the message for the skills that write
    /// one onto the unit.
    pub(crate) fn cast_on_ground(&mut self, skill: SkillEnum, level: i16, x: i16, y: i16) {
        if skill_needs_talkbox(skill) {
            self.windows.skill_talkbox_dialog = Some(SkillTalkboxDialog::new(skill, level, x, y));
            return;
        }
        self.channel.send_packet(build_use_skill_to_ground_packet(
            skill,
            level,
            x,
            y,
            self.active_packetver,
        ));
    }

    /// Drives the standing pickup intent: waits out the walk, then either grabs an
    /// adjacent item or walks another step toward it. Re-evaluated every frame
    /// until the item is taken or gone.
    pub(crate) fn check_pending_pickup(&mut self) {
        let Some(item_id) = self.game.pending_casts.pending_pickup_item_id else {
            return;
        };
        let Some(floor_item) = self.game.world.floor_items.get(&item_id) else {
            self.game.pending_casts.pending_pickup_item_id = None;
            return;
        };
        let (item_x, item_y) = (floor_item.x as i32, floor_item.y as i32);
        let Some(player) = self.game.world.entities.player() else {
            return;
        };
        if player.movement.is_moving() || player.is_move_locked() {
            return;
        }
        let (px, py) = player.movement.cell_position();
        let dx = (px as i32 - item_x).unsigned_abs();
        let dy = (py as i32 - item_y).unsigned_abs();
        if dx <= 1 && dy <= 1 {
            self.channel
                .send_packet(build_pickup_item_packet(item_id, self.active_packetver));
            self.game.pending_casts.pending_pickup_item_id = None;
            return;
        }
        let Some(gat) = &self.game.session.gat else {
            return;
        };
        match try_move_to(gat, px, py, item_x, item_y) {
            Some(move_action) => self.channel.send_packet(build_request_move_packet(
                move_action.dest_x,
                move_action.dest_y,
                self.active_packetver,
            )),
            None => self.game.pending_casts.pending_pickup_item_id = None,
        }
    }

    pub(crate) fn process_scheduled_hits(&mut self) {
        let now = self.start_time.elapsed().as_secs_f32();
        let entity_ids: Vec<u32> = self.game.world.entities.iter().map(|e| e.id).collect();
        for entity_id in entity_ids {
            let ready = if let Some(entity) = self.game.world.entities.get_mut(entity_id) {
                entity.scheduled_hits.drain_ready(now)
            } else {
                continue;
            };
            for hit in ready {
                self.emit_damage_number(entity_id, &hit);
                self.spawn_hit_effect(entity_id, &hit);
                if hit.damage > 0 && hit.attacker_gid != entity_id {
                    self.queue_hit_sound(entity_id, hit.attacker_gid, hit.skill.is_some());
                }

                if matches!(
                    hit.message,
                    DamageMessage::Attacked | DamageMessage::AttackedMultiHit { .. }
                ) && hit.damage > 0
                {
                    let is_sonic_or_chain = matches!(
                        hit.skill,
                        Some(SkillEnum::AsSonicblow | SkillEnum::MoChaincombo)
                    );
                    let hurt_secs = self
                        .game
                        .world
                        .entities
                        .get(entity_id)
                        .zip(self.game.sprite_caches.sprites.get(&entity_id))
                        .and_then(|(entity, sprite)| {
                            sprite
                                .body_act
                                .action_group_duration_ms(entity.hurt_action_group())
                        })
                        .map(|ms| ms / 1000.0);
                    if let Some(entity) = self.game.world.entities.get_mut(entity_id) {
                        entity.enter_hurt(hit.attacked_mt_secs, hurt_secs);

                        if is_sonic_or_chain {
                            entity.spin_quarter_turn();
                        }
                    }
                }
            }
        }
        self.process_skill_unit_hits(now);
    }

    fn process_skill_unit_hits(&mut self, now: f32) {
        let unit_ids: Vec<u32> = self.game.world.skill_unit_hits.keys().copied().collect();
        for unit_id in unit_ids {
            let ready = match self.game.world.skill_unit_hits.get_mut(&unit_id) {
                Some(queue) => queue.drain_ready(now),
                None => continue,
            };
            for hit in ready {
                self.emit_damage_number(unit_id, &hit);
                self.spawn_hit_effect(unit_id, &hit);
                if hit.damage > 0 {
                    self.queue_hit_sound(unit_id, hit.attacker_gid, hit.skill.is_some());
                }
            }
        }
        self.game
            .world
            .skill_unit_hits
            .retain(|_, queue| !queue.is_empty());
    }

    pub(crate) fn update_arrows(&mut self, delta: f32) {
        for arrow in &mut self.game.world.arrows {
            arrow.advance(delta);
        }
        self.game.world.arrows.retain(|a| !a.is_done());
    }

    pub(crate) fn process_caster_replays(&mut self) {
        let now = self.start_time.elapsed().as_secs_f32();
        for entity in self.game.world.entities.iter_mut() {
            let mut replay_skill = None;
            let before_count = entity.pending_attack_replays.len();
            entity.pending_attack_replays.retain(|&(fire_at, skill)| {
                if now >= fire_at {
                    replay_skill = Some(skill);
                    false
                } else {
                    true
                }
            });
            if let Some(skill) = replay_skill {
                let after_count = entity.pending_attack_replays.len();
                tracing::info!(
                    "Caster replay fired for entity {}: state={:?}, drained {} replays, {} remaining",
                    entity.id,
                    entity.state,
                    before_count - after_count,
                    after_count
                );
                entity.enter_attack_replay(skill);
            }
        }
    }

    fn spawn_hit_effect(&mut self, entity_id: u32, hit: &ScheduledHit) {
        if hit.damage <= 0 {
            return;
        }
        let skill = hit.skill;
        let attacker_job = self
            .game
            .world
            .entities
            .get(hit.attacker_gid)
            .and_then(|e| JobName::try_from_value(e.job as usize).ok())
            .unwrap_or(JobName::Novice);
        let target_is_self = hit.attacker_gid == entity_id;
        let unit_pos = self.skill_unit_world_pos(entity_id);
        let target_pos = self.entity_world_pos(entity_id).or(unit_pos);
        let attacker_pos = self.entity_world_pos(hit.attacker_gid);
        let markers = derive_hit_effect(
            skill,
            hit.is_critical,
            attacker_job,
            target_is_self,
            hit.hit_index,
        );
        if markers.spins_target
            && let Some(entity) = self.game.world.entities.get_mut(entity_id)
        {
            entity.spin_quarter_turn();
        }
        for effect in markers.iter() {
            match (is_trail_effect(effect), attacker_pos, target_pos) {
                (true, Some(from), Some(to)) if !target_is_self => {
                    self.effect_queue.spawn_trail(effect, from, to);
                }
                _ => match unit_pos {
                    Some(pos) => self.effect_queue.spawn_at(effect, pos),
                    None => self.effect_queue.spawn_on(effect, entity_id),
                },
            }
        }
    }

    pub(crate) fn emit_damage_number(&mut self, entity_id: u32, hit: &ScheduledHit) {
        const IGNORE_DAMAGE: i32 = -30000;
        const NEVERSEE_DAMAGE: i32 = -29999;
        if hit.damage == IGNORE_DAMAGE || hit.damage == NEVERSEE_DAMAGE {
            return;
        }
        if !self.config.display.show_other_damage {
            let me = self.game.world.entities.player_id();
            if me != Some(entity_id) && me != Some(hit.attacker_gid) {
                return;
            }
        }
        let is_miss = match hit.message {
            DamageMessage::AttackedMultiHit { total_damage } => total_damage == 0,
            _ => hit.damage == 0,
        };
        if is_miss && !self.game.prefs.show_miss {
            return;
        }

        // A miss belongs to whoever swung, not to whoever dodged.
        let display_entity = if is_miss && hit.attacker_gid != 0 {
            hit.attacker_gid
        } else {
            entity_id
        };

        // The number hangs off the actor that took the hit, and drifts by that
        // actor's own facing, not by where the blow came from.
        let dir = self
            .game
            .world
            .entities
            .get(entity_id)
            .map(|e| e.facing_degrees)
            .unwrap_or(0.0);
        let is_player_target = self
            .game
            .world
            .entities
            .get(entity_id)
            .map(|e| e.entity_type == ragnarok_game::entity::EntityType::Player)
            .unwrap_or(false);
        // Only a player's own swings redden their miss; a monster's stay white.
        let attacker_is_player = self
            .game
            .world
            .entities
            .get(hit.attacker_gid)
            .is_some_and(|e| e.entity_type == ragnarok_game::entity::EntityType::Player);
        if is_miss && self.blocks_incoming_blow(entity_id, hit.attacker_gid) {
            self.effect_queue.spawn_on(EffectId::Guard, entity_id);
            return;
        }
        self.game.combat.damage_numbers.emit(
            display_entity,
            dir,
            hit,
            is_player_target,
            attacker_is_player,
        );
    }

    /// Whether the blow reads as absorbed rather than missed: only the local
    /// player, only under Kyrie Eleison or Parrying, and only against a monster.
    fn blocks_incoming_blow(&self, defender_gid: u32, attacker_gid: u32) -> bool {
        if self.game.world.entities.player_id() != Some(defender_gid) {
            return false;
        }
        let attacker_is_monster = self
            .game
            .world
            .entities
            .get(attacker_gid)
            .is_some_and(|e| e.entity_type == ragnarok_game::entity::EntityType::Monster);
        attacker_is_monster
            && self
                .game
                .character
                .active_statuses
                .iter()
                .any(|s| s.efst == EFST_KYRIE || s.efst == EFST_PARRYING)
    }
}
