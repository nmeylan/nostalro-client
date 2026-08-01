use super::lifecycle::SessionChange;
use crate::App;
use crate::game_state::TrapUnit;
use models::enums::EnumWithNumberValue;
use models::enums::action::ActionType;
use models::enums::class::JobName;
use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;
use models::enums::vanish::VanishType;
use models::enums::weapon::WeaponType;
use ragnarok_formats::act::SpriteActionType;
use ragnarok_game::ailment;
use ragnarok_game::arrow::{ArrowProjectile, arrow_shower_cells, flight_secs_for_cell_distance};
use ragnarok_game::boss_info::{BossInfoKind, BossMark, boss_info_line};
use ragnarok_game::damage_number::{DamageNumber, DamageNumberType};
use ragnarok_game::effect::{
    OPT3_BLADESTOP, StatusKind, StatusSound, UNT_USED_TRAPS, devil_blind_effect,
    monster_opt3_reaction, opt3_bit_for_icon, opt3_bits, persistent_aura, player_opt3_reaction,
    skill_unit_effect, skill_unit_entry_sound, status_reaction, status_reaction_by_efst,
    trap_model_name, trap_trigger_effect,
};
use ragnarok_game::entity::{ChatBubbleState, Entity, EntityState, EntityType};
use ragnarok_game::entity_collection::GROUND_SKILL_EXEC_SECS;
use ragnarok_game::graffiti::Graffiti;
use ragnarok_game::level_aura;
use ragnarok_game::movement::direction_from_positions;
use ragnarok_game::scheduled_hit::Swing;
use ragnarok_game::sound::tables::{
    StatusSoundKind, job_hit_sound, skill_hit_sound, status_sound, weapon_hit_sound,
};
use ragnarok_game::sprite_path::{
    JT_WARPNPC, OPTION_HIDE, OPTION_RUWACH, OPTION_SIGHT, OPTION_STATUS_ICONS,
    cart_design_from_option, entity_type_from_job, has_falcon, is_hidden, visual_job,
};
use ragnarok_game::status_icon::status_icon_info;
use ragnarok_renderer::SfxPos;

/// A monster spawned with this value in the `head`/hair field is the player's pet.
const PET_HEAD_MARKER: u16 = 100;

/// The doubled mote column shows even with `/aura` off; the ring and the floor
/// glow are what the toggle takes away.
const LEVEL_AURA_MOTES: &[EffectId] = &[EffectId::Level993, EffectId::Level993];
const LEVEL_AURA_LAYERS: &[EffectId] = &[
    EffectId::Level993,
    EffectId::Level993,
    EffectId::Level99,
    EffectId::Level992,
];
const BOSS_AURA_LAYERS: &[EffectId] = &[EffectId::Green995, EffectId::Green996, EffectId::Green993];

fn is_weather_effect(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Snow
            | EffectId::Sakura
            | EffectId::Maple
            | EffectId::Cloud
            | EffectId::Cloud2
            | EffectId::Cloud3
            | EffectId::Cloud4
            | EffectId::Cloud5
            | EffectId::Cloud6
            | EffectId::Cloud7
            | EffectId::Cloud8
            | EffectId::Pokjuk
            | EffectId::PokjukSound
    )
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_entity_spawned(
        &mut self,
        gid: u32,
        aid: u32,
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
        guild_id: u32,
        guild_emblem_version: i32,
        is_new_entry: bool,
    ) {
        if self.game.world.entities.player_id() == Some(gid) {
            if effect_state != 0 {
                self.handle_entity_option_changed(gid, body_state, health_state, effect_state);
            }
            return;
        }
        let stale = self
            .game
            .world
            .entities
            .get(gid)
            .is_some_and(|e| e.state == EntityState::Dead || e.is_fading());
        if stale {
            self.despawn_entity_effects(gid);
            self.game.world.entities.remove(gid);
            self.game.sprite_caches.sprites.remove(&gid);
            self.remove_gr2_model(gid);
        } else if let Some(existing) = self.game.world.entities.get_mut(gid) {
            existing.movement.set_speed(speed);
            // A re-declared entity is back in sight: cancel any out-of-sight fade
            // so it is not removed mid-frame, which would strip its keyed effects
            // (e.g. a warp portal) that the server will not re-send.
            existing.fade = None;
            // A fresh spawn for an already-visible entity re-declares its cell: on
            // a same-map teleport the master's companion is re-sent here rather
            // than vanished, so honour the new position instead of stranding it.
            let (cx, cy) = existing.movement.cell_position();
            if cx != x || cy != y {
                existing.movement.set_position(x as f32, y as f32);
                existing.state = EntityState::Standing;
                existing.state_timer = 0.0;
            }
            let effect_changed = existing.effect_state != effect_state;
            if effect_changed {
                self.handle_entity_option_changed(gid, body_state, health_state, effect_state);
            }
            self.refresh_level_aura(gid);
            self.refresh_boss_aura(gid);
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
        entity.is_pet = entity_type == EntityType::Monster && head == PET_HEAD_MARKER;
        if entity.is_pet {
            entity.pet_accessory = head_bottom;
        }
        entity.guild_id = guild_id;
        entity.guild_emblem_version = guild_emblem_version;
        entity.is_gm = entity_type == EntityType::Player && self.config.is_gm_account(aid);
        match posture {
            1 => entity.state = EntityState::Dead,
            2 => entity.state = EntityState::Sitting,
            _ => {}
        }
        self.game.world.entities.insert(entity);
        self.game.world.entities.register_account_id(aid, gid);
        self.request_entity_guild_emblem(guild_id, guild_emblem_version);
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
        let pet_accessory = self
            .game
            .world
            .entities
            .get(gid)
            .filter(|e| e.is_pet && e.pet_accessory != 0)
            .map(|e| e.pet_accessory);
        if let Some(accessory) = pet_accessory {
            self.load_pet_sprite(gid, sprite_job, accessory);
        }
        if is_new_entry && entity_type == EntityType::Player && !is_hidden(effect_state) {
            self.effect_queue.spawn_on(EffectId::Entry2, gid);
        }
        if let Some(design) = cart_design_from_option(effect_state) {
            if let Some(entity) = self.game.world.entities.get_mut(gid) {
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
            .session
            .server_time
            .observe_server_tick(start_time, local_ms);
        let already_moving_to_dest = self
            .game
            .world
            .entities
            .get(gid)
            .filter(|e| e.movement.is_moving())
            .and_then(|e| e.movement.destination())
            .is_some_and(|(dx, dy)| dx == dest_x && dy == dest_y);
        if already_moving_to_dest {
            return;
        }
        if let Some(gat) = &self.game.session.gat {
            let (sx, sy) = self
                .game
                .world
                .entities
                .get(gid)
                .map(|e| e.movement.cell_position())
                .unwrap_or((start_x, start_y));
            let path = ragnarok_game::path::path_search(gat, sx, sy, dest_x, dest_y);
            if !path.is_empty() {
                let now = local_ms as f32 / 1000.0;
                if let Some(entity) = self.game.world.entities.get_mut(gid) {
                    entity.begin_move(path, now);
                }
            }
        }
    }

    pub(super) fn handle_entity_vanished(&mut self, gid: u32, vanish_type: VanishType) {
        if self.game.combat.attack_target_id == Some(gid) {
            self.game.combat.attack_target_id = None;
        }
        self.clear_cast_mark(gid);
        if self.game.companions.pet.gid == Some(gid) {
            self.game.companions.pet.clear_entity();
        }
        if let Some(h) = self
            .game
            .companions
            .homunculus
            .as_mut()
            .filter(|h| h.gid == gid)
        {
            if matches!(vanish_type, VanishType::Die) {
                h.hp = 0;
            }
        }
        if let Some(m) = self
            .game
            .companions
            .mercenary
            .as_mut()
            .filter(|m| m.gid == gid)
        {
            if matches!(vanish_type, VanishType::Die) {
                m.hp = 0;
            }
        }
        match vanish_type {
            VanishType::Die => {
                if let Some(entity) = self.game.world.entities.get_mut(gid) {
                    entity.request_pending_death();
                }
                self.despawn_level_aura(gid);
                self.despawn_boss_aura(gid);
                self.despawn_pk_rank_aura(gid);
                if self.game.world.entities.player_id() == Some(gid) {
                    if let Some(pos) = self.entity_world_pos(gid) {
                        self.effect_queue.spawn_at(EffectId::Devil, pos);
                    }
                    self.on_session_change(SessionChange::Death);
                }
            }
            VanishType::OutOfSight => {
                if let Some(entity) = self.game.world.entities.get_mut(gid) {
                    entity.start_vanish_fade();
                }
            }
            _ => {
                let poof = match vanish_type {
                    VanishType::Teleport => {
                        let hidden = self
                            .game
                            .world
                            .entities
                            .get(gid)
                            .is_some_and(|e| is_hidden(e.effect_state));
                        (!hidden).then_some(EffectId::Teleportation2)
                    }
                    VanishType::Loggout => Some(EffectId::Teleportation2),
                    _ => None,
                };
                if let Some(effect) = poof
                    && let Some(pos) = self.entity_world_pos(gid)
                {
                    self.effect_queue.spawn_at(effect, pos);
                }
                self.despawn_entity_effects(gid);
                let r1 = self.game.world.entities.remove(gid).is_some();
                let r2 = self.game.sprite_caches.sprites.remove(&gid).is_some();
                self.remove_gr2_model(gid);
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
            .session
            .server_time
            .observe_server_tick(start_time, local_ms);
        let action_start = self
            .game
            .session
            .server_time
            .server_to_local_secs_clamped(start_time, local_ms);
        let local_now = local_ms as f32 / 1000.0;
        let age = (local_now - action_start).max(0.0);
        match action {
            ActionType::Sit => {
                if let Some(entity) = self.game.world.entities.get_mut(gid) {
                    entity.state = EntityState::Sitting;
                    entity.state_timer = 0.0;
                }
            }
            ActionType::Stand => {
                if let Some(entity) = self.game.world.entities.get_mut(gid) {
                    entity.state = EntityState::Standing;
                    entity.state_timer = 0.0;
                }
                if self.game.world.entities.is_player(gid) {
                    self.game.session.doridori.reset();
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
                    .world
                    .entities
                    .get(target_gid)
                    .map(|e| e.movement.cell_position());
                let mut shooter_cell = None;
                let motion_factor = ragnarok_game::entity::attack_motion_factor(attack_mt);
                let swing_secs = self
                    .game
                    .world
                    .entities
                    .get(gid)
                    .zip(self.game.sprite_caches.sprites.get(&gid))
                    .and_then(|(entity, sprite)| {
                        let group = entity.resolved_attack_action_index(&sprite.body_act);
                        sprite.body_act.action_group_duration_ms(group)
                    })
                    .map(|ms| ms * motion_factor / 1000.0);
                if let Some(entity) = self.game.world.entities.get_mut(gid) {
                    if let Some(tp) = target_pos {
                        let sp = entity.movement.cell_position();
                        if let Some(dir) = direction_from_positions(sp.0, sp.1, tp.0, tp.1) {
                            entity.direction = dir;
                        }
                    }
                    let duration =
                        swing_secs.unwrap_or_else(|| ((attack_mt as f32 / 1000.0) - age).max(0.5));
                    entity.enter_attack(duration, motion_factor);
                    entity.target_gid = Some(target_gid);
                    if entity.weapon == Some(WeaponType::Bow) {
                        shooter_cell = Some(entity.movement.cell_position());
                    }
                }
                if Some(gid) == self.game.world.entities.player_id() {
                    self.send_queued_move();
                }
                let is_endure = matches!(
                    action,
                    ActionType::AttackNomotion | ActionType::AttackMultipleNomotion
                );
                let effective_count = match action {
                    ActionType::AttackMultiple
                    | ActionType::AttackMultipleNomotion
                    | ActionType::AttackMultipleCritical => count.max(1) as u16,
                    _ => 1,
                };
                if let (Some(sc), Some(tp)) = (shooter_cell, target_pos) {
                    self.spawn_arrow_projectile(sc, tp, attack_mt, effective_count);
                }
                // Only a player lands a distinct off-hand blow; a monster's
                // second damage field is part of the same swing.
                let left_damage = if self
                    .game
                    .world
                    .entities
                    .get(gid)
                    .is_some_and(|e| e.entity_type == EntityType::Player)
                {
                    left_damage
                } else {
                    0
                };
                let swing = Swing {
                    damage,
                    left_damage,
                    count: effective_count,
                    is_endure,
                    is_critical: matches!(
                        action,
                        ActionType::AttackCritical | ActionType::AttackMultipleCritical
                    ),
                    attacker_gid: gid,
                    attacked_mt_secs: attacked_mt as f32 / 1000.0,
                    fire_at: action_start + (attack_mt as f32 / 1000.0).max(0.0),
                };
                if let Some(target) = self.game.world.entities.get_mut(target_gid) {
                    for hit in swing.schedule() {
                        target.scheduled_hits.push(hit);
                    }
                }
            }
            ActionType::AttackLucky => {
                let dir = self
                    .game
                    .world
                    .entities
                    .get(target_gid)
                    .map(|e| e.direction)
                    .unwrap_or(0);
                self.game.combat.damage_numbers.add(DamageNumber::new(
                    target_gid,
                    0,
                    DamageNumberType::Lucky,
                    dir,
                ));
            }
            ActionType::Itempickup => {
                let motion_secs = self
                    .game
                    .world
                    .entities
                    .get(gid)
                    .zip(self.game.sprite_caches.sprites.get(&gid))
                    .filter(|(entity, _)| {
                        matches!(
                            entity.entity_type,
                            EntityType::Player | EntityType::Mercenary
                        )
                    })
                    .and_then(|(_, sprite)| {
                        sprite
                            .body_act
                            .action_group_duration_ms(SpriteActionType::Pickup as usize)
                    })
                    .map(|ms| ms / 1000.0);
                if let Some(entity) = self.game.world.entities.get_mut(gid) {
                    entity.enter_pickup(motion_secs);
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
        let (Some(gat), Some(coords)) = (&self.game.session.gat, &self.game.session.map_coords)
        else {
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
                .world
                .arrows
                .push(ArrowProjectile::new(from, to, delay, flight));
        }
    }

    /// Arrow Shower rains an arrow on the aimed cell and on each of its eight
    /// neighbours, so several land on bare ground.
    pub(super) fn spawn_arrow_shower(&mut self, src_gid: u32, x: i16, y: i16) {
        if x < 0 || y < 0 {
            return;
        }
        let Some(caster) = self.game.world.entities.get(src_gid) else {
            return;
        };
        let shooter = caster.movement.cell_position();
        let base_action = caster.action_index();
        let direction = caster.direction;
        let (Some(gat), Some(coords)) = (&self.game.session.gat, &self.game.session.map_coords)
        else {
            return;
        };
        let cell_world = |x: f32, y: f32| {
            let (wx, _, wz) = coords.cell_to_world(x + 0.5, y + 0.5);
            [wx, gat.get_height(x + 0.5, y + 0.5) - 10.0, wz]
        };
        let from = cell_world(shooter.0 as f32, shooter.1 as f32);
        let legs = arrow_shower_cells((x as u16, y as u16)).map(|(cx, cy)| {
            let (dx, dy) = (cx as f32 - shooter.0 as f32, cy as f32 - shooter.1 as f32);
            let flight = flight_secs_for_cell_distance((dx * dx + dy * dy).sqrt());
            (cell_world(cx as f32, cy as f32), flight)
        });

        let land_at =
            GROUND_SKILL_EXEC_SECS * self.atk_keyframe_fraction(src_gid, base_action, direction);
        for (to, flight) in legs {
            let delay = (land_at - flight).max(0.0);
            self.game
                .world
                .arrows
                .push(ArrowProjectile::new(from, to, delay, flight));
        }
    }

    pub(super) fn handle_attack_failed_for_distance(
        &mut self,
        target_aid: u32,
        target_x: u16,
        target_y: u16,
        x: u16,
        y: u16,
        range: i16,
    ) {
        self.game.combat.attack_range = range;

        let target_id = self.game.world.entities.resolve_key(target_aid);
        if self.game.combat.attack_target_id != Some(target_id) {
            return;
        }
        let target_alive = self
            .game
            .world
            .entities
            .get(target_id)
            .is_some_and(|e| e.state != EntityState::Dead && !e.is_fading());
        if !target_alive {
            self.stop_attacking();
            return;
        }

        if let Some(target) = self.game.world.entities.get_mut(target_id) {
            target
                .movement
                .correct_to_cell(target_x as f32, target_y as f32);
        }
        if let Some(player) = self.game.world.entities.player_mut() {
            player.movement.correct_to_cell(x as f32, y as f32);
        }
        self.game.combat.attack_request_cooldown = 0.0;
        self.resume_attack(target_id);
    }

    pub(super) fn handle_entity_hp_changed(&mut self, gid: u32, hp: u32, max_hp: u32) {
        if self.game.world.entities.is_player(gid) {
            self.game.character.hp = hp;
            self.game.character.max_hp = max_hp;
        } else if let Some(entity) = self.game.world.entities.get_mut(gid) {
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
        let gid = self.game.world.entities.resolve_key(gid);
        tracing::debug!(
            "EntityOptionChanged: gid={gid} body=0x{body_state:04x} health=0x{health_state:04x} effect_state=0x{effect_state:08x}"
        );
        if self.game.world.entities.is_player(gid) {
            self.game.character.effect_state = effect_state;
        }
        let is_player = self.game.world.entities.player_id() == Some(gid);
        let prev_health = self
            .game
            .world
            .entities
            .get(gid)
            .map(|e| e.health_state)
            .unwrap_or(0);
        let prev_body = self
            .game
            .world
            .entities
            .get(gid)
            .map(|e| e.body_state)
            .unwrap_or(0);
        let old_effect_state = self
            .game
            .world
            .entities
            .get(gid)
            .map(|e| e.effect_state)
            .unwrap_or(0);
        if let Some(entity) = self.game.world.entities.get_mut(gid) {
            entity.body_state = body_state;
            entity.health_state = health_state;
        }
        {
            let was_frozen = prev_body == ailment::OPT1_FREEZE;
            let now_frozen = body_state == ailment::OPT1_FREEZE;
            if now_frozen && !was_frozen {
                self.queue_status_sound(gid, StatusSoundKind::FreezeEnter);
            } else if was_frozen && !now_frozen {
                self.game
                    .world
                    .freeze_shatters
                    .push(crate::game_state::FreezeShatter {
                        gid,
                        started_at: None,
                    });
                self.queue_status_sound(gid, StatusSoundKind::FreezeExit);
            }
            if prev_body == ailment::OPT1_STONE && body_state != ailment::OPT1_STONE {
                self.queue_status_sound(gid, StatusSoundKind::StoneCurseExit);
            }
            if body_state == ailment::OPT1_STUN && prev_body != ailment::OPT1_STUN {
                self.queue_status_sound(gid, StatusSoundKind::StunEnter);
            }
            let gained = |bit: i16| health_state & bit != 0 && prev_health & bit == 0;
            if gained(ailment::OPT2_POISON)
                || gained(ailment::OPT2_DEADLYPOISON)
                || gained(ailment::OPT2_BLEEDING)
            {
                self.queue_status_sound(gid, StatusSoundKind::PoisonSet);
            }
            if gained(ailment::OPT2_CURSE) {
                self.queue_status_sound(gid, StatusSoundKind::CurseSet);
            }
            if gained(ailment::OPT2_SILENCE) {
                self.queue_status_sound(gid, StatusSoundKind::SilenceSet);
            }
            if gained(ailment::OPT2_CONFUSION) {
                self.queue_status_sound(gid, StatusSoundKind::ConfusionSet);
            }
            if gained(ailment::OPT2_BLIND) {
                self.queue_status_sound(gid, StatusSoundKind::BlindSet);
            }
        }
        if is_player
            && let Some(player) = self.game.world.entities.player_mut()
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
        if let Some(entity) = self.game.world.entities.get_mut(gid) {
            let old_sprite_job = visual_job(entity.job, entity.effect_state);
            let new_sprite_job = visual_job(entity.job, effect_state);
            old_cart = cart_design_from_option(entity.effect_state);
            new_cart = cart_design_from_option(effect_state);
            old_falcon = has_falcon(entity.effect_state);
            new_falcon = has_falcon(effect_state);
            entity.cart_type = new_cart;
            tracing::debug!(
                "  old_effect=0x{:08x} new_effect=0x{effect_state:08x} job={}",
                entity.effect_state,
                entity.job
            );
            let orcish_changed = ragnarok_game::sprite_path::is_orcish(entity.effect_state)
                != ragnarok_game::sprite_path::is_orcish(effect_state);
            entity.effect_state = effect_state;
            if old_sprite_job != new_sprite_job || orcish_changed {
                let sprite_job = new_sprite_job;
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
                        let e = self.game.world.entities.get(gid).unwrap();
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
            if is_player {
                self.game.character.cart_design = new_cart;
            }
        }
        if old_falcon != new_falcon {
            if new_falcon {
                self.spawn_falcon_visual(gid);
            } else {
                self.despawn_falcon_visual(gid);
            }
        }
        if is_player {
            for &(bit, efst) in OPTION_STATUS_ICONS {
                let was = old_effect_state & bit != 0;
                let now = effect_state & bit != 0;
                if was != now {
                    self.track_player_status(efst, now, 0, 0);
                }
            }
            let gained_hide =
                old_effect_state & OPTION_HIDE == 0 && effect_state & OPTION_HIDE != 0;
            if gained_hide && self.player_hide_move_blocked() {
                if let Some(player) = self.game.world.entities.player_mut() {
                    player.movement.stop();
                }
            }
        }
        self.refresh_level_aura(gid);
        self.refresh_boss_aura(gid);
        self.refresh_detect_aura(gid);
        self.refresh_pk_rank_aura(gid);
    }

    pub(super) fn refresh_detect_aura(&mut self, gid: u32) {
        let Some(effect_state) = self.game.world.entities.get(gid).map(|e| e.effect_state) else {
            return;
        };
        let want_sight = effect_state & OPTION_SIGHT != 0;
        match (
            want_sight,
            self.game.effect_keys.sight_aura_keys.contains_key(&gid),
        ) {
            (true, false) => {
                let key = self.next_entity_effect_key();
                self.effect_queue.spawn_on_keyed(EffectId::Sight2, gid, key);
                self.game.effect_keys.sight_aura_keys.insert(gid, key);
                self.queue_status_sound(gid, StatusSoundKind::DetectOn);
            }
            (false, true) => {
                if let Some(key) = self.game.effect_keys.sight_aura_keys.remove(&gid) {
                    self.effect_queue.despawn(key);
                }
            }
            _ => {}
        }
        let want_ruwach = effect_state & OPTION_RUWACH != 0;
        match (
            want_ruwach,
            self.game.effect_keys.ruwach_aura_keys.contains_key(&gid),
        ) {
            (true, false) => {
                let key = self.next_entity_effect_key();
                self.effect_queue.spawn_on_keyed(EffectId::Ruwach, gid, key);
                self.game.effect_keys.ruwach_aura_keys.insert(gid, key);
                self.queue_status_sound(gid, StatusSoundKind::DetectOn);
            }
            (false, true) => {
                if let Some(key) = self.game.effect_keys.ruwach_aura_keys.remove(&gid) {
                    self.effect_queue.despawn(key);
                }
            }
            _ => {}
        }
    }

    /// opt3 arrives as a whole word, so reconcile every bit against the last one
    /// seen: newly set bits launch their aura, cleared bits despawn it.
    pub(super) fn handle_entity_opt3_changed(
        &mut self,
        gid: u32,
        effect_state: i32,
        base_level: i32,
        opt3: i32,
    ) {
        let gid = self.game.world.entities.resolve_key(gid);
        let Some((body_state, health_state, entity_type, previous)) = self
            .game
            .world
            .entities
            .get(gid)
            .map(|e| (e.body_state, e.health_state, e.entity_type, e.opt3))
        else {
            return;
        };
        if let Some(entity) = self.game.world.entities.get_mut(gid) {
            entity.opt3 = opt3;
            entity.base_level = base_level as i16;
        }
        self.handle_entity_option_changed(gid, body_state, health_state, effect_state);

        // A hidden actor shows none of these auras.
        let visible_opt3 = if is_hidden(effect_state) { 0 } else { opt3 };
        let is_player_actor = entity_type == EntityType::Player;
        let reaction_of = |bit| {
            if is_player_actor {
                player_opt3_reaction(bit)
            } else {
                monster_opt3_reaction(bit)
            }
        };

        let was = opt3_bits(previous);
        let now = opt3_bits(visible_opt3);
        for bit in was.iter().copied().filter(|b| !now.contains(b)) {
            let Some(reaction) = reaction_of(bit) else {
                continue;
            };
            if let Some(key) = self.game.effect_keys.opt3_keys.remove(&(gid, bit)) {
                self.effect_queue.despawn(key);
            }
            for &id in reaction.on_clear {
                self.effect_queue.spawn_on(id, gid);
            }
            if reaction.grip {
                self.release_grip(gid);
            }
        }
        for bit in now.iter().copied().filter(|b| !was.contains(b)) {
            let Some(reaction) = reaction_of(bit) else {
                continue;
            };
            if !reaction.aura.is_empty() {
                let key = self.next_entity_effect_key();
                for &id in reaction.aura {
                    self.effect_queue.spawn_on_keyed(id, gid, key);
                }
                self.game.effect_keys.opt3_keys.insert((gid, bit), key);
            }
            if reaction.grip {
                self.apply_grip(gid);
            }
        }
    }

    fn apply_grip(&mut self, gid: u32) {
        if let Some(entity) = self.game.world.entities.get_mut(gid) {
            entity.rooted = true;
            entity.movement.stop();
        }
    }

    fn release_grip(&mut self, gid: u32) {
        if let Some(entity) = self.game.world.entities.get_mut(gid) {
            entity.rooted = false;
            entity.forced_animation = None;
        }
    }

    /// The status icons and the opt3 word describe the same body buffs on two
    /// channels. Only the icon carries a duration, so it owns the aura for its bit
    /// and the spawn-time one steps aside.
    fn sync_opt3_bit_with_status(&mut self, gid: u32, icon: ClientEffectIcon, active: bool) {
        let Some(bit) = opt3_bit_for_icon(icon) else {
            return;
        };
        if let Some(entity) = self.game.world.entities.get_mut(gid) {
            if active {
                entity.opt3 |= bit;
            } else {
                entity.opt3 &= !bit;
            }
        }
        if let Some(key) = self.game.effect_keys.opt3_keys.remove(&(gid, bit)) {
            self.effect_queue.despawn(key);
        }
        if !active && bit == OPT3_BLADESTOP {
            self.release_grip(gid);
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
        let gid = self.game.world.entities.resolve_key(gid);
        let is_player = self.game.world.entities.player_id() == Some(gid);
        let raw_reaction = status_reaction_by_efst(efst);
        let icon = ClientEffectIcon::try_from_value(efst as usize).ok();

        if let Some(icon) = icon {
            if let Some(entity) = self.game.world.entities.get_mut(gid) {
                entity.react_to_status(icon, active);
            }
            if is_player {
                self.track_player_status(efst, active, val1, remain_ms as u64);
            }
            self.sync_opt3_bit_with_status(gid, icon, active);
        }

        let Some(reaction) = raw_reaction.or_else(|| icon.and_then(status_reaction)) else {
            return;
        };

        if reaction.night_filter && is_player {
            self.game.schedulers.day_night.set_night(active);
        }

        if reaction.screen_ripple && is_player {
            self.game.session.screen_ripple = active;
        }

        if reaction.kind == StatusKind::PushCart {
            self.handle_push_cart_status(gid, active, val1);
            return;
        }

        if reaction.kind == StatusKind::DevilBlind {
            if is_player {
                self.handle_devil_blind_status(gid, efst, active, reaction.on_activate_sound);
            }
            return;
        }

        let map_key = (gid, efst);
        if let Some(old_key) = self.game.effect_keys.status_buff_keys.remove(&map_key) {
            self.effect_queue.despawn(old_key);
        }
        if active && !reaction.aura.is_empty() {
            let key = self.next_entity_effect_key();
            for &id in reaction.aura {
                self.effect_queue
                    .spawn_on_keyed_for(id, gid, key, remain_ms);
            }
            self.game.effect_keys.status_buff_keys.insert(map_key, key);
        }

        if active
            && let Some(sound) = reaction.on_activate_sound
            && (is_player || !sound.local_only)
        {
            self.play_status_sound(gid, sound);
        }

        let bursts = if active {
            reaction.on_activate
        } else {
            reaction.on_deactivate
        };
        for &id in bursts {
            self.effect_queue.spawn_on(id, gid);
        }
    }

    fn play_status_sound(&mut self, gid: u32, sound: StatusSound) {
        match sound.pos {
            SfxPos::Ui(depth) => self.sound_queue.ui_at_depth(sound.wave, depth),
            SfxPos::WorldAtDepth(depth) => {
                if let Some(pos) = self.entity_world_pos(gid) {
                    self.sound_queue.world_at_depth(sound.wave, pos, depth);
                }
            }
            SfxPos::World => {
                if let Some(pos) = self.entity_world_pos(gid) {
                    self.sound_queue.world(sound.wave, pos);
                }
            }
        }
    }

    fn handle_devil_blind_status(
        &mut self,
        gid: u32,
        efst: i16,
        active: bool,
        sound: Option<StatusSound>,
    ) {
        let map_key = (gid, efst);
        if !active {
            if let Some(key) = self.game.effect_keys.status_buff_keys.remove(&map_key) {
                self.effect_queue.despawn(key);
            }
            return;
        }
        if self
            .game
            .effect_keys
            .status_buff_keys
            .contains_key(&map_key)
        {
            return;
        }
        let level = self
            .game
            .character
            .skills
            .get_skill(SkillEnum::SgDevil.id() as u16)
            .map(|skill| skill.level)
            .unwrap_or(0);
        let Some(overlay) = u8::try_from(level).ok().and_then(devil_blind_effect) else {
            return;
        };
        let key = self.next_entity_effect_key();
        self.effect_queue.spawn_on_keyed(overlay, gid, key);
        self.game.effect_keys.status_buff_keys.insert(map_key, key);
        self.effect_queue.spawn_on(EffectId::Blackdevil, gid);
        if let Some(sound) = sound {
            self.play_status_sound(gid, sound);
        }
    }

    /// A map change wipes the effect queue, and the server only re-sends the
    /// ailment bits — every still-running buff visual has to be rebuilt from the
    /// status list the client keeps for the local player.
    pub(super) fn refresh_player_status_buffs(&mut self) {
        let Some(gid) = self.game.world.entities.player_id() else {
            return;
        };
        let now_ms = self.start_time.elapsed().as_millis() as u64;
        let statuses: Vec<(i16, Option<u64>)> = self
            .game
            .character
            .active_statuses
            .iter()
            .map(|s| (s.efst, s.end_ms))
            .collect();
        for (efst, end_ms) in statuses {
            let Some(aura) = persistent_aura(efst) else {
                continue;
            };
            let remain_ms = end_ms
                .map(|end| end.saturating_sub(now_ms).min(u32::MAX as u64) as u32)
                .unwrap_or(0);
            let key = self.next_entity_effect_key();
            for &id in aura {
                self.effect_queue
                    .spawn_on_keyed_for(id, gid, key, remain_ms);
            }
            self.game
                .effect_keys
                .status_buff_keys
                .insert((gid, efst), key);
        }
    }

    /// Track a status on the local player and preload its bar icon. Statuses the
    /// icon table does not cover are still tracked: the list is what the buff
    /// visuals are rebuilt from after a map change, and the bar skips them.
    /// `life_ms` of 0 renders no countdown wedge.
    pub(super) fn track_player_status(&mut self, efst: i16, active: bool, val1: i32, life_ms: u64) {
        if !active {
            self.game.character.clear_status(efst);
            return;
        }
        let loaded = match status_icon_info(efst) {
            Some(info) => {
                let path = format!("data/texture/effect/{}", info.icon);
                match (self.renderer.as_mut(), self.grf.as_ref()) {
                    (Some(r), Some(g)) => r.preload_textures(&[path.as_str()], g),
                    _ => false,
                }
            }
            None => false,
        };
        let now_ms = self.start_time.elapsed().as_millis() as u64;
        self.game
            .character
            .apply_status(efst, val1, now_ms, life_ms, loaded);
    }

    fn handle_push_cart_status(&mut self, gid: u32, active: bool, val1: i32) {
        let is_player = self.game.world.entities.player_id() == Some(gid);
        if active {
            let design = val1.clamp(0, u8::MAX as i32) as u8;
            if let Some(entity) = self.game.world.entities.get_mut(gid) {
                entity.cart_type = Some(design);
            }
            self.spawn_cart_visual(gid, design);
            if is_player {
                self.game.character.cart_design = Some(design);
            }
        } else if is_player {
            self.handle_cart_off();
        } else {
            if let Some(entity) = self.game.world.entities.get_mut(gid) {
                entity.cart_type = None;
            }
            self.despawn_cart_visual(gid);
        }
    }

    pub(crate) fn refresh_level_aura(&mut self, gid: u32) {
        let Some((entity_type, effect_state, entity_level, alive)) = self
            .game
            .world
            .entities
            .get(gid)
            .map(|e| (e.entity_type, e.effect_state, e.base_level, e.is_alive()))
        else {
            return;
        };
        let base_level = if self.game.world.entities.player_id() == Some(gid) {
            self.game.character.base_level as i16
        } else {
            entity_level
        };
        let visible =
            alive && level_aura::level_aura_visible(entity_type, base_level, effect_state);
        let want = visible.then(|| {
            if self.config.display.show_level_aura {
                LEVEL_AURA_LAYERS
            } else {
                LEVEL_AURA_MOTES
            }
        });
        let have = self.game.effect_keys.level_aura_keys.get(&gid).copied();
        if have.map(|(_, layers)| layers) == want {
            return;
        }
        if have.is_some() {
            self.despawn_level_aura(gid);
        }
        if let Some(layers) = want {
            let key = self.next_entity_effect_key();
            for &id in layers {
                self.effect_queue.spawn_on_keyed(id, gid, key);
            }
            self.game
                .effect_keys
                .level_aura_keys
                .insert(gid, (key, layers));
        }
    }

    pub(crate) fn despawn_level_aura(&mut self, gid: u32) {
        if let Some((key, _)) = self.game.effect_keys.level_aura_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
    }

    pub(super) fn refresh_boss_aura(&mut self, gid: u32) {
        let Some((entity_type, is_boss, effect_state, level, alive)) =
            self.game.world.entities.get(gid).map(|e| {
                (
                    e.entity_type,
                    e.is_boss,
                    e.effect_state,
                    e.base_level,
                    e.is_alive(),
                )
            })
        else {
            return;
        };
        let want = alive
            && self.config.custom.boss_aura
            && self.config.display.show_level_aura
            && level_aura::boss_aura_visible(entity_type, is_boss, level, effect_state);
        let have = self.game.effect_keys.boss_aura_keys.contains_key(&gid);
        match (want, have) {
            (true, false) => {
                let key = self.next_entity_effect_key();
                for &id in BOSS_AURA_LAYERS {
                    self.effect_queue.spawn_on_keyed(id, gid, key);
                }
                self.game.effect_keys.boss_aura_keys.insert(gid, key);
            }
            (false, true) => self.despawn_boss_aura(gid),
            _ => {}
        }
    }

    pub(crate) fn despawn_boss_aura(&mut self, gid: u32) {
        if let Some(key) = self.game.effect_keys.boss_aura_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
    }

    /// Re-establish the portal swirl for a warp NPC whose effect was dropped
    /// while the entity itself survived — e.g. a same-map move that resets the
    /// effect holder but keeps the entity, where the server only re-declares it
    /// via the already-visible path that never re-runs the spawn gate.
    pub(super) fn refresh_warp_portal(&mut self, gid: u32) {
        let is_warp = self
            .game
            .world
            .entities
            .get(gid)
            .is_some_and(|e| e.entity_type == EntityType::Npc && e.job == JT_WARPNPC);
        if is_warp {
            self.spawn_warp_portal(gid);
        }
    }

    pub(super) fn spawn_warp_portal(&mut self, gid: u32) {
        if self.game.effect_keys.warp_portal_keys.contains_key(&gid) {
            return;
        }
        let key = self.next_entity_effect_key();
        self.effect_queue
            .spawn_on_keyed(EffectId::Warpzone2, gid, key);
        self.game.effect_keys.warp_portal_keys.insert(gid, key);
    }

    pub(crate) fn despawn_warp_portal(&mut self, gid: u32) {
        if let Some(key) = self.game.effect_keys.warp_portal_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
    }

    /// Spirit spheres (Call Spirits / Explosion Spirits etc.): `count` orbiting
    /// balls, replaced whenever the server re-sends the count and cleared at 0.
    /// Champions and Gunslingers get their own sphere variant.
    pub(super) fn handle_spirits_changed(&mut self, gid: u32, count: u8) {
        if let Some(old_key) = self.game.effect_keys.spirit_keys.remove(&gid) {
            self.effect_queue.despawn(old_key);
        }
        if count == 0 {
            return;
        }
        let Some(job) = self
            .game
            .world
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
        self.game.effect_keys.spirit_keys.insert(gid, key);
    }

    /// PvP ranking broadcast: the server sends it to the whole area, so it
    /// carries the rank of every player around us, not only our own.
    pub(super) fn handle_pvp_ranking_changed(&mut self, account_id: u32, ranking: i32, total: i32) {
        if !self.game.session.map_properties.is_pk_zone() {
            return;
        }
        let gid = self.game.world.entities.resolve_key(account_id);
        let is_player = self.game.world.entities.player_id() == Some(gid);
        let Some(entity) = self.game.world.entities.get_mut(gid) else {
            return;
        };
        if !is_player && is_hidden(entity.effect_state) {
            return;
        }
        let old_rank = entity.pk_rank;
        entity.pk_rank = ranking;
        entity.pk_total = total;
        if is_player && old_rank != ranking {
            self.announce_own_pvp_rank(old_rank, ranking);
        }
        self.refresh_pk_rank_aura(gid);
    }

    fn announce_own_pvp_rank(&mut self, old_rank: i32, ranking: i32) {
        let top = level_aura::TOP_RANK_THRESHOLD;
        let name = self.game.character.name.clone();
        if old_rank > top && ranking <= top {
            self.windows.chat_window.add_system(format!(
                "Congratulations, {name} moved up to rank {ranking}."
            ));
        } else if old_rank <= top && ranking > top {
            self.windows
                .chat_window
                .add_system(format!("Too bad, {name} dropped to rank {ranking}."));
        }
        if let Some(pos) = self
            .game
            .world
            .entities
            .player_id()
            .and_then(|gid| self.entity_world_pos(gid))
        {
            self.sound_queue
                .world("effect\\number_change.wav".to_string(), pos);
        }
    }

    pub(crate) fn refresh_pk_rank_aura(&mut self, gid: u32) {
        let Some((entity_type, effect_state, rank, alive)) = self
            .game
            .world
            .entities
            .get(gid)
            .map(|e| (e.entity_type, e.effect_state, e.pk_rank, e.is_alive()))
        else {
            return;
        };
        let want = alive
            && self.game.session.map_properties.is_pk_zone()
            && level_aura::pk_rank_aura_visible(entity_type, rank, effect_state);
        // The tint is baked in at spawn, so a rank change has to respawn.
        let have = self
            .game
            .effect_keys
            .toprank_keys
            .get(&gid)
            .is_some_and(|&(_, spawned_rank)| spawned_rank == rank);
        match (want, have) {
            (true, false) => {
                self.despawn_pk_rank_aura(gid);
                let key = self.next_entity_effect_key();
                self.effect_queue.spawn_on_keyed_with_count(
                    EffectId::Toprank,
                    gid,
                    key,
                    rank as u8,
                );
                self.game.effect_keys.toprank_keys.insert(gid, (key, rank));
            }
            (false, _) => self.despawn_pk_rank_aura(gid),
            _ => {}
        }
    }

    pub(crate) fn despawn_pk_rank_aura(&mut self, gid: u32) {
        if let Some((key, _)) = self.game.effect_keys.toprank_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
    }

    /// The server drops the pvp mapflag by sending the new map property first
    /// and the cleared ranks after, so the rank packets that follow are already
    /// gated out by the PK-zone check.
    pub(crate) fn clear_pvp_ranks(&mut self) {
        for (_, (key, _)) in self.game.effect_keys.toprank_keys.drain() {
            self.effect_queue.despawn(key);
        }
        for entity in self.game.world.entities.iter_mut() {
            entity.pk_rank = 0;
            entity.pk_total = 0;
        }
    }

    pub(crate) fn despawn_entity_effects(&mut self, gid: u32) {
        self.despawn_level_aura(gid);
        self.despawn_boss_aura(gid);
        self.despawn_pk_rank_aura(gid);
        self.despawn_warp_portal(gid);
        if let Some(key) = self.game.effect_keys.spirit_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
        if let Some(key) = self.game.effect_keys.sight_aura_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
        if let Some(key) = self.game.effect_keys.ruwach_aura_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
        // The server never re-sends a buff for an actor that re-enters sight, so a
        // surviving key would leave the aura unreachable and double it on respawn.
        let stale: Vec<(u32, i16)> = self
            .game
            .effect_keys
            .status_buff_keys
            .keys()
            .copied()
            .filter(|(id, _)| *id == gid)
            .collect();
        for map_key in stale {
            if let Some(key) = self.game.effect_keys.status_buff_keys.remove(&map_key) {
                self.effect_queue.despawn(key);
            }
        }
        let stale: Vec<(u32, i32)> = self
            .game
            .effect_keys
            .opt3_keys
            .keys()
            .copied()
            .filter(|(id, _)| *id == gid)
            .collect();
        for map_key in stale {
            if let Some(key) = self.game.effect_keys.opt3_keys.remove(&map_key) {
                self.effect_queue.despawn(key);
            }
        }
    }

    fn next_entity_effect_key(&mut self) -> u32 {
        let key = 0x8000_0000 | self.game.effect_keys.next_status_buff_key;
        self.game.effect_keys.next_status_buff_key =
            (self.game.effect_keys.next_status_buff_key + 1) & 0x7fff_ffff;
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
            && let Some(trap) = self.game.world.trap_units.remove(&gid)
        {
            if let Some(burst) = trap_trigger_effect(trap.unit_id) {
                self.effect_queue.spawn_at(burst, trap.world);
            }
            return;
        }

        let left_hand_is_weapon = self.game.world.entities.is_player(gid)
            && self
                .game
                .character
                .inventory
                .equipped_in_slot(models::enums::item::EquipmentLocation::HandLeft)
                .is_some_and(|item| item.is_weapon());
        let mut job_change: Option<u16> = None;
        if let Some(entity) = self.game.world.entities.get_mut(gid) {
            if sprite_type == 0 && entity.job != value && entity.job != 0 {
                job_change = Some(value);
            }
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
            } else {
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
            let is_player = self.game.world.entities.player_id() == Some(gid);
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

        if let Some(new_job) = job_change
            && self.game.world.entities.is_player(gid)
        {
            // The skill tree is stale until the server resends the skill block.
            self.game.character.skills.close();
            let job_name = ragnarok_game::character::job_class_name(new_job);
            let text = format!("Your job has changed to {job_name}.");
            self.windows.chat_window.add_system(text.clone());
            self.game.broadcast.poptip.push(text.clone());
            self.game.broadcast.banner.enqueue(text, 1);
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
        if is_weather_effect(id) {
            self.spawn_weather_effect(id, gid);
            return;
        }
        match value {
            Some(v) if v > 0 => self
                .effect_queue
                .spawn_on_with_count(id, gid, v.min(255) as u8),
            _ => self.effect_queue.spawn_on(id, gid),
        }
    }

    /// Server-driven per-map weather (snow, sakura, maple, clouds, fireworks).
    /// The mapflag is resent on every load-end and on `/refresh`; a live instance
    /// suppresses the resend so weather never stacks. Duration `0` maps to
    /// `u32::MAX` so the effect lives until map change clears `weather_keys`.
    fn spawn_weather_effect(&mut self, id: EffectId, gid: u32) {
        if self.game.effect_keys.weather_keys.contains_key(&id) {
            return;
        }
        let key = self.next_entity_effect_key();
        self.effect_queue.spawn_on_keyed_for(id, gid, key, 0);
        self.game.effect_keys.weather_keys.insert(id, key);
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
        match code {
            0 | 7 | 9 => self.sound_queue.ui("levelup.wav"),
            _ => {}
        }
        self.effect_queue.spawn_on(id, gid);

        if self.game.world.entities.player_id() == Some(gid) {
            match code {
                0 | 7 | 9 => self.windows.levelup_notification.notify_base_level_up(),
                1 | 8 => self.windows.levelup_notification.notify_job_level_up(),
                _ => {}
            }
        }
    }

    pub(crate) fn entity_world_pos(&self, gid: u32) -> Option<[f32; 3]> {
        let (gat, coords) = (
            self.game.session.gat.as_ref()?,
            self.game.session.map_coords.as_ref()?,
        );
        let (cx, cy) = self.game.world.entities.get(gid)?.movement.position();
        let (wx, _, wz) = coords.cell_to_world(cx + 0.5, cy + 0.5);
        Some([wx, gat.get_height(cx + 0.5, cy + 0.5), wz])
    }

    /// Damage-taken hit sound at the victim's position. Skill damage uses the
    /// generic enemy-hit wave; melee uses the victim's body material (PC) or the
    /// attacker's weapon class (monster/NPC victim).
    pub(crate) fn queue_hit_sound(&mut self, victim_gid: u32, attacker_gid: u32, is_skill: bool) {
        let roll = self.next_sfx_rand();
        let wav = if is_skill {
            skill_hit_sound(roll)
        } else {
            let victim_humanoid = self.game.world.entities.get(victim_gid).is_some_and(|e| {
                matches!(e.entity_type, EntityType::Player | EntityType::Mercenary)
            });
            if victim_humanoid {
                self.game
                    .world
                    .entities
                    .get(victim_gid)
                    .and_then(|e| JobName::try_from_value(e.job as usize).ok())
                    .map(|j| job_hit_sound(j).to_string())
                    .unwrap_or_else(|| skill_hit_sound(roll))
            } else {
                let weapon = self
                    .game
                    .world
                    .entities
                    .get(attacker_gid)
                    .and_then(|e| e.weapon);
                let is_taekwon = self.game.world.entities.player_id() == Some(attacker_gid)
                    && self
                        .game
                        .world
                        .entities
                        .get(attacker_gid)
                        .and_then(|e| JobName::try_from_value(e.job as usize).ok())
                        == Some(JobName::Taekwon);
                weapon_hit_sound(weapon, roll, is_taekwon)
            }
        };
        if let Some(pos) = self.entity_world_pos(victim_gid) {
            self.sound_queue.world(wav, pos);
        }
    }

    fn queue_status_sound(&mut self, gid: u32, kind: StatusSoundKind) {
        if let Some(wav) = status_sound(kind)
            && let Some(pos) = self.entity_world_pos(gid)
        {
            self.sound_queue.world(wav.to_string(), pos);
        }
    }

    pub(super) fn handle_entity_resurrected(&mut self, gid: u32) {
        if let Some(entity) = self.game.world.entities.get_mut(gid) {
            entity.revive();
        }
        self.refresh_level_aura(gid);
        self.refresh_boss_aura(gid);
        self.refresh_pk_rank_aura(gid);
        if self.game.world.entities.player_id() == Some(gid) {
            self.on_session_change(SessionChange::Resurrect);
        }
    }

    pub(super) fn handle_mvp_reward(&mut self, gid: u32) {
        self.effect_queue.spawn_on(EffectId::Mvp, gid);
        match self.entity_world_pos(gid) {
            Some(pos) => self
                .sound_queue
                .world("effect\\st_mvp.wav".to_string(), pos),
            None => self.sound_queue.ui("effect\\st_mvp.wav"),
        }
    }

    pub(super) fn handle_skill_unit_entered(
        &mut self,
        aid: u32,
        creator_aid: u32,
        x: i16,
        y: i16,
        unit_id: u8,
        is_visible: bool,
    ) {
        let (Some(gat), Some(coords)) = (
            self.game.session.gat.as_ref(),
            self.game.session.map_coords.as_ref(),
        ) else {
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
            let trap = TrapUnit {
                unit_id,
                world,
                cell: (x, y),
            };
            if is_visible {
                self.game.world.hidden_traps.remove(&aid);
                self.game.world.trap_units.insert(aid, trap);
            } else {
                self.game.world.hidden_traps.insert(aid, trap);
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
            // eprintln!(
            //     "[song-unit] REPOSITION unit_id={unit_id:#x} aid={aid} creator={creator_aid} cell=({x},{y})"
            // );
            return;
        }
        // eprintln!(
        //     "[song-unit] SPAWN      unit_id={unit_id:#x} aid={aid} creator={creator_aid} cell=({x},{y})"
        // );
        if let Some(sfx) = skill_unit_entry_sound(unit_id)
            && (sfx.one_in <= 1 || self.next_sfx_rand() % sfx.one_in as u32 == 0)
        {
            self.sound_queue.world(sfx.wave.to_string(), world);
        }
        self.effect_queue.spawn_at_keyed(effect, world, aid);
    }

    pub(super) fn handle_skill_unit_disappeared(&mut self, aid: u32) {
        // eprintln!("[song-unit] DISAPPEAR aid={aid}");
        self.effect_queue.despawn(aid);
        self.game.world.trap_units.remove(&aid);
        self.game.world.hidden_traps.remove(&aid);
        self.game.world.graffiti.remove(&aid);
        self.game.world.talkbox_bubbles.remove(&aid);
    }

    /// Someone stepped on a Talkie Box: its message rides above the box itself,
    /// not in the chat log. Boxes cast by others were placed hidden to us, so
    /// they are still on the hidden side.
    pub(super) fn handle_talkbox_contents(&mut self, aid: u32, message: String) {
        if message.is_empty() {
            return;
        }
        let Some(&TrapUnit { world, .. }) = self
            .game
            .world
            .trap_units
            .get(&aid)
            .or_else(|| self.game.world.hidden_traps.get(&aid))
        else {
            tracing::debug!("Talkie Box {aid} has no known position");
            return;
        };
        self.game
            .world
            .talkbox_bubbles
            .insert(aid, (world, ChatBubbleState::new(message)));
    }

    pub(super) fn handle_boss_info(
        &mut self,
        kind: BossInfoKind,
        x: u16,
        y: u16,
        respawn_hour: u16,
        respawn_minute: u16,
        name: String,
    ) {
        match kind {
            BossInfoKind::Alive | BossInfoKind::AliveAnnounced => {
                self.game.boss_mark = Some(BossMark {
                    x,
                    y,
                    name: name.clone(),
                });
            }
            BossInfoKind::NotOnMap | BossInfoKind::Dead => self.game.boss_mark = None,
        }
        if let Some(line) = boss_info_line(kind, &name, respawn_hour, respawn_minute) {
            self.windows.chat_window.add_notice(line);
        }
    }

    pub(super) fn handle_graffiti_entered(
        &mut self,
        aid: u32,
        creator_aid: u32,
        x: i16,
        y: i16,
        message: String,
    ) {
        let yaw = self
            .game
            .world
            .entities
            .get(self.game.world.entities.resolve_key(creator_aid))
            .map(|e| e.direction as f32 * std::f32::consts::FRAC_PI_4)
            .unwrap_or(0.0);
        self.build_graffiti_texture(aid, &message);
        self.game.world.graffiti.insert(
            aid,
            Graffiti {
                creator_aid,
                cell_x: x.max(0) as u16,
                cell_y: y.max(0) as u16,
                yaw,
                message,
            },
        );
    }

    fn build_graffiti_texture(&mut self, aid: u32, message: &str) {
        let key = ragnarok_renderer::graffiti::texture_key(aid);
        let (Some(renderer), Some(grf)) = (self.renderer.as_mut(), self.grf.as_ref()) else {
            return;
        };
        match renderer.build_graffiti_texture(&key, message, grf) {
            true => {}
            false => tracing::warn!("Failed to compose graffiti texture for unit {aid}"),
        }
    }

    pub(super) fn handle_map_cell_changed(&mut self, x: i16, y: i16, cell_type: i32) {
        if let Some(gat) = &mut self.game.session.gat {
            gat.set_cell_type(x as i32, y as i32, cell_type);
        }
    }

    /// A skill-unit update reveals a trap that was hidden from us (e.g. an ankle
    /// snare springs on a monster): promote it so its ground model is built.
    pub(super) fn handle_skill_unit_updated(&mut self, gid: u32) {
        if let Some(trap) = self.game.world.hidden_traps.remove(&gid) {
            self.game.world.trap_units.insert(gid, trap);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_ids_dedup_but_fireworks_throw_stays_one_shot() {
        for id in [
            EffectId::Snow,
            EffectId::Sakura,
            EffectId::Maple,
            EffectId::Cloud4,
            EffectId::Cloud5,
            EffectId::Pokjuk,
            EffectId::PokjukSound,
        ] {
            assert!(is_weather_effect(id), "{id:?} should be keyed weather");
        }
        assert!(
            !is_weather_effect(EffectId::Throwitem2),
            "the fireworks item-toss is a normal one-shot, not deduped weather"
        );
    }
}
