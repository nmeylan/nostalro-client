use crate::App;
use models::enums::action::ActionType;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;
use models::enums::weapon::WeaponType;
use ragnarok_game::effect::{
    begin_cast_effect, beginspell_for_element, caster_skill_effects, ground_placed_effect,
    is_ground_cast, is_trail_effect, target_skill_effects, trail_arrival_secs,
};
use ragnarok_game::movement::direction_from_positions;
use ragnarok_game::skill_action::{skill_motion_type, SkillMotionType};
use ragnarok_game::scheduled_hit::{DamageMessage, ScheduledHit};

impl App {
    pub(super) fn handle_skill_list_received(
        &mut self,
        skills: Vec<ragnarok_game::event::SkillInfo>,
    ) {
        let icon_paths = self.game.character.skills.apply_skill_list(skills);
        self.preload_item_icons(icon_paths);
    }

    pub(super) fn handle_skill_added(&mut self, skill: ragnarok_game::event::SkillInfo) {
        let icon_path = self.game.character.skills.apply_skill_added(skill);
        self.preload_item_icons(vec![icon_path]);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_skill_damage(
        &mut self,
        skill_id: u16,
        src_gid: u32,
        target_gid: u32,
        damage: i32,
        attack_mt: i32,
        attacked_mt: i32,
        count: i16,
        level: i16,
        action: ActionType,
        skill_name: Option<String>,
        start_time: u32,
    ) {
        let local_ms = self.start_time.elapsed().as_millis() as u32;
        self.game.server_time.observe_server_tick(start_time, local_ms);
        // Server timeline anchor (in the past by ~half-RTT): all skill timings derive from
        // when the cast actually resolved on the server, not from the late arrival here.
        let now = self
            .game
            .server_time
            .server_to_local_secs_clamped(start_time, local_ms);
        let age = (local_ms as f32 / 1000.0 - now).max(0.0);

        let effective_count = match action {
            ActionType::AttackMultiple | ActionType::AttackMultipleNomotion => count.max(1) as u16,
            ActionType::Skill if count > 1 => count as u16,
            _ => 1,
        };
        tracing::info!(
            "SkillDamage: skill_id={skill_id}, src_gid={src_gid}, count={count}, action={action:?}, effective_count={effective_count}"
        );

        let suppress_flinch = matches!(
            action,
            ActionType::AttackNomotion | ActionType::AttackMultipleNomotion
        ) || (action == ActionType::Skill && effective_count == 1);

        let target_pos = self
            .game
            .entities
            .get(target_gid)
            .map(|e| e.movement.cell_position());
        if let Some(entity) = self.game.entities.get_mut(src_gid) {
            if let Some(dst) = target_pos {
                let src = entity.movement.cell_position();
                if let Some(dir) = direction_from_positions(src.0, src.1, dst.0, dst.1) {
                    entity.direction = dir;
                }
            }
            let duration = ((attack_mt as f32 / 1000.0) - age).max(0.3);
            entity.enter_skill_exec(duration, skill_id, effective_count);
        }

        // Arrow-consuming skills fire the same flying arrow as a normal ranged
        // attack: bow skills use the Attack motion, whip/instrument skills the
        // Attack2 motion. Multi-hit skills (e.g. Arrow Vulcan) fire one per hit.
        if let Some(caster) = self.game.entities.get(src_gid) {
            let weapon = caster.weapon;
            let fires_arrow = match skill_motion_type(skill_id) {
                SkillMotionType::Attack => weapon == Some(WeaponType::Bow),
                SkillMotionType::Attack2 => matches!(
                    weapon,
                    Some(WeaponType::Bow | WeaponType::Whip | WeaponType::Musical)
                ),
                _ => false,
            };
            if fires_arrow {
                let shooter_cell = caster.movement.cell_position();
                if let Some(tp) = target_pos {
                    self.spawn_arrow_projectile(shooter_cell, tp, attack_mt, effective_count);
                }
            }
        }

        // Show chat bubble on caster (e.g., "SM_BASH !!")
        if let Some(name) = skill_name
            && let Some(entity) = self.game.entities.get_mut(src_gid)
            && entity.entity_type != ragnarok_game::entity::EntityType::Monster
        {
            entity.chat_bubble = Some(ragnarok_game::entity::ChatBubbleState::new(format!(
                "{} !!",
                name
            )));
        }

        let delay_time = (attack_mt as f32 / 2000.0).max(0.0);
        let per_hit_damage = if effective_count > 1 && damage > 0 {
            damage / effective_count as i32
        } else {
            damage
        };

        let message = if suppress_flinch {
            DamageMessage::AttackedNoMotion
        } else if effective_count > 1 {
            DamageMessage::AttackedMultiHit {
                total_damage: damage,
            }
        } else {
            DamageMessage::Attacked
        };

        // A trailing projectile (Fireball, Soul Strike, …) takes time to reach
        // the target. Hold the hit — spark, damage number and flinch — until it
        // arrives, mirroring how the flying arrow lands on the scheduled hit.
        // The projectile plays from real time, so its arrival sits `age` (the
        // server-anchor backdate) ahead of `now`. Fixed-speed projectiles take
        // longer for farther targets, so the flight is measured at the actual
        // caster→target distance.
        let projectile_distance = self
            .skill_trail_endpoints(src_gid, target_gid)
            .map(|(from, to)| {
                let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
                (dx * dx + dz * dz).sqrt()
            })
            .unwrap_or(0.0);
        let hit_delay = match Self::skill_projectile_flight_secs(skill_id, projectile_distance) {
            flight if flight > 0.0 => delay_time.max(age + flight),
            _ => delay_time,
        };

        let double_attack_term = 0.2;
        if let Some(target) = self.game.entities.get_mut(target_gid) {
            for i in 0..effective_count {
                let hit_time = now + hit_delay + (i as f32 * double_attack_term);
                target.scheduled_hits.push(ScheduledHit {
                    message,
                    damage: per_hit_damage,
                    fire_at: hit_time,
                    attacker_gid: src_gid,
                    skill_id,
                    is_last_hit: i == effective_count - 1,
                    is_critical: false,
                    hit_index: i,
                    attacked_mt_secs: attacked_mt as f32 / 1000.0,
                });
            }
        }

        // The begin-cast glyph is NOT fired here: the server sends
        // `ZC_USESKILL_ACK` for every skill use (even instant ones), so it
        // already fired from `spawn_skill_begin_cast` at cast start. Firing it
        // again at the damage moment would double the cast circle.
        self.spawn_skill_attack_effect(skill_id, src_gid, target_gid, effective_count, level);

        // The hit spark is NOT spawned here: it must land with the damage, one
        // per hit, at each scheduled hit's fire time (a ranged skill's spark
        // would otherwise flash before the projectile reaches the target). The
        // §2d derivation runs in `process_scheduled_hits` instead.

        let replays_caster = skill_id == SkillEnum::AsSonicblow.id() as u16
            || skill_id == SkillEnum::ChChaincrush.id() as u16
            || skill_id == SkillEnum::CgArrowvulcan.id() as u16;
        tracing::info!(
            "SkillDamage replay check: replays_caster={replays_caster}, effective_count={effective_count}"
        );
        if replays_caster && effective_count > 1 {
            if let Some(caster) = self.game.entities.get_mut(src_gid) {
                for i in 1..effective_count {
                    let hit_time = now + delay_time + (i as f32 * double_attack_term);
                    caster.pending_attack_replays.push((hit_time, skill_id));
                }
                tracing::info!(
                    "Scheduled {} caster replays for entity {src_gid}",
                    effective_count - 1
                );
            } else {
                tracing::warn!("Caster entity {src_gid} NOT FOUND for replay scheduling");
            }
        }
    }

    // TODO(linelink): wire the Soul Linker tether here once a packet exposes
    // the partner AID. The renderer side is ready — call
    // `self.effect_queue.spawn_link(EffectId::Linelink{,2,3}, caster_gid,
    // partner_gid)` and the holder will track both actors live each frame.

    /// Damage-skill visuals fired at the `ZC_NOTIFY_SKILL` moment, read from the
    /// per-skill table (§2c/§2d): the caster-released `cast` glyph, the spell
    /// landing on the target (`on_target`), and the projectile (`before_hit`).
    /// The per-hit impact spark is the separate `hit` slot, fired on the
    /// scheduled-hit timeline (`process_scheduled_hits`), not here.
    fn spawn_skill_attack_effect(
        &mut self,
        skill_id: u16,
        src_gid: u32,
        target_gid: u32,
        count: u16,
        level: i16,
    ) {
        let skill = SkillEnum::from_id(skill_id as u32);

        // Ground-cast skills (Storm Gust, Thunderstorm, …) launch their
        // `cast`/`on_target` slots once from `ZC_NOTIFY_GROUNDSKILL` (placed at
        // the cell), matching the original's split between the ground packet
        // and the damage packet. The damage path here plays only the per-hit
        // spark for them, so skip the use-time slots.
        let ground_cast = is_ground_cast(skill);

        // Caster→target endpoints, snapshotted once at spawn — faithful to the
        // original, which passes the positions by value rather than re-tracking
        // a moving target. Shared by every slot that can hold a travelling trail
        // (caster-released `cast`, `on_target`, `before_hit`).
        let trail = self.skill_trail_endpoints(src_gid, target_gid);

        // The skill's hit count drives how many sub-projectiles/sub-bolts the
        // effect renders (one per hit). Pass it through verbatim — each effect
        // clamps to its own meaningful range (Soul Strike to its soul count,
        // etc.). Only narrow to the `u8` the channel carries.
        let hits = count.min(u8::MAX as u16) as u8;

        // Caster-released visual (Pierce self-glyph, the boomerang out-and-back).
        // Once at the damage moment. Trail effects here (Grimtooth, Finger
        // Offensive, Shield Boomerang) are projectiles released from the caster,
        // so they travel the caster→target line rather than collapsing onto the
        // caster; the rest are self-anchored glyphs/auras.
        if !ground_cast {
            for e in caster_skill_effects(skill).cast {
                match trail {
                    Some((from, to)) if is_trail_effect(*e) => {
                        self.effect_queue.spawn_trail_with_count(*e, from, to, hits);
                    }
                    _ => self.effect_queue.spawn_on(*e, src_gid),
                }
            }
        }

        let target = target_skill_effects(skill);
        // Spell landing on the target (bolts, Brandish, Frost Diver). The effect
        // renders the per-hit sub-bolts itself. Trail effects parked here
        // (Fireball, Waterball, Jupitel) travel the caster→target line instead
        // of collapsing onto the target, matching how the viewer routes them.
        for e in target.on_target.iter().filter(|_| !ground_cast) {
            // Lif Moonlight visual scales with skill level (1/2/3+ -> moon 1/2/3).
            let e = match (*e, level) {
                (EffectId::Hflimoon1, 2) => EffectId::Hflimoon2,
                (EffectId::Hflimoon1, l) if l >= 3 => EffectId::Hflimoon3,
                (other, _) => other,
            };
            match trail {
                Some((from, to)) if is_trail_effect(e) => {
                    self.effect_queue.spawn_trail_with_count(e, from, to, hits);
                }
                _ => self.effect_queue.spawn_on_with_count(e, target_gid, hits),
            }
        }

        // Projectile toward the target (Soul Strike).
        if !target.before_hit.is_empty()
            && let Some((from, to)) = trail
        {
            for e in target.before_hit {
                self.effect_queue.spawn_trail_with_count(*e, from, to, hits);
            }
        }
    }

    /// Seconds the skill's trailing projectile takes to reach a target
    /// `distance_units` away, taken across every slot that can launch one — the
    /// caster-released `cast` (Shield Boomerang, Grimtooth), `on_target` and
    /// `before_hit` (the longest wins). Fixed-speed projectiles scale with
    /// distance; fixed-frame ones ignore it. `0.0` if no timed projectile.
    fn skill_projectile_flight_secs(skill_id: u16, distance_units: f32) -> f32 {
        let skill = SkillEnum::from_id(skill_id as u32);
        let t = target_skill_effects(skill);
        caster_skill_effects(skill)
            .cast
            .iter()
            .chain(t.on_target.iter())
            .chain(t.before_hit.iter())
            .filter_map(|e| trail_arrival_secs(*e, distance_units))
            .fold(0.0_f32, f32::max)
    }

    /// Feet-world positions of caster and target for a projectile trail. `None`
    /// if the map or either entity is missing.
    fn skill_trail_endpoints(
        &self,
        src_gid: u32,
        target_gid: u32,
    ) -> Option<([f32; 3], [f32; 3])> {
        let (gat, coords) = (self.game.gat.as_ref()?, self.game.map_coords.as_ref()?);
        let cell_world = |gid: u32| {
            let (cx, cy) = self.game.entities.get(gid)?.movement.cell_position();
            let (wx, _, wz) = coords.cell_to_world(cx as f32 + 0.5, cy as f32 + 0.5);
            Some([wx, gat.get_height(cx as f32 + 0.5, cy as f32 + 0.5) - 10.0, wz])
        };
        Some((cell_world(src_gid)?, cell_world(target_gid)?))
    }

    /// Begin-cast glyph on the caster — the begin-spell cast circle fired at
    /// `ZC_USESKILL_ACK` (cast starts), suppressed for skills that hide their
    /// cast aura (Bowling Bash, Brandish, Spiral Pierce).
    ///
    /// `cast_ms` is the cast time from the packet; the cast circle's lifetime is
    /// exactly that duration (matching the original game). A zero cast time
    /// (instant cast, e.g. Sight, or any spell under full instant-cast) shows no
    /// circle at all — the original ties the begin glyph's lifetime to the cast
    /// time, so a zero duration renders nothing.
    pub(super) fn spawn_skill_begin_cast(
        &mut self,
        skill_id: u16,
        caster_gid: u32,
        property: u32,
        cast_ms: u32,
    ) {
        if cast_ms == 0 {
            return;
        }
        let skill = SkillEnum::from_id(skill_id as u32);
        if caster_skill_effects(skill).hide_cast_aura {
            return;
        }
        for e in begin_cast_effect(skill) {
            let e = if *e == EffectId::Beginspell {
                beginspell_for_element(property)
            } else {
                *e
            };
            self.effect_queue.spawn_on_for(e, caster_gid, cast_ms);
        }
    }

    /// Cast effect on the caster + landing effect on the recipient for
    /// no-damage skills — the `cast` / `on_target` slots fired at
    /// `ZC_USE_SKILL` (buffs, heals, status grants). The damage-skill cast /
    /// projectile path is wired separately in B5.
    pub(super) fn spawn_skill_no_damage_effects(
        &mut self,
        skill_id: u16,
        src_gid: u32,
        target_gid: u32,
    ) {
        let skill = SkillEnum::from_id(skill_id as u32);
        for e in caster_skill_effects(skill).cast {
            self.effect_queue.spawn_on(*e, src_gid);
        }
        for e in target_skill_effects(skill).on_target {
            self.effect_queue.spawn_on(*e, target_gid);
        }
    }

    /// Position-cast skill effects (`ZC_NOTIFY_GROUNDSKILL`): place the skill's
    /// AoE visual **at the targeted cell**, matching the original game's
    /// `Am_Groundskill` per-skill cell placement (Storm Gust's storm, Meteor's
    /// strike, Lord of Vermilion's field, Thunderstorm's bolts — all on the
    /// ground, not the caster). Unit skills (Volcano, traps, Ice Wall, …) render
    /// from their unit packets instead, so `ground_placed_effect` omits them.
    /// The damage path skips these skills' `cast`/`on_target` slots
    /// ([`is_ground_cast`]); per-target hit sparks still come from the damage packet.
    pub(super) fn spawn_ground_skill_effects(&mut self, skill_id: u16, level: i16, x: i16, y: i16) {
        let skill = SkillEnum::from_id(skill_id as u32);
        let effects = ground_placed_effect(skill, level);
        if effects.is_empty() {
            return;
        }
        let (Some(gat), Some(coords)) = (self.game.gat.as_ref(), self.game.map_coords.as_ref())
        else {
            return;
        };
        let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
        let (wx, _, wz) = coords.cell_to_world(cx, cy);
        let world = [wx, gat.get_height(cx, cy), wz];
        for e in effects {
            self.effect_queue.spawn_at(*e, world);
        }
    }

    pub(super) fn handle_skill_failed(&mut self, skill_id: u16, cause: u8) {
        self.game.pending_skill_target = None;
        self.game.pending_skill_id = None;
        self.game.pending_skill_level = None;
        let msg = ragnarok_game::skill::skill_failure_message(cause);
        tracing::info!("Skill {skill_id} failed (cause: {cause}): {msg}");
        self.game.chat_window.add_system(msg.to_string());
    }
}
