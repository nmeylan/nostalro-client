use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::action::ActionType;
use models::enums::class::JobName;
use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::effect_id::EffectId;
use models::enums::vanish::VanishType;
use models::enums::weapon::WeaponType;
use ragnarok_game::ailment;
use ragnarok_game::arrow::{ArrowProjectile, flight_secs_for_cell_distance};
use ragnarok_game::damage_number::{DamageNumber, DamageNumberType};
use ragnarok_game::effect::buff_effect;
use ragnarok_game::effect::{
    UNT_USED_TRAPS, skill_unit_effect, trap_model_name, trap_trigger_effect,
};
use ragnarok_game::entity::{Entity, EntityState, EntityType};
use ragnarok_game::level_aura;
use ragnarok_game::movement::direction_from_positions;
use ragnarok_game::scheduled_hit::{DamageMessage, ScheduledHit};
use ragnarok_game::sprite_path::{
    JT_WARPNPC, OPTION_RIDING, OPTION_RUWACH, OPTION_SIGHT, cart_design_from_option,
    entity_type_from_job, has_falcon, is_hidden, visual_job,
};
use ragnarok_game::status_icon::status_icon_info;

const LEVEL_AURA_LAYERS: &[EffectId] = &[EffectId::Level99, EffectId::Level992, EffectId::Level993];
const BOSS_AURA_LAYERS: &[EffectId] = &[EffectId::Green995, EffectId::Green996, EffectId::Level993];

impl App {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_entity_spawned(
        &mut self,
        gid: u32,
        job: u16,
        speed: u16,
        sex: u8,
        head: u16,
        weapon: u16,
        shield: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        hair_color: u16,
        x: u16,
        y: u16,
        direction: u8,
        body_state: i16,
        health_state: i16,
        effect_state: i32,
        base_level: i16,
        is_boss: bool,
        posture: u8,
    ) {
        if self.game.entities.player_id() == Some(gid) {
            if effect_state != 0 {
                self.handle_entity_option_changed(gid, body_state, health_state, effect_state);
            }
            return;
        }
        let stale = self
            .game
            .entities
            .get(gid)
            .is_some_and(|e| e.state == EntityState::Dead || e.is_fading());
        if stale {
            self.despawn_entity_effects(gid);
            self.game.entities.remove(gid);
            self.game.sprites.remove(&gid);
        } else if let Some(existing) = self.game.entities.get_mut(gid) {
            existing.movement.set_speed(speed);
            if existing.effect_state != effect_state {
                self.handle_entity_option_changed(gid, body_state, health_state, effect_state);
            }
            return;
        }
        let entity_type = entity_type_from_job(job);
        let mut entity = Entity::new(
            gid,
            entity_type,
            job,
            sex,
            head,
            hair_color,
            weapon,
            head_top,
            head_mid,
            head_bottom,
            shield,
            x,
            y,
            direction,
            speed,
        );
        entity.effect_state = effect_state;
        entity.body_state = body_state;
        entity.health_state = health_state;
        entity.base_level = base_level;
        entity.is_boss = is_boss;
        match posture {
            1 => entity.state = EntityState::Dead,
            2 => entity.state = EntityState::Sitting,
            _ => {}
        }
        self.game.entities.insert(entity);
        let sprite_job = visual_job(job, effect_state);
        self.load_entity_sprite(
            gid,
            entity_type,
            sprite_job,
            sex,
            head,
            weapon,
            shield,
            head_top,
            head_mid,
            head_bottom,
            hair_color,
            direction,
        );
        if entity_type == EntityType::Player && !is_hidden(effect_state) {
            self.effect_queue.spawn_on(EffectId::Entry2, gid);
        }
        if let Some(design) = cart_design_from_option(effect_state) {
            if let Some(entity) = self.game.entities.get_mut(gid) {
                entity.cart_type = Some(design);
            }
            self.spawn_cart_visual(gid, design);
        }
        if has_falcon(effect_state) {
            self.spawn_falcon_visual(gid);
        }
        self.refresh_level_aura(gid);
        self.refresh_boss_aura(gid);
        if entity_type == EntityType::Npc && job == JT_WARPNPC {
            self.spawn_warp_portal(gid);
        }
    }

    pub(super) fn handle_entity_moved(
        &mut self,
        gid: u32,
        start_x: u16,
        start_y: u16,
        dest_x: u16,
        dest_y: u16,
        start_time: u32,
    ) {
        let local_ms = self.start_time.elapsed().as_millis() as u32;
        self.game
            .server_time
            .observe_server_tick(start_time, local_ms);
        let already_moving_to_dest = self
            .game
            .entities
            .get(gid)
            .filter(|e| e.movement.is_moving())
            .and_then(|e| e.movement.destination())
            .is_some_and(|(dx, dy)| dx == dest_x && dy == dest_y);
        if already_moving_to_dest {
            return;
        }
        if let Some(gat) = &self.game.gat {
            let (sx, sy) = self
                .game
                .entities
                .get(gid)
                .map(|e| e.movement.cell_position())
                .unwrap_or((start_x, start_y));
            let path = ragnarok_game::path::path_search(gat, sx, sy, dest_x, dest_y);
            if !path.is_empty() {
                let now = local_ms as f32 / 1000.0;
                if let Some(entity) = self.game.entities.get_mut(gid) {
                    entity.movement.start_move(path, now);
                    entity.state_timer = 0.0;
                }
            }
        }
    }

    pub(super) fn handle_entity_vanished(&mut self, gid: u32, vanish_type: VanishType) {
        if self.game.attack_target_id == Some(gid) {
            self.game.attack_target_id = None;
        }
        match vanish_type {
            VanishType::Die => {
                if let Some(entity) = self.game.entities.get_mut(gid) {
                    entity.request_pending_death();
                }
            }
            VanishType::OutOfSight => {
                if let Some(entity) = self.game.entities.get_mut(gid) {
                    entity.start_vanish_fade();
                    tracing::debug!("EntityVanished(outofsight): gid={gid}");
                }
            }
            _ => {
                let poof = match vanish_type {
                    VanishType::Teleport => {
                        let hidden = self
                            .game
                            .entities
                            .get(gid)
                            .is_some_and(|e| is_hidden(e.effect_state));
                        (!hidden).then_some(EffectId::Teleportation2)
                    }
                    VanishType::Loggout => Some(EffectId::Exit2),
                    _ => None,
                };
                if let Some(effect) = poof
                    && let Some(pos) = self.entity_world_pos(gid)
                {
                    self.effect_queue.spawn_at(effect, pos);
                }
                self.despawn_entity_effects(gid);
                let r1 = self.game.entities.remove(gid).is_some();
                let r2 = self.game.sprites.remove(&gid).is_some();
                tracing::debug!("EntityVanished: gid={gid} type={vanish_type:?} r1={r1} r2={r2}");
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_entity_action(
        &mut self,
        gid: u32,
        target_gid: u32,
        action: ActionType,
        damage: i32,
        left_damage: i32,
        attack_mt: i32,
        attacked_mt: i32,
        count: i16,
        start_time: u32,
    ) {
        let local_ms = self.start_time.elapsed().as_millis() as u32;
        self.game
            .server_time
            .observe_server_tick(start_time, local_ms);
        let action_start = self
            .game
            .server_time
            .server_to_local_secs_clamped(start_time, local_ms);
        let local_now = local_ms as f32 / 1000.0;
        let age = (local_now - action_start).max(0.0);
        match action {
            ActionType::Sit => {
                if let Some(entity) = self.game.entities.get_mut(gid) {
                    entity.state = EntityState::Sitting;
                    entity.state_timer = 0.0;
                }
            }
            ActionType::Stand => {
                if let Some(entity) = self.game.entities.get_mut(gid) {
                    entity.state = EntityState::Standing;
                    entity.state_timer = 0.0;
                }
            }
            ActionType::Attack
            | ActionType::AttackNomotion
            | ActionType::AttackRepeat
            | ActionType::AttackMultiple
            | ActionType::AttackMultipleNomotion
            | ActionType::AttackCritical => {
                let target_pos = self
                    .game
                    .entities
                    .get(target_gid)
                    .map(|e| e.movement.cell_position());
                let mut shooter_cell = None;
                if let Some(entity) = self.game.entities.get_mut(gid) {
                    if let Some(tp) = target_pos {
                        let sp = entity.movement.cell_position();
                        if let Some(dir) = direction_from_positions(sp.0, sp.1, tp.0, tp.1) {
                            entity.direction = dir;
                        }
                    }
                    let duration = ((attack_mt as f32 / 1000.0) - age).max(0.5);
                    entity.enter_attack(duration);
                    if entity.weapon == Some(WeaponType::Bow) {
                        shooter_cell = Some(entity.movement.cell_position());
                    }
                }
                let is_endure = matches!(
                    action,
                    ActionType::AttackNomotion | ActionType::AttackMultipleNomotion
                );
                let effective_count = match action {
                    ActionType::AttackMultiple | ActionType::AttackMultipleNomotion => {
                        count.max(1) as u16
                    }
                    _ => 1,
                };
                if let (Some(sc), Some(tp)) = (shooter_cell, target_pos) {
                    self.spawn_arrow_projectile(sc, tp, attack_mt, effective_count);
                }
                let total_damage = damage + left_damage;
                let now = action_start;
                let delay_time = (attack_mt as f32 / 1000.0).max(0.0);
                let per_hit_damage = if effective_count > 1 && total_damage > 0 {
                    total_damage / effective_count as i32
                } else {
                    total_damage
                };

                let is_critical = matches!(action, ActionType::AttackCritical);
                if let Some(target) = self.game.entities.get_mut(target_gid) {
                    let double_attack_term = 0.2;
                    for i in 0..effective_count {
                        let hit_time = now + delay_time + (i as f32 * double_attack_term);
                        let msg = if is_endure {
                            DamageMessage::AttackedNoMotion
                        } else if effective_count > 1 {
                            DamageMessage::AttackedMultiHit { total_damage }
                        } else {
                            DamageMessage::Attacked
                        };
                        target.scheduled_hits.push(ScheduledHit {
                            message: msg,
                            damage: per_hit_damage,
                            fire_at: hit_time,
                            attacker_gid: gid,
                            skill_id: 0,
                            is_last_hit: i == effective_count - 1,
                            is_critical,
                            hit_index: i,
                            attacked_mt_secs: attacked_mt as f32 / 1000.0,
                        });
                    }
                }
            }
            ActionType::AttackLucky => {
                let dir = self
                    .game
                    .entities
                    .get(target_gid)
                    .map(|e| e.direction)
                    .unwrap_or(0);
                self.game.damage_numbers.add(DamageNumber::new(
                    target_gid,
                    0,
                    DamageNumberType::Lucky,
                    dir,
                ));
            }
            ActionType::Itempickup => {
                if let Some(entity) = self.game.entities.get_mut(gid) {
                    entity.enter_pickup(0.5);
                }
            }
            _ => {}
        }
    }

    pub(super) fn spawn_arrow_projectile(
        &mut self,
        shooter: (u16, u16),
        target: (u16, u16),
        attack_mt: i32,
        count: u16,
    ) {
        let (Some(gat), Some(coords)) = (&self.game.gat, &self.game.map_coords) else {
            return;
        };
        let cell_world = |x: u16, y: u16| {
            let (wx, _, wz) = coords.cell_to_world(x as f32 + 0.5, y as f32 + 0.5);
            let wy = gat.get_height(x as f32 + 0.5, y as f32 + 0.5);
            [wx, wy - 10.0, wz]
        };
        let from = cell_world(shooter.0, shooter.1);
        let to = cell_world(target.0, target.1);
        let dx = target.0 as f32 - shooter.0 as f32;
        let dy = target.1 as f32 - shooter.1 as f32;
        let dist_cells = (dx * dx + dy * dy).sqrt();
        let flight = flight_secs_for_cell_distance(dist_cells);
        let land_at = (attack_mt as f32 / 1000.0).max(flight);
        let double_attack_term = 0.2;
        for i in 0..count.max(1) {
            let delay = (land_at - flight + i as f32 * double_attack_term).max(0.0);
            self.game
                .arrows
                .push(ArrowProjectile::new(from, to, delay, flight));
        }
    }

    pub(super) fn handle_entity_hp_changed(&mut self, gid: u32, hp: u32, max_hp: u32) {
        if self.game.entities.is_player(gid) {
            self.game.character.hp = hp;
            self.game.character.max_hp = max_hp;
        } else if let Some(entity) = self.game.entities.get_mut(gid) {
            entity.hp = Some(hp);
            entity.max_hp = Some(max_hp);
        }
    }

    pub(super) fn handle_entity_option_changed(
        &mut self,
        gid: u32,
        body_state: i16,
        health_state: i16,
        effect_state: i32,
    ) {
        tracing::debug!(
            "EntityOptionChanged: gid={gid} body=0x{body_state:04x} health=0x{health_state:04x} effect_state=0x{effect_state:08x}"
        );
        if self.game.entities.is_player(gid) {
            self.game.character.effect_state = effect_state;
        }
        let is_player = self.game.entities.player_id() == Some(gid);
        let prev_health = self
            .game
            .entities
            .get(gid)
            .map(|e| e.health_state)
            .unwrap_or(0);
        let prev_body = self
            .game
            .entities
            .get(gid)
            .map(|e| e.body_state)
            .unwrap_or(0);
        if let Some(entity) = self.game.entities.get_mut(gid) {
            entity.body_state = body_state;
            entity.health_state = health_state;
        }
        {
            let was_frozen = prev_body == ailment::OPT1_FREEZE;
            let now_frozen = body_state == ailment::OPT1_FREEZE;
            if now_frozen && !was_frozen {
                // TODO(audio): play _stonecurse.wav (no audio subsystem yet)
            } else if was_frozen && !now_frozen {
                self.game.freeze_shatters.push(crate::game_state::FreezeShatter {
                    gid,
                    started_at: None,
                });
                // TODO(audio): play _frozen_explosion.wav (no audio subsystem yet)
            }
        }
        if is_player
            && let Some(player) = self.game.entities.player_mut()
            && ailment::movement_blocked(body_state, player.rooted)
        {
            player.movement.stop();
        }
        if is_player {
            let was_blind = prev_health & ailment::OPT2_BLIND != 0;
            let now_blind = health_state & ailment::OPT2_BLIND != 0;
            if now_blind && !was_blind {
                self.effect_queue.spawn_on_keyed(EffectId::Blind, gid, gid);
            } else if was_blind && !now_blind {
                self.effect_queue.despawn(gid);
            }
        }
        let (mut old_cart, mut new_cart) = (None, None);
        let (mut old_falcon, mut new_falcon) = (false, false);
        if let Some(entity) = self.game.entities.get_mut(gid) {
            let old_riding = (entity.effect_state & OPTION_RIDING) != 0;
            let new_riding = (effect_state & OPTION_RIDING) != 0;
            old_cart = cart_design_from_option(entity.effect_state);
            new_cart = cart_design_from_option(effect_state);
            old_falcon = has_falcon(entity.effect_state);
            new_falcon = has_falcon(effect_state);
            entity.cart_type = new_cart;
            tracing::debug!(
                "  old_effect=0x{:08x} old_riding={old_riding} new_riding={new_riding} job={}",
                entity.effect_state,
                entity.job
            );
            entity.effect_state = effect_state;
            if old_riding != new_riding {
                let sprite_job = visual_job(entity.job, effect_state);
                let (sex, head, shield, head_top, head_mid, head_bottom, hair_color) = (
                    entity.sex,
                    entity.head,
                    entity.shield,
                    entity.head_top,
                    entity.head_mid,
                    entity.head_bottom,
                    entity.hair_color,
                );
                let entity_type = entity.entity_type;
                if is_player {
                    let (weapon, cloth_color) = {
                        let e = self.game.entities.get(gid).unwrap();
                        (e.weapon, e.cloth_color)
                    };
                    self.load_player_sprite(
                        gid,
                        sprite_job,
                        sex,
                        head,
                        hair_color,
                        cloth_color,
                        weapon,
                        head_top,
                        head_mid,
                        head_bottom,
                        shield,
                    );
                } else {
                    self.load_entity_sprite(
                        gid,
                        entity_type,
                        sprite_job,
                        sex,
                        head,
                        0,
                        shield,
                        head_top,
                        head_mid,
                        head_bottom,
                        hair_color,
                        0,
                    );
                }
            }
        }
        if old_cart != new_cart {
            match new_cart {
                Some(design) => self.spawn_cart_visual(gid, design),
                None => self.despawn_cart_visual(gid),
            }
        }
        if old_falcon != new_falcon {
            if new_falcon {
                self.spawn_falcon_visual(gid);
            } else {
                self.despawn_falcon_visual(gid);
            }
        }
        self.refresh_level_aura(gid);
        self.refresh_boss_aura(gid);
        self.refresh_detect_aura(gid);
    }

    /// Detect-hidden auras (Sight / Ruwach): the original shows no effect at
    /// cast and instead re-launches the aura for as long as the OPTION bit is
    /// set. Reconcile each against its option bit — spawn a persistent orbit
    /// when the bit turns on, drop it when it clears.
    pub(super) fn refresh_detect_aura(&mut self, gid: u32) {
        let Some(effect_state) = self.game.entities.get(gid).map(|e| e.effect_state) else {
            return;
        };
        let want_sight = effect_state & OPTION_SIGHT != 0;
        match (want_sight, self.game.sight_aura_keys.contains_key(&gid)) {
            (true, false) => {
                let key = self.next_entity_effect_key();
                self.effect_queue.spawn_on_keyed(EffectId::Sight2, gid, key);
                self.game.sight_aura_keys.insert(gid, key);
            }
            (false, true) => {
                if let Some(key) = self.game.sight_aura_keys.remove(&gid) {
                    self.effect_queue.despawn(key);
                }
            }
            _ => {}
        }
        let want_ruwach = effect_state & OPTION_RUWACH != 0;
        match (want_ruwach, self.game.ruwach_aura_keys.contains_key(&gid)) {
            (true, false) => {
                let key = self.next_entity_effect_key();
                self.effect_queue.spawn_on_keyed(EffectId::Ruwach, gid, key);
                self.game.ruwach_aura_keys.insert(gid, key);
            }
            (false, true) => {
                if let Some(key) = self.game.ruwach_aura_keys.remove(&gid) {
                    self.effect_queue.despawn(key);
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_status_effect_changed(
        &mut self,
        gid: u32,
        efst: i16,
        active: bool,
        remain_ms: u32,
        val1: i32,
    ) {
        let Ok(icon) = ClientEffectIcon::try_from_value(efst as usize) else {
            return;
        };
        if icon == ClientEffectIcon::OnPushCart {
            self.handle_push_cart_status(gid, active, val1);
            return;
        }
        if icon == ClientEffectIcon::Run {
            let stopped = self
                .game
                .entities
                .get_mut(gid)
                .map(|e| {
                    let was_running = e.is_running;
                    e.is_running = active;
                    e.footstep_timer = 0.0;
                    was_running && !active
                })
                .unwrap_or(false);
            if stopped {
                self.effect_queue.spawn_on(EffectId::Stopeffect, gid);
            }
            return;
        }
        if icon == ClientEffectIcon::Ting {
            if active && let Some(e) = self.game.entities.get_mut(gid) {
                e.is_running = false;
                e.footstep_timer = 0.0;
                self.effect_queue.spawn_on(EffectId::Quakebody, gid);
            }
            return;
        }
        if icon == ClientEffectIcon::Mindbreaker && active {
            self.effect_queue.spawn_on(EffectId::Magiccrasher2, gid);
        }
        if self.game.entities.player_id() == Some(gid) {
            if let Some(info) = status_icon_info(efst) {
                if !active {
                    self.game.character.clear_status(efst);
                } else {
                    let path = format!("data/texture/effect/{}", info.icon);
                    let loaded = match (self.renderer.as_mut(), self.grf.as_ref()) {
                        (Some(r), Some(g)) => r.preload_textures(&[path.as_str()], g),
                        _ => false,
                    };
                    let now_ms = self.start_time.elapsed().as_millis() as u64;
                    self.game
                        .character
                        .apply_status(efst, val1, now_ms, remain_ms as u64, loaded);
                }
            }
        }
        let Some(buff) = buff_effect(icon) else {
            return;
        };
        let map_key = (gid, efst);
        if let Some(old_key) = self.game.status_buff_keys.remove(&map_key) {
            self.effect_queue.despawn(old_key);
        }
        if !active {
            return;
        }
        let key = 0x8000_0000 | self.game.next_status_buff_key;
        self.game.next_status_buff_key = (self.game.next_status_buff_key + 1) & 0x7fff_ffff;
        for &id in buff.body {
            self.effect_queue
                .spawn_on_keyed_for(id, gid, key, remain_ms);
        }
        self.game.status_buff_keys.insert(map_key, key);
    }

    fn handle_push_cart_status(&mut self, gid: u32, active: bool, val1: i32) {
        let is_player = self.game.entities.player_id() == Some(gid);
        if active {
            let design = val1.clamp(0, u8::MAX as i32) as u8;
            let had_cart = self
                .game
                .entities
                .get(gid)
                .is_some_and(|e| e.cart_type.is_some());
            if let Some(entity) = self.game.entities.get_mut(gid) {
                entity.cart_type = Some(design);
            }
            self.spawn_cart_visual(gid, design);
            if is_player {
                self.game.character.cart_design = Some(design);
                if !had_cart {
                    self.game.character.cart.open();
                }
            }
        } else if is_player {
            self.handle_cart_off();
        } else {
            if let Some(entity) = self.game.entities.get_mut(gid) {
                entity.cart_type = None;
            }
            self.despawn_cart_visual(gid);
        }
    }

    pub(super) fn refresh_level_aura(&mut self, gid: u32) {
        let Some((entity_type, effect_state, entity_level)) = self
            .game
            .entities
            .get(gid)
            .map(|e| (e.entity_type, e.effect_state, e.base_level))
        else {
            return;
        };
        let base_level = if self.game.entities.player_id() == Some(gid) {
            self.game.character.base_level as i16
        } else {
            entity_level
        };
        let want = self.config.display.show_level_aura
            && level_aura::level_aura_visible(entity_type, base_level, effect_state);
        let have = self.game.level_aura_keys.contains_key(&gid);
        match (want, have) {
            (true, false) => {
                let key = self.next_entity_effect_key();
                for &id in LEVEL_AURA_LAYERS {
                    self.effect_queue.spawn_on_keyed(id, gid, key);
                }
                self.game.level_aura_keys.insert(gid, key);
            }
            (false, true) => self.despawn_level_aura(gid),
            _ => {}
        }
    }

    pub(crate) fn despawn_level_aura(&mut self, gid: u32) {
        if let Some(key) = self.game.level_aura_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
    }

    pub(super) fn refresh_boss_aura(&mut self, gid: u32) {
        let Some((entity_type, is_boss, effect_state)) = self
            .game
            .entities
            .get(gid)
            .map(|e| (e.entity_type, e.is_boss, e.effect_state))
        else {
            return;
        };
        let want = self.config.display.show_level_aura
            && level_aura::boss_aura_visible(entity_type, is_boss, effect_state);
        let have = self.game.boss_aura_keys.contains_key(&gid);
        match (want, have) {
            (true, false) => {
                let key = self.next_entity_effect_key();
                for &id in BOSS_AURA_LAYERS {
                    self.effect_queue.spawn_on_keyed(id, gid, key);
                }
                self.game.boss_aura_keys.insert(gid, key);
            }
            (false, true) => self.despawn_boss_aura(gid),
            _ => {}
        }
    }

    pub(crate) fn despawn_boss_aura(&mut self, gid: u32) {
        if let Some(key) = self.game.boss_aura_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
    }

    pub(super) fn spawn_warp_portal(&mut self, gid: u32) {
        if self.game.warp_portal_keys.contains_key(&gid) {
            return;
        }
        let key = self.next_entity_effect_key();
        self.effect_queue
            .spawn_on_keyed(EffectId::Warpzone2, gid, key);
        self.game.warp_portal_keys.insert(gid, key);
    }

    pub(crate) fn despawn_warp_portal(&mut self, gid: u32) {
        if let Some(key) = self.game.warp_portal_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
    }

    /// Spirit spheres (Call Spirits / Explosion Spirits etc.): `count` orbiting
    /// balls, replaced whenever the server re-sends the count and cleared at 0.
    /// Champions and Gunslingers get their own sphere variant.
    pub(super) fn handle_spirits_changed(&mut self, gid: u32, count: u8) {
        if let Some(old_key) = self.game.spirit_keys.remove(&gid) {
            self.effect_queue.despawn(old_key);
        }
        if count == 0 {
            return;
        }
        let Some(job) = self
            .game
            .entities
            .get(gid)
            .and_then(|e| JobName::try_from_value(e.job as usize).ok())
        else {
            return;
        };
        let effect = match job {
            JobName::Champion => EffectId::Chookgi2,
            JobName::Gunslinger => EffectId::Chookgi3,
            _ => EffectId::Chookgi,
        };
        let key = self.next_entity_effect_key();
        self.effect_queue
            .spawn_on_keyed_with_count(effect, gid, key, count);
        self.game.spirit_keys.insert(gid, key);
    }

    pub(crate) fn despawn_entity_effects(&mut self, gid: u32) {
        self.despawn_level_aura(gid);
        self.despawn_boss_aura(gid);
        self.despawn_warp_portal(gid);
        if let Some(key) = self.game.spirit_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
        if let Some(key) = self.game.sight_aura_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
        if let Some(key) = self.game.ruwach_aura_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
    }

    fn next_entity_effect_key(&mut self) -> u32 {
        let key = 0x8000_0000 | self.game.next_status_buff_key;
        self.game.next_status_buff_key = (self.game.next_status_buff_key + 1) & 0x7fff_ffff;
        key
    }

    pub(super) fn handle_entity_sprite_changed(
        &mut self,
        gid: u32,
        sprite_type: u8,
        value: u16,
        value2: u16,
    ) {
        // A trap springs when the server changes its base look to UNT_USED_TRAPS:
        // fire the stored trigger burst at the trap cell (the trap model is
        // removed shortly after by ZC_SKILL_DISAPPEAR).
        if sprite_type == 0
            && value == UNT_USED_TRAPS as u16
            && let Some((unit_id, world)) = self.game.trap_units.remove(&gid)
        {
            if let Some(burst) = trap_trigger_effect(unit_id) {
                self.effect_queue.spawn_at(burst, world);
            }
            return;
        }

        let left_hand_is_weapon = self.game.entities.is_player(gid)
            && self
                .game
                .character
                .inventory
                .equipped_in_slot(models::enums::item::EquipmentLocation::HandLeft)
                .is_some_and(|item| item.is_weapon());
        if let Some(entity) = self.game.entities.get_mut(gid) {
            if sprite_type == 2 {
                let right_type = ragnarok_game::sprite_path::weapon_view_id_to_type(value);
                if left_hand_is_weapon {
                    let left_type = ragnarok_game::sprite_path::weapon_view_id_to_type(value2);
                    entity.weapon = match (right_type, left_type) {
                        (Some(r), Some(l)) => {
                            ragnarok_game::sprite_path::dual_wield_type(r, l).or(Some(r))
                        }
                        (None, Some(l)) => Some(l),
                        _ => right_type,
                    };
                    entity.shield = 0;
                } else {
                    entity.weapon = right_type;
                    entity.shield = value2;
                }
                tracing::debug!(
                    "LOOK_WEAPON: gid={gid} value={value} value2={value2} left_is_weapon={left_hand_is_weapon} → weapon={:?} shield={}",
                    entity.weapon,
                    entity.shield
                );
            } else {
                tracing::debug!("SpriteChange: gid={gid} type={sprite_type} value={value}");
                entity.apply_sprite_change(sprite_type, value);
            }
            let weapon_type = entity.weapon;
            let sprite_job = visual_job(entity.job, entity.effect_state);
            let (sex, head, shield, head_top, head_mid, head_bottom, hair_color, cloth_color) = (
                entity.sex,
                entity.head,
                entity.shield,
                entity.head_top,
                entity.head_mid,
                entity.head_bottom,
                entity.hair_color,
                entity.cloth_color,
            );
            let entity_type = entity.entity_type;
            let is_player = self.game.entities.player_id() == Some(gid);
            if is_player {
                self.load_player_sprite(
                    gid,
                    sprite_job,
                    sex,
                    head,
                    hair_color,
                    cloth_color,
                    weapon_type,
                    head_top,
                    head_mid,
                    head_bottom,
                    shield,
                );
            } else {
                self.load_entity_sprite(
                    gid,
                    entity_type,
                    sprite_job,
                    sex,
                    head,
                    0,
                    shield,
                    head_top,
                    head_mid,
                    head_bottom,
                    hair_color,
                    0,
                );
            }
        }
    }

    pub(super) fn handle_play_effect_on_entity(
        &mut self,
        gid: u32,
        effect_id: i32,
        value: Option<i32>,
    ) {
        let Ok(id) = EffectId::try_from_value(effect_id as usize) else {
            return;
        };
        match value {
            Some(v) if v > 0 => self
                .effect_queue
                .spawn_on_with_count(id, gid, v.min(255) as u8),
            _ => self.effect_queue.spawn_on(id, gid),
        }
    }

    pub(super) fn handle_play_misc_effect_on_entity(&mut self, gid: u32, code: u8) {
        let id = match code {
            0 => EffectId::Angel,
            1 => EffectId::Joblvup,
            2 => EffectId::Refinefail,
            3 => EffectId::Refineok,
            5 => EffectId::PharmacyOk,
            6 => EffectId::PharmacyFail,
            7 => EffectId::Angel2,
            8 => EffectId::Joblvup50,
            9 => EffectId::Angel3,
            _ => return,
        };
        self.effect_queue.spawn_on(id, gid);
    }

    pub(crate) fn entity_world_pos(&self, gid: u32) -> Option<[f32; 3]> {
        let (gat, coords) = (self.game.gat.as_ref()?, self.game.map_coords.as_ref()?);
        let (cx, cy) = self.game.entities.get(gid)?.movement.position();
        let (wx, _, wz) = coords.cell_to_world(cx + 0.5, cy + 0.5);
        Some([wx, gat.get_height(cx + 0.5, cy + 0.5), wz])
    }

    pub(super) fn handle_entity_resurrected(&mut self, gid: u32) {
        if let Some(entity) = self.game.entities.get_mut(gid) {
            entity.revive();
        }
    }

    pub(super) fn handle_mvp_reward(&mut self, gid: u32) {
        self.effect_queue.spawn_on(EffectId::Mvp, gid);
    }

    pub(super) fn handle_skill_unit_entered(
        &mut self,
        aid: u32,
        x: i16,
        y: i16,
        unit_id: u8,
        is_visible: bool,
    ) {
        let (Some(gat), Some(coords)) = (self.game.gat.as_ref(), self.game.map_coords.as_ref())
        else {
            return;
        };
        let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
        let (wx, _, wz) = coords.cell_to_world(cx, cy);
        let world = [wx, gat.get_height(cx, cy), wz];

        // Deployed traps render as a 3D model built in the update loop, and store
        // a trigger burst that fires when a monster springs the trap (a
        // `UNT_USED_TRAPS` look change) — not at placement. A trap hidden from us
        // (cast by others) is held aside until a skill-unit update reveals it.
        if trap_model_name(unit_id).is_some() {
            if is_visible {
                self.game.hidden_traps.remove(&aid);
                self.game.trap_units.insert(aid, (unit_id, world));
            } else {
                self.game.hidden_traps.insert(aid, (unit_id, world));
            }
            return;
        }

        if !is_visible {
            return;
        }
        let Some(effect) = skill_unit_effect(unit_id) else {
            return;
        };
        if self.effect_holder.reposition_by_key(aid, world) {
            return;
        }
        self.effect_queue.spawn_at_keyed(effect, world, aid);
    }

    pub(super) fn handle_skill_unit_disappeared(&mut self, aid: u32) {
        self.effect_queue.despawn(aid);
        self.game.trap_units.remove(&aid);
        self.game.hidden_traps.remove(&aid);
    }

    /// A skill-unit update reveals a trap that was hidden from us (e.g. an ankle
    /// snare springs on a monster): promote it so its ground model is built.
    pub(super) fn handle_skill_unit_updated(&mut self, gid: u32) {
        if let Some(trap) = self.game.hidden_traps.remove(&gid) {
            self.game.trap_units.insert(gid, trap);
        }
    }
}
