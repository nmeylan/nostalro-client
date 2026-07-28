use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::class::JobName;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::effect::{derive_hit_effect, is_trail_effect};
use ragnarok_game::entity::EntityState;
use ragnarok_game::movement::direction_from_positions;
use ragnarok_game::scheduled_hit::{DamageMessage, ScheduledHit};
use ragnarok_game::skill::skill_needs_talkbox;
use ragnarok_network::{
    build_pickup_item_packet, build_use_skill_packet, build_use_skill_to_ground_packet,
};
use ragnarok_ui_component::game::skill_talkbox_dialog::SkillTalkboxDialog;

const DIRECTION_COUNT: u8 = 8;
const QUARTER_TURN_DIRECTIONS: u8 = DIRECTION_COUNT / 4;

const EFST_KYRIE: i16 = 19;
const EFST_PARRYING: i16 = 104;

impl App {
    pub(crate) fn check_pending_attack(&mut self, delta: f32) {
        self.game.combat.attack_request_cooldown =
            (self.game.combat.attack_request_cooldown - delta).max(0.0);

        if self.game.pending_casts.pending_skill_id.is_some() {
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
            .is_some_and(|e| e.state != EntityState::Dead && !e.is_fading());
        if !target_alive {
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

        let range = self.game.combat.attack_range as i32;
        let dx = (px as i32 - target_pos.0 as i32).abs();
        let dy = (py as i32 - target_pos.1 as i32).abs();
        let dist = dx.max(dy);

        if dist <= range {
            let player_state = self
                .game
                .world
                .entities
                .player()
                .map(|e| e.state)
                .unwrap_or(EntityState::Standing);
            if matches!(
                player_state,
                EntityState::Standing | EntityState::ReadyFight
            ) {
                self.send_attack_packet(target_id);
                self.game.combat.attack_request_cooldown = 0.3;
            }
        } else if let Some(player) = self.game.world.entities.player()
            && !matches!(
                player.state,
                EntityState::Casting
                    | EntityState::SkillExec
                    | EntityState::Dead
                    | EntityState::Sitting
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
        let (skill_id, level) = match (
            self.game.pending_casts.pending_skill_id,
            self.game.pending_casts.pending_skill_level,
        ) {
            (Some(sid), Some(lvl)) => (sid, lvl),
            _ => return,
        };

        let target_id = match self.game.combat.attack_target_id {
            Some(id) => id,
            None => {
                self.game.pending_casts.pending_skill_id = None;
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
            self.game.pending_casts.pending_skill_id = None;
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
            .resolve_cast_skill(skill_id)
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
            if self.skill_on_cooldown(skill_id) {
                return;
            }
            self.channel.send_packet(build_use_skill_packet(
                skill_id,
                level,
                target_id,
                self.active_packetver,
            ));
            self.game.pending_casts.pending_skill_id = None;
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
        let (skill_id, level, x, y) = match self.game.pending_casts.pending_ground_cast {
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
            .resolve_cast_skill(skill_id)
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
            if self.skill_on_cooldown(skill_id) {
                return;
            }
            self.cast_on_ground(skill_id, level, x, y);
            self.game.pending_casts.pending_ground_cast = None;
        } else {
            self.try_move_toward(x as i32, y as i32, px, py, skill_range);
        }
    }

    /// Places a ground skill, first collecting the message for the skills that write
    /// one onto the unit.
    pub(crate) fn cast_on_ground(&mut self, skill_id: u16, level: i16, x: i16, y: i16) {
        if skill_needs_talkbox(skill_id) {
            self.windows.skill_talkbox_dialog =
                Some(SkillTalkboxDialog::new(skill_id, level, x, y));
            return;
        }
        self.channel.send_packet(build_use_skill_to_ground_packet(
            skill_id,
            level,
            x,
            y,
            self.active_packetver,
        ));
    }

    pub(crate) fn check_pending_pickup(&mut self) {
        let item_id = match self.game.pending_casts.pending_pickup_item_id {
            Some(id) => id,
            None => return,
        };
        if !self.game.world.floor_items.contains_key(&item_id) {
            self.game.pending_casts.pending_pickup_item_id = None;
            return;
        }
        let (px, py) = self
            .game
            .world
            .entities
            .player()
            .map(|e| e.movement.cell_position())
            .unwrap_or((0, 0));
        let floor_item = &self.game.world.floor_items[&item_id];
        let dx = (px as i32 - floor_item.x as i32).unsigned_abs();
        let dy = (py as i32 - floor_item.y as i32).unsigned_abs();
        if dx <= 1 && dy <= 1 {
            self.channel
                .send_packet(build_pickup_item_packet(item_id, self.active_packetver));
            if let Some(entity) = self.game.world.entities.player_mut() {
                entity.movement.stop();
                entity.enter_pickup(0.5);
            }
            self.game.pending_casts.pending_pickup_item_id = None;
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
                if hit.damage > 0 {
                    self.queue_hit_sound(entity_id, hit.attacker_gid, hit.skill_id != 0);
                }

                if matches!(
                    hit.message,
                    DamageMessage::Attacked | DamageMessage::AttackedMultiHit { .. }
                ) && hit.damage > 0
                {
                    let is_sonic_or_chain = hit.skill_id == SkillEnum::AsSonicblow.id() as u16
                        || hit.skill_id == SkillEnum::MoChaincombo.id() as u16;
                    let attacker_pos = self
                        .game
                        .world
                        .entities
                        .get(hit.attacker_gid)
                        .map(|e| e.movement.cell_position());
                    if let Some(entity) = self.game.world.entities.get_mut(entity_id) {
                        if !is_sonic_or_chain && let Some(ap) = attacker_pos {
                            let tp = entity.movement.cell_position();
                            if let Some(dir) = direction_from_positions(tp.0, tp.1, ap.0, ap.1) {
                                entity.direction = dir;
                            }
                        }
                        entity.enter_hurt(hit.attacked_mt_secs);

                        if is_sonic_or_chain {
                            entity.direction = ((entity.direction as i32 + 2) % 8) as u8;
                        }
                    }
                }
            }
        }
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
            let mut replay_skill_id = None;
            let before_count = entity.pending_attack_replays.len();
            entity
                .pending_attack_replays
                .retain(|&(fire_at, skill_id)| {
                    if now >= fire_at {
                        replay_skill_id = Some(skill_id);
                        false
                    } else {
                        true
                    }
                });
            if let Some(skill_id) = replay_skill_id {
                let after_count = entity.pending_attack_replays.len();
                tracing::info!(
                    "Caster replay fired for entity {}: state={:?}, drained {} replays, {} remaining",
                    entity.id,
                    entity.state,
                    before_count - after_count,
                    after_count
                );
                entity.enter_attack_replay(skill_id);
            }
        }
    }

    fn spawn_hit_effect(&mut self, entity_id: u32, hit: &ScheduledHit) {
        if hit.damage <= 0 {
            return;
        }
        let skill = (hit.skill_id != 0).then(|| SkillEnum::from_id(hit.skill_id as u32));
        let attacker_job = self
            .game
            .world
            .entities
            .get(hit.attacker_gid)
            .and_then(|e| JobName::try_from_value(e.job as usize).ok())
            .unwrap_or(JobName::Novice);
        let target_is_self = hit.attacker_gid == entity_id;
        let target_pos = self.entity_world_pos(entity_id);
        let attacker_pos = self.entity_world_pos(hit.attacker_gid);
        let markers = derive_hit_effect(skill, hit.is_critical, attacker_job, target_is_self);
        if markers.spins_target
            && let Some(entity) = self.game.world.entities.get_mut(entity_id)
        {
            entity.direction = (entity.direction + QUARTER_TURN_DIRECTIONS) % DIRECTION_COUNT;
        }
        for effect in markers.iter() {
            match (is_trail_effect(effect), attacker_pos, target_pos) {
                (true, Some(from), Some(to)) if !target_is_self => {
                    self.effect_queue.spawn_trail(effect, from, to);
                }
                _ => self.effect_queue.spawn_on(effect, entity_id),
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

        let display_entity = if is_miss && hit.attacker_gid != 0 {
            hit.attacker_gid
        } else {
            entity_id
        };

        let target_pos = self
            .game
            .world
            .entities
            .get(entity_id)
            .map(|e| e.movement.cell_position());
        let attacker_pos = self
            .game
            .world
            .entities
            .get(hit.attacker_gid)
            .map(|e| e.movement.cell_position());
        let dir = match (attacker_pos, target_pos) {
            (Some(ap), Some(tp)) => direction_from_positions(ap.0, ap.1, tp.0, tp.1).unwrap_or(0),
            _ => self
                .game
                .world
                .entities
                .get(entity_id)
                .map(|e| e.direction)
                .unwrap_or(0),
        };
        let is_player_target = self
            .game
            .world
            .entities
            .get(entity_id)
            .map(|e| e.entity_type == ragnarok_game::entity::EntityType::Player)
            .unwrap_or(false);
        let is_player_attacker = self.game.world.entities.player_id() == Some(hit.attacker_gid);
        if is_miss && self.blocks_incoming_blow(entity_id, hit.attacker_gid) {
            self.effect_queue.spawn_on(EffectId::Guard, entity_id);
            return;
        }
        self.game.combat.damage_numbers.emit(
            display_entity,
            dir,
            hit,
            is_player_target,
            is_player_attacker,
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
