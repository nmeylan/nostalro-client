use crate::App;
use models::enums::action::ActionType;
use models::enums::vanish::VanishType;
use models::enums::weapon::WeaponType;
use ragnarok_game::arrow::{flight_secs_for_cell_distance, ArrowProjectile};
use ragnarok_game::damage_number::{DamageNumber, DamageNumberType};
use ragnarok_game::entity::{Entity, EntityState};
use ragnarok_game::movement::direction_from_positions;
use ragnarok_game::scheduled_hit::{DamageMessage, ScheduledHit};
use ragnarok_game::sprite_path::{entity_type_from_job, visual_job, OPTION_RIDING};

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
        effect_state: i32,
    ) {
        if self.game.entities.player_id() == Some(gid) {
            if effect_state != 0 {
                self.handle_entity_option_changed(gid, effect_state);
            }
            return;
        }
        if let Some(existing) = self.game.entities.get_mut(gid) {
            existing.movement.set_speed(speed);
            if existing.effect_state != effect_state {
                self.handle_entity_option_changed(gid, effect_state);
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
        if body_state == 2 {
            entity.state = EntityState::Sitting;
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
        if let Some(gat) = &self.game.gat {
            let (sx, sy) = self
                .game
                .entities
                .get(gid)
                .map(|e| e.movement.cell_position())
                .unwrap_or((start_x, start_y));
            let path = ragnarok_game::path::path_search(gat, sx, sy, dest_x, dest_y);
            if !path.is_empty() {
                let local_ms = self.start_time.elapsed().as_millis() as u32;
                let is_just_spawned = self.game.entities.get(gid).is_some_and(|e| e.just_spawned);
                let move_start = if is_just_spawned {
                    self.game
                        .server_time
                        .server_to_local_secs(start_time, local_ms)
                } else {
                    local_ms as f32 / 1000.0
                };
                if let Some(entity) = self.game.entities.get_mut(gid) {
                    entity.movement.start_move(path, move_start);
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
    ) {
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
                    let duration = (attack_mt as f32 / 1000.0).max(0.5);
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
                let now = self.start_time.elapsed().as_secs_f32();
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

    /// Spawn flying arrow(s) from a bow/whip/instrument attacker's cell to the
    /// target cell. The arrow stays hidden until late in the attack motion then
    /// zips to the target (fixed ~192 ms, faster up close) so it lands as the
    /// damage applies. Multi-hit attacks (Double Attack, Arrow Vulcan) fire one
    /// arrow per hit, staggered by the same term used for scheduled hits. Cells
    /// are raised to chest height (negative Y = up).
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

    pub(super) fn handle_entity_option_changed(&mut self, gid: u32, effect_state: i32) {
        tracing::debug!("EntityOptionChanged: gid={gid} effect_state=0x{effect_state:08x}");
        if self.game.entities.is_player(gid) {
            self.game.character.effect_state = effect_state;
        }
        if let Some(entity) = self.game.entities.get_mut(gid) {
            let old_riding = (entity.effect_state & OPTION_RIDING) != 0;
            let new_riding = (effect_state & OPTION_RIDING) != 0;
            tracing::debug!("  old_effect=0x{:08x} old_riding={old_riding} new_riding={new_riding} job={}", entity.effect_state, entity.job);
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
                let is_player = self.game.entities.player_id() == Some(gid);
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
    }

    pub(super) fn handle_entity_sprite_changed(
        &mut self,
        gid: u32,
        sprite_type: u8,
        value: u16,
        value2: u16,
    ) {
        let left_hand_is_weapon = self.game.entities.is_player(gid)
            && self
                .game
                .character
                .inventory
                .equipped_in_slot(models::enums::item::EquipmentLocation::HandLeft)
                .is_some_and(|item| item.is_weapon());
        if let Some(entity) = self.game.entities.get_mut(gid) {
            if sprite_type == 2 {
                // LOOK_WEAPON: value=weapon, value2=left hand (weapon if dual-wield, shield otherwise)
                // Rathena converts both LOOK_WEAPON and LOOK_SHIELD to sprite_type=2 on the wire
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
}
