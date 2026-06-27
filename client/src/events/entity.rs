use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::action::ActionType;
use models::enums::effect_id::EffectId;
use models::enums::vanish::VanishType;
use models::enums::weapon::WeaponType;
use models::enums::client_effect_icon::ClientEffectIcon;
use ragnarok_game::ailment;
use ragnarok_game::level_aura;
use ragnarok_game::effect::buff_effect;
use ragnarok_game::arrow::{flight_secs_for_cell_distance, ArrowProjectile};
use ragnarok_game::damage_number::{DamageNumber, DamageNumberType};
use ragnarok_game::entity::{Entity, EntityState, EntityType};
use ragnarok_game::effect::skill_unit_effect;
use ragnarok_game::movement::direction_from_positions;
use ragnarok_game::status_icon::status_icon_info;
use ragnarok_game::scheduled_hit::{DamageMessage, ScheduledHit};
use ragnarok_game::sprite_path::{cart_design_from_option, entity_type_from_job, has_falcon, is_hidden, visual_job, JT_WARPNPC, OPTION_RIDING};

/// The level-99 aura is a composite the original game stacks together: the blue
/// spinning ring, the rising pikapika sparkles, and the orbiting light motes.
const LEVEL_AURA_LAYERS: &[EffectId] = &[EffectId::Level99, EffectId::Level992, EffectId::Level993];

/// The boss aura is the green reskin of the level-99 aura: the green ring and
/// green floor glow over the shared sparkle column.
const BOSS_AURA_LAYERS: &[EffectId] =
    &[EffectId::Green995, EffectId::Green996, EffectId::Level993];

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
        // A spawn for a gid still occupied by a corpse (death fade) or an
        // entity fading out of sight means the server reused the id for a fresh
        // entity. Drop the stale one so it is recreated below, otherwise the
        // respawn stays invisible until the map is reloaded.
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
        // Posture (sit/dead) is the entry packet's `state` byte, NOT `body_state`:
        // `body_state == 2` is OPT1_FREEZE, an ailment handled by the body-tint pass.
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
                self.game.server_time.observe_server_tick(start_time, local_ms);
                let move_start = self
                    .game
                    .server_time
                    .server_to_local_secs_clamped(start_time, local_ms);
                if let Some(entity) = self.game.entities.get_mut(gid) {
                    entity.movement.correct_to_cell(sx as f32, sy as f32);
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
                // Teleport-out / zone-exit leave a poof at the last position;
                // capture it before the entity is removed. Trick-dead just leaves.
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
        self.game.server_time.observe_server_tick(start_time, local_ms);
        // Server timeline anchor: when the action actually began, in local seconds. Events
        // arrive ~half-RTT late, so this is in the past; timings derive from it, not now.
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
        if let Some(entity) = self.game.entities.get_mut(gid) {
            entity.body_state = body_state;
            entity.health_state = health_state;
        }
        // Stop optimistic move prediction the instant an incapacitating ailment
        // lands — the server won't ack the move, so without this the local
        // player keeps walking client-side (STONEWAIT still allows movement).
        if is_player
            && ailment::movement_blocked(body_state)
            && let Some(player) = self.game.entities.player_mut()
        {
            player.movement.stop();
        }
        // Blind washes the screen, but only for the local player (others' blind
        // is invisible). Toggle the persistent fullscreen overlay on its edges,
        // keyed by the player gid so it can be despawned when blind clears.
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
        // Spawn/replace/despawn the trailing pushcart when its OPTION bit changes.
        if old_cart != new_cart {
            match new_cart {
                Some(design) => self.spawn_cart_visual(gid, design),
                None => self.despawn_cart_visual(gid),
            }
        }
        // Spawn/despawn the falcon companion when its OPTION bit changes.
        if old_falcon != new_falcon {
            if new_falcon {
                self.spawn_falcon_visual(gid);
            } else {
                self.despawn_falcon_visual(gid);
            }
        }
        // Hide/cloak/burrow deletes the aura; reappearing respawns it.
        self.refresh_level_aura(gid);
        self.refresh_boss_aura(gid);
    }

    /// Toggle a persistent body-buff visual from `ZC_MSG_STATE_CHANGE`. Each
    /// `(gid, efst)` gets a unique owner key so turning one buff off despawns
    /// exactly that effect, leaving the entity's other buffs untouched.
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
        // The push-cart status drives the trailing cart visual: `val1` carries
        // the cart design. This is the modern delivery path (the legacy OPTION
        // bit is handled in `handle_entity_option_changed`).
        if icon == ClientEffectIcon::OnPushCart {
            self.handle_push_cart_status(gid, active, val1);
            return;
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
        // Drop any prior instance first — clears it on the off-packet, and
        // re-ups the timer when the same buff is re-applied.
        if let Some(old_key) = self.game.status_buff_keys.remove(&map_key) {
            self.effect_queue.despawn(old_key);
        }
        if !active {
            return;
        }
        // High bit set so buff keys never collide with the real gid/aid values
        // other keyed effects use (Blind, ground units).
        let key = 0x8000_0000 | self.game.next_status_buff_key;
        self.game.next_status_buff_key = (self.game.next_status_buff_key + 1) & 0x7fff_ffff;
        for &id in buff.body {
            self.effect_queue.spawn_on_keyed_for(id, gid, key, remain_ms);
        }
        self.game.status_buff_keys.insert(map_key, key);
    }

    /// Apply the push-cart status to an entity: spawn the trailing cart visual
    /// (design = `val1`) when active, or tear it down when cleared. For the local
    /// player a cleared cart also empties the cart inventory and closes its
    /// window — reusing the cart-off path.
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
                // Surface the cart inventory the moment the local player gains a
                // cart so it is reachable without knowing the hotkey; re-applying
                // the same cart (relog/map change) leaves the window state alone.
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

    /// Spawn or despawn a character's level-99 aura to match its current level
    /// and visibility. The single chokepoint for the state-driven trigger:
    /// callers invoke it after an entity's level or `effect_state` changes. The
    /// aura is entity-anchored, so it follows the actor for free; every layer is
    /// keyed the same so hiding/cloaking (or leaving view) despawns the whole
    /// aura at once.
    pub(super) fn refresh_level_aura(&mut self, gid: u32) {
        let Some((entity_type, effect_state, entity_level)) = self
            .game
            .entities
            .get(gid)
            .map(|e| (e.entity_type, e.effect_state, e.base_level))
        else {
            return;
        };
        // The local player's level lives on `Character`; its entity is created
        // at login before any level is known, so its `base_level` stays 0.
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
                // The full aura is three composed layers, not just the ring:
                // the blue ring (`Level99`), the rising pikapika sparkles
                // (`Level992`), and the orbiting freezing-circle motes
                // (`Level993`) — the original game's `EF_LEVEL99` + `_2` + `_3`.
                for &id in LEVEL_AURA_LAYERS {
                    self.effect_queue.spawn_on_keyed(id, gid, key);
                }
                self.game.level_aura_keys.insert(gid, key);
            }
            (false, true) => self.despawn_level_aura(gid),
            _ => {}
        }
    }

    /// Drop a tracked level aura when its entity leaves view. The holder keeps
    /// entity-anchored effects alive after the entity is gone, so without this
    /// the ring would freeze at the actor's last cell.
    pub(crate) fn despawn_level_aura(&mut self, gid: u32) {
        if let Some(key) = self.game.level_aura_keys.remove(&gid) {
            self.effect_queue.despawn(key);
        }
    }

    /// Spawn the green level-99 aura on an MVP/boss monster. Boss-ness is fixed
    /// at spawn (`StandEntry7`'s `is_boss`, never updated by a later packet), so
    /// unlike the level aura this is evaluated once. The layers are
    /// entity-anchored, so the aura follows the monster for free, and keyed the
    /// same so leaving view despawns them together.
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

    /// The warp portal NPC renders no body in the original game — its whole
    /// visual is this persistent warp-zone effect, launched once on spawn and
    /// following the (fixed) NPC.
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

    /// Despawn every entity-anchored persistent effect tracked for `gid`. Called
    /// at each entity-removal chokepoint so auras/portals never outlive their
    /// actor.
    pub(crate) fn despawn_entity_effects(&mut self, gid: u32) {
        self.despawn_level_aura(gid);
        self.despawn_boss_aura(gid);
        self.despawn_warp_portal(gid);
    }

    /// Mint an owner key in the high-bit namespace so keyed entity effects never
    /// collide with the raw `gid`/`aid` values other keyed effects use.
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

    /// Generic server-driven effect (`ZC_NOTIFY_EFFECT2`/`3`): play a raw `EF_*`
    /// effect on the entity. The Effect3 extra datum rides `hit_count`.
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
            Some(v) if v > 0 => self.effect_queue.spawn_on_with_count(id, gid, v.min(255) as u8),
            _ => self.effect_queue.spawn_on(id, gid),
        }
    }

    /// Misc effect (`ZC_NOTIFY_EFFECT`): `code` is an `e_notify_effect` code, not
    /// an `EF_*` id, so it gets a fixed remap (same as the original game).
    pub(super) fn handle_play_misc_effect_on_entity(&mut self, gid: u32, code: u8) {
        let id = match code {
            0 => EffectId::Angel,     // base level-up
            1 => EffectId::Joblvup,   // job level-up
            2 => EffectId::Refinefail,
            3 => EffectId::Refineok,
            5 => EffectId::PharmacyOk,
            6 => EffectId::PharmacyFail,
            7 => EffectId::Angel2,    // super-novice base level-up
            8 => EffectId::Joblvup50, // super-novice job level-up
            9 => EffectId::Angel3,    // taekwon-class base level-up
            _ => return,              // 4 = game-over screen, not an effect
        };
        self.effect_queue.spawn_on(id, gid);
    }

    /// Resolve an entity's current cell to a world position at the **ground**
    /// (`get_height`), matching the sprite feet anchor and the per-frame
    /// resolver. Effects that sit higher apply their own lift from here.
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

    /// A persistent ground-skill unit appeared at a cell (`ZC_SKILL_ENTRY`).
    /// One packet per occupied cell, so the wall/area shape comes from the
    /// server's per-cell positions — we render one effect per packet, keyed by
    /// the unit `aid` so its disappear packet can remove it. Hidden units
    /// (`is_visible == false`, the server's `DUMMYSKILL` hack) render nothing.
    pub(super) fn handle_skill_unit_entered(
        &mut self,
        aid: u32,
        x: i16,
        y: i16,
        unit_id: u8,
        is_visible: bool,
    ) {
        if !is_visible {
            return;
        }
        let Some(effect) = skill_unit_effect(unit_id) else {
            return;
        };
        let (Some(gat), Some(coords)) = (self.game.gat.as_ref(), self.game.map_coords.as_ref())
        else {
            return;
        };
        let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
        let (wx, _, wz) = coords.cell_to_world(cx, cy);
        let world = [wx, gat.get_height(cx, cy), wz];
        // A re-sent entry for a live `aid` is the server relocating that unit
        // (a song area sliding with its performer), not a new one — move the
        // existing effect instead of stacking a duplicate at the old cell.
        if self.effect_holder.reposition_by_key(aid, world) {
            return;
        }
        self.effect_queue.spawn_at_keyed(effect, world, aid);
    }

    /// A ground-skill unit was removed (`ZC_SKILL_DISAPPEAR`): drop every
    /// effect spawned under its `aid`.
    pub(super) fn handle_skill_unit_disappeared(&mut self, aid: u32) {
        self.effect_queue.despawn(aid);
    }
}
