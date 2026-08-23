use crate::App;
use crate::game_state::CastMark;
use models::enums::EnumWithStringValue;
use models::enums::action::ActionType;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::data_table::skill_name_table::format_skill_display_name;
use models::enums::weapon::WeaponType;
use ragnarok_game::autocounter;
use ragnarok_game::cast_scope::CastScope;
use ragnarok_game::cursor::PendingSkillTarget;
use ragnarok_game::damage_number::{DamageNumber, DamageNumberType};
use ragnarok_game::effect::{
    beginspell_for_element, caster_cast_on_use, caster_skill_effects, casting_skill,
    fire_glyph_effect, ground_placed_effect, is_cast_circle, is_caster_link_effect, is_ground_cast,
    is_trail_effect, potion_throw_index, sevenwind_aura, suppresses_visuals_on_damage,
    target_skill_effects, trail_arrival_secs,
};
use ragnarok_game::entity::EntityType;
use ragnarok_game::event::GameEvent;
use ragnarok_game::job_class::job_class_name;
use ragnarok_game::movement::direction_from_positions;
use ragnarok_game::scheduled_hit::{DamageMessage, ScheduledHit};
use ragnarok_game::skill::SkillTargetType;
use ragnarok_game::skill_action::{SkillMotionType, skill_motion_type};
use ragnarok_game::sound::tables::{
    SkillSoundPos, skill_cast_begin_sound, skill_projectile_sound, skill_use_sound,
};
use ragnarok_game::sprite_path::hide_allows_skill;
use ragnarok_game::star_gladiator::{
    FEEL_PLACE_CONFIRM_MSG, StarSubject, TARGET_HP_RESULT, star_notice,
};
use ragnarok_network::build_change_direction_packet;
use ragnarok_network::build_shortcut_key_change_packet;
use ragnarok_network::build_use_skill_packet;

/// AL_HEAL's green heal sparkle size by healed amount, matching the original
/// game's thresholds (the tiniest and largest heals share the biggest sparkle).
fn heal_effect_for_amount(amount: i16) -> EffectId {
    match amount {
        a if a >= 4000 => EffectId::Heal4,
        a if a >= 2000 => EffectId::Heal2,
        a if a >= 200 => EffectId::Heal,
        _ => EffectId::Heal4,
    }
}

impl App {
    pub(super) fn handle_skill_list_received(
        &mut self,
        skills: Vec<ragnarok_game::event::SkillInfo>,
    ) {
        let icon_paths = self.game.character.skills.apply_skill_list(skills);
        self.preload_item_icons(icon_paths);
    }

    pub(super) fn handle_skill_added(&mut self, skill: ragnarok_game::event::SkillInfo) {
        let (id, level) = (skill.id, skill.level);
        let before_level = self.skill_level(id);
        let icon_path = self.game.character.skills.apply_skill_added(skill);
        self.preload_item_icons(vec![icon_path]);
        self.sync_hotkey_skill_level(id, before_level, level);
    }

    pub(super) fn handle_skill_updated(
        &mut self,
        id: u16,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        upgradable: bool,
    ) {
        let before_level = self.skill_level(id);
        self.game
            .character
            .skills
            .update_skill(id, level, sp_cost, attack_range, upgradable);
        self.sync_hotkey_skill_level(id, before_level, level);
    }

    fn skill_level(&self, id: u16) -> i16 {
        self.game
            .character
            .skills
            .get_skill(id)
            .map_or(0, |s| s.level)
    }

    fn sync_hotkey_skill_level(&mut self, id: u16, before_level: i16, level: i16) {
        let changed = self
            .game
            .character
            .hotkeys
            .apply_skill_level_change(id, before_level, level);
        for index in changed {
            let (is_skill, id, count) = self.game.character.hotkeys.to_server_format(index);
            self.channel.send_packet(build_shortcut_key_change_packet(
                index as u16,
                is_skill,
                id,
                count,
                self.active_packetver,
            ));
        }
    }

    pub(super) fn skill_display_name(&self, internal_name: &str) -> Option<String> {
        Some(format_skill_display_name(
            internal_name,
            self.game.data_table.skill_name.as_ref(),
        ))
    }

    pub(super) fn skill_display_name_by_id(&self, skill_id: u16) -> Option<String> {
        self.skill_display_name(SkillEnum::from_id(skill_id as u32).to_name())
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
        start_time: u32,
    ) {
        let local_ms = self.start_time.elapsed().as_millis() as u32;
        self.game
            .session
            .server_time
            .observe_server_tick(start_time, local_ms);
        // Server timeline anchor (in the past by ~half-RTT): all skill timings derive from
        // when the cast actually resolved on the server, not from the late arrival here.
        let now = self
            .game
            .session
            .server_time
            .server_to_local_secs_clamped(start_time, local_ms);
        let local_now = local_ms as f32 / 1000.0;
        let age = (local_now - now).max(0.0);

        let effective_count = match action {
            ActionType::AttackMultiple
            | ActionType::AttackMultipleNomotion
            | ActionType::AttackMultipleCritical => count.max(1) as u16,
            ActionType::Skill if count > 1 => count as u16,
            _ => 1,
        };
        let is_critical = matches!(
            action,
            ActionType::AttackCritical | ActionType::AttackMultipleCritical
        );
        tracing::info!(
            "SkillDamage: skill_id={skill_id}, src_gid={src_gid}, count={count}, action={action:?}, effective_count={effective_count}"
        );

        let display_name = self.skill_display_name_by_id(skill_id);
        self.game
            .world
            .entities
            .show_skill_chat_bubble(src_gid, skill_id, display_name);

        if let Some(wav) = skill_projectile_sound(SkillEnum::from_id(skill_id as u32)) {
            self.sound_queue.ui(wav);
        }

        // A hunter/sniper's falcon darts at the struck target on Blitz Beat and
        // Falcon Assault. Auto Blitz Beat arrives through this same packet, so it
        // is covered without extra wiring.
        if self.game.sprite_caches.falcons.contains_key(&src_gid)
            && matches!(
                SkillEnum::from_id(skill_id as u32),
                SkillEnum::HtBlitzbeat | SkillEnum::SnFalconassault
            )
            && let Some(target) = self.entity_world_pos(target_gid)
        {
            self.start_falcon_flight(src_gid, target);
        }

        let suppress_flinch = matches!(
            action,
            ActionType::AttackNomotion | ActionType::AttackMultipleNomotion
        );

        let target_pos = self
            .game
            .world
            .entities
            .get(target_gid)
            .map(|e| e.movement.cell_position());
        let mut caster_anim = None;
        if let Some(entity) = self.game.world.entities.get_mut(src_gid) {
            if let Some(dst) = target_pos {
                let src = entity.movement.cell_position();
                if let Some(dir) = direction_from_positions(src.0, src.1, dst.0, dst.1) {
                    entity.set_facing(dir);
                }
            }
            let duration = ((attack_mt as f32 / 1000.0) - age).max(0.3);
            entity.enter_skill_exec(duration, skill_id, effective_count);
            caster_anim = Some((duration, entity.action_index(), entity.direction));
        }

        // The swing connects when the caster's animation reaches its "atk"
        // keyframe; the hit, the flinch and the flying arrow all land there, on
        // the same local clock as the animation.
        let anim_hit = caster_anim
            .map(|(duration, base_action, dir)| {
                duration * self.atk_keyframe_fraction(src_gid, base_action, dir)
            })
            .unwrap_or(0.0);

        // Arrow-consuming skills fire the same flying arrow as a normal ranged
        // attack: bow skills use the Attack motion, whip/instrument skills the
        // Attack2 motion. Multi-hit skills (e.g. Arrow Vulcan) fire one per hit.
        if let Some(caster) = self.game.world.entities.get(src_gid) {
            let weapon = caster.weapon;
            // Arrow Shower rains its own nine-arrow fan off the ground packet,
            // so the per-target arrow would double up on it.
            let fires_arrow = SkillEnum::from_id(skill_id as u32) != SkillEnum::AcShower
                && match skill_motion_type(skill_id) {
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
                    self.spawn_arrow_projectile(
                        shooter_cell,
                        tp,
                        (anim_hit * 1000.0) as i32,
                        effective_count,
                    );
                }
            }
        }

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
        // arrives. Fixed-speed projectiles take longer for farther targets, so
        // the flight is measured at the actual caster→target distance.
        let projectile_distance = self
            .skill_trail_endpoints(src_gid, target_gid)
            .map(|(from, to)| {
                let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
                (dx * dx + dz * dz).sqrt()
            })
            .unwrap_or(0.0);
        // Blitz Beat / Falcon Assault have no flying trail effect — the falcon
        // itself is the projectile, so hold the hit until the bird reaches the
        // target (the falcon flight was launched at the top of this handler).
        let falcon_flight = if self.game.sprite_caches.falcons.contains_key(&src_gid)
            && matches!(
                SkillEnum::from_id(skill_id as u32),
                SkillEnum::HtBlitzbeat | SkillEnum::SnFalconassault
            ) {
            crate::sprite::falcon::FALCON_FLIGHT_OUT_SECS
        } else {
            0.0
        };
        let flight =
            Self::skill_projectile_flight_secs(skill_id, projectile_distance).max(falcon_flight);
        let hit_extra_delay =
            target_skill_effects(SkillEnum::from_id(skill_id as u32)).hit_extra_delay_secs;
        let hit_delay = anim_hit.max(flight) + hit_extra_delay;

        let double_attack_term = 0.2;
        if let Some(target) = self.game.world.entities.get_mut(target_gid) {
            for i in 0..effective_count {
                let hit_time = local_now + hit_delay + (i as f32 * double_attack_term);
                target.scheduled_hits.push(ScheduledHit {
                    message,
                    damage: per_hit_damage,
                    fire_at: hit_time,
                    attacker_gid: src_gid,
                    skill_id,
                    is_last_hit: i == effective_count - 1,
                    is_critical,
                    hit_index: i,
                    attacked_mt_secs: attacked_mt as f32 / 1000.0,
                });
            }
        }

        // The begin-cast glyph is NOT fired here: the server sends
        // `ZC_USESKILL_ACK` for every skill use (even instant ones), so it
        // already fired from `spawn_skill_begin_cast` at cast start. Firing it
        // again at the damage moment would double the cast circle.
        self.spawn_skill_attack_effect(
            skill_id,
            src_gid,
            target_gid,
            effective_count,
            level,
            damage,
        );

        // The hit spark is NOT spawned here: it must land with the damage, one
        // per hit, at each scheduled hit's fire time (a ranged skill's spark
        // would otherwise flash before the projectile reaches the target). The
        // derivation runs in `process_scheduled_hits` instead.

        let replays_caster = skill_id == SkillEnum::AsSonicblow.id() as u16
            || skill_id == SkillEnum::ChChaincrush.id() as u16
            || skill_id == SkillEnum::CgArrowvulcan.id() as u16;
        tracing::info!(
            "SkillDamage replay check: replays_caster={replays_caster}, effective_count={effective_count}"
        );
        if replays_caster && effective_count > 1 {
            if let Some(caster) = self.game.world.entities.get_mut(src_gid) {
                for i in 1..effective_count {
                    let hit_time = local_now + anim_hit + (i as f32 * double_attack_term);
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
    /// per-skill table: the caster-released `cast` glyph, the spell
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
        damage: i32,
    ) {
        let skill = SkillEnum::from_id(skill_id as u32);

        // AL_HEAL is dual-natured: cast on the living it restores HP and plays the
        // green heal from the no-damage path; cast on undead/demon it deals damage
        // and arrives here on the damage packet, where the original game plays the
        // Heal3 variant on the target. It has no projectile or caster glyph, so this
        // is its only damage-path visual.
        if skill == SkillEnum::AlHeal {
            self.effect_queue.spawn_on(EffectId::Heal3, target_gid);
            return;
        }

        // ALL_RESURRECTION is dual-natured the same way, but on undead its only
        // visual is the holy spark, which lands on the hit timeline.
        if suppresses_visuals_on_damage(skill, damage) {
            return;
        }

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
            // Execution-time caster glyph (Brandish Spear's burst, Charge
            // Arrow's spark). Fired on the damage packet rather than the cast
            // bar, so it still shows on instant casts and on skills that hide
            // their cast aura.
            for e in fire_glyph_effect(skill) {
                self.effect_queue.spawn_on(*e, src_gid);
            }
            // Self-centered AoE skills fire their caster signature from the
            // no-damage (use) packet the server sends first; re-firing it here
            // would duplicate it once per hit target and a full wind-up late.
            let caster_cast = if caster_cast_on_use(skill) {
                &[][..]
            } else {
                caster_skill_effects(skill).cast
            };
            for e in caster_cast {
                match trail {
                    // Caster-anchored-with-target (Soul Breaker): spawn on the
                    // caster so it recolors the caster body, crescent still aimed
                    // at the target via the link endpoints.
                    _ if is_caster_link_effect(*e) => {
                        self.effect_queue.spawn_link(*e, src_gid, target_gid);
                    }
                    Some((from, to)) if is_trail_effect(*e) => {
                        let (from, to) = Self::ground_erupting_trail(*e, (from, to));
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
                    let (from, to) = Self::ground_erupting_trail(e, (from, to));
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

    /// Fraction `[0, 1)` through the caster's attack/skill animation at which
    /// its swing connects — the `atk` keyframe. Defaults to mid-animation when
    /// the caster sprite or action is unavailable.
    pub(super) fn atk_keyframe_fraction(
        &self,
        caster_gid: u32,
        base_action: usize,
        direction: u8,
    ) -> f32 {
        let Some(sprite) = self.game.sprite_caches.sprites.get(&caster_gid) else {
            return 0.5;
        };
        let act = &sprite.body_act;
        let action_count = act.actions.len();
        if action_count == 0 {
            return 0.5;
        }
        let action_idx = (base_action * 8 + direction as usize) % action_count;
        let motion_count = act.actions[action_idx].motions.len();
        if motion_count == 0 {
            return 0.5;
        }
        ragnarok_formats::act::atk_keyframe_index(act, action_idx) as f32 / motion_count as f32
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
    /// Travelling projectiles fly at chest height, so trail endpoints are lifted
    /// this far above the ground (−Y is up).
    const PROJECTILE_CHEST_LIFT: f32 = 10.0;

    fn skill_trail_endpoints(&self, src_gid: u32, target_gid: u32) -> Option<([f32; 3], [f32; 3])> {
        let (gat, coords) = (
            self.game.session.gat.as_ref()?,
            self.game.session.map_coords.as_ref()?,
        );
        let cell_world = |gid: u32| {
            let (cx, cy) = self.game.world.entities.get(gid)?.movement.cell_position();
            let (wx, _, wz) = coords.cell_to_world(cx as f32 + 0.5, cy as f32 + 0.5);
            Some([
                wx,
                gat.get_height(cx as f32 + 0.5, cy as f32 + 0.5) - Self::PROJECTILE_CHEST_LIFT,
                wz,
            ])
        };
        Some((cell_world(src_gid)?, cell_world(target_gid)?))
    }

    /// Ice-spike trails (Frost Diver, Grimtooth) erupt from the ground, not the
    /// chest-height projectile line, so drop their endpoints back to the surface.
    fn ground_erupting_trail(
        id: EffectId,
        (mut from, mut to): ([f32; 3], [f32; 3]),
    ) -> ([f32; 3], [f32; 3]) {
        if matches!(id, EffectId::Frostdiver | EffectId::Grimtooth) {
            from[1] += Self::PROJECTILE_CHEST_LIFT;
            to[1] += Self::PROJECTILE_CHEST_LIFT;
        }
        (from, to)
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
        let skill = SkillEnum::from_id(skill_id as u32);
        let casting = casting_skill(skill);
        let hide_aura = casting.hide_cast_aura;
        for e in casting.begin {
            if ragnarok_profiling::debug::trace_effects() {
                tracing::info!(
                    "[effect-timing t={}ms] cast-start begin effect {} queued (skill={}, caster={caster_gid}, cast_ms={cast_ms})",
                    self.start_time.elapsed().as_millis(),
                    e.as_str(),
                    skill.to_name(),
                );
            }
            if is_cast_circle(*e) {
                // The cast circle's lifetime is the cast time; an instant cast
                // or a skill that hides its aura shows no circle.
                if cast_ms == 0 || hide_aura {
                    continue;
                }
                let e = if *e == EffectId::Beginspell {
                    beginspell_for_element(property)
                } else {
                    *e
                };
                self.effect_queue.spawn_on_for(e, caster_gid, cast_ms);
            } else {
                // Caster body-flash (e.g. Spiral Pierce's yellow flash): plays
                // for its own fixed duration, even on instant casts and when
                // the cast aura is hidden.
                self.effect_queue.spawn_on(*e, caster_gid);
            }
        }
        self.queue_skill_sound(skill_cast_begin_sound(skill), caster_gid);
    }

    /// What the cast marks out for as long as it runs: a reticle on the victim of
    /// a targeted cast, or a square of ground under a placed one. A self-cast
    /// marks nothing.
    pub(super) fn spawn_cast_mark(
        &mut self,
        skill_id: u16,
        caster_gid: u32,
        target_gid: u32,
        x: i16,
        y: i16,
        cast_ms: u32,
    ) {
        self.clear_cast_mark(caster_gid);
        if cast_ms == 0 {
            return;
        }
        if target_gid != 0 {
            if target_gid == caster_gid || self.game.world.entities.get(target_gid).is_none() {
                return;
            }
            self.effect_queue
                .spawn_on_for(EffectId::Lockon, target_gid, cast_ms);
            self.game.world.cast_marks.insert(
                caster_gid,
                CastMark::Lockon {
                    target_gid,
                    remaining: cast_ms as f32 / 1000.0,
                },
            );
            return;
        }
        if x < 0 || y < 0 {
            return;
        }
        let scope = CastScope::new(
            skill_id,
            x as u16,
            y as u16,
            self.is_hostile_caster(caster_gid),
            cast_ms as f32 / 1000.0,
        );
        self.game
            .world
            .cast_marks
            .insert(caster_gid, CastMark::Scope(scope));
    }

    /// Whether `caster_gid` placing a cast reads as an enemy, which paints its
    /// ground scope red instead of white.
    fn is_hostile_caster(&self, caster_gid: u32) -> bool {
        let props = &self.game.session.map_properties;
        let Some(caster) = self.game.world.entities.get(caster_gid) else {
            return false;
        };
        if props.is_siege() {
            let my_guild = self.game.guild.as_ref().map_or(0, |g| g.gdid);
            return caster.guild_id != my_guild;
        }
        if !props.is_pvp() || self.game.world.entities.is_player(caster_gid) {
            return false;
        }
        let in_my_party = self
            .game
            .party
            .as_ref()
            .is_some_and(|p| p.members.iter().any(|m| m.aid == caster_gid));
        !in_my_party && caster.entity_type == EntityType::Monster
    }

    /// Drops whatever `caster_gid`'s cast was marking, so an interrupted cast
    /// does not leave its reticle or ground square behind. A zero gid is the
    /// local player, as on the cancel packet.
    pub(crate) fn clear_cast_mark(&mut self, caster_gid: u32) {
        let caster_gid = if caster_gid == 0 {
            self.game.world.entities.player_id().unwrap_or(0)
        } else {
            caster_gid
        };
        if let Some(CastMark::Lockon { target_gid, .. }) =
            self.game.world.cast_marks.remove(&caster_gid)
        {
            self.effect_holder
                .despawn_effect_on_entity(EffectId::Lockon, target_gid);
        }
    }

    pub(crate) fn update_cast_marks(&mut self, delta: f32) {
        self.game.world.cast_marks.retain(|_, mark| match mark {
            CastMark::Scope(scope) => scope.tick(delta),
            CastMark::Lockon { remaining, .. } => {
                *remaining -= delta;
                *remaining > 0.0
            }
        });
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
        level: i16,
    ) {
        let skill = SkillEnum::from_id(skill_id as u32);
        // A thrown bottle (Potion Pitcher, Berserk Pitcher) travels the
        // caster→target line rather than sitting on the caster, matching the
        // original's launch-from-caster + reposition-to-target.
        let trail = self.skill_trail_endpoints(src_gid, target_gid);
        let is_sevenwind = skill == SkillEnum::TkSevenwind;
        for e in caster_skill_effects(skill).cast {
            let e = if is_sevenwind && *e == EffectId::Beginasura1 {
                sevenwind_aura(level)
            } else {
                *e
            };
            // High Jump's landing takes over from the leap: delete the airborne
            // Jumpbody so the caster drops in from above at the landing cell.
            if e == EffectId::Landbody {
                self.effect_holder
                    .despawn_effect_on_entity(EffectId::Jumpbody, src_gid);
            }
            if is_sevenwind {
                self.effect_holder.despawn_effect_on_entity(e, src_gid);
            }
            match trail {
                // Potion Pitcher throws the potion icon for its level.
                Some((from, to)) if e == EffectId::Throwitem2 => {
                    let potion = potion_throw_index(skill, level).unwrap_or(1);
                    self.effect_queue
                        .spawn_trail_with_count(e, from, to, potion);
                }
                Some((from, to)) if is_trail_effect(e) => {
                    self.effect_queue.spawn_trail(e, from, to)
                }
                _ => self.effect_queue.spawn_on(e, src_gid),
            }
        }
        // AL_HEAL and WE_MALE ("I Will Protect You") share the amount-tiered green
        // heal glyph and rising green number driven by the packet's level field.
        let is_heal_tier = matches!(skill, SkillEnum::AlHeal | SkillEnum::WeMale);
        // The rising body glyph is a player-sprite animation, so only a player
        // target gets it; anything else keeps just the ground burst.
        let target_is_player = self
            .game
            .world
            .entities
            .get(target_gid)
            .is_some_and(|e| e.entity_type == EntityType::Player);
        for e in target_skill_effects(skill).on_target {
            if *e == EffectId::Revive && !target_is_player {
                continue;
            }
            let e = if is_heal_tier && *e == EffectId::Heal {
                heal_effect_for_amount(level)
            } else {
                *e
            };
            match trail {
                Some((from, to)) if is_trail_effect(e) => {
                    self.effect_queue.spawn_trail(e, from, to)
                }
                _ => self.effect_queue.spawn_on(e, target_gid),
            }
        }
        if is_heal_tier && level > 0 {
            self.game.combat.damage_numbers.add(DamageNumber::new(
                target_gid,
                level as i32,
                DamageNumberType::Heal,
                0.0,
            ));
        }
        // WE_FEMALE ("I Look up to You") restores partner SP: a light-blue rising
        // recovery number.
        if skill == SkillEnum::WeFemale && level > 0 {
            self.game
                .combat
                .damage_numbers
                .add(DamageNumber::effect_number(
                    target_gid,
                    level as i32,
                    [85.0 / 255.0, 177.0 / 255.0, 255.0 / 255.0],
                    0.0,
                ));
        }
        // Potion Pitcher and the Slim/Berserk variants report the SP they restored
        // as a MG_SRECOVERY no-damage packet whose level field carries the amount:
        // a blue rising recovery number on the target, no glyph.
        if skill == SkillEnum::MgSrecovery && level > 0 {
            self.game
                .combat
                .damage_numbers
                .add(DamageNumber::effect_number(
                    target_gid,
                    level as i32,
                    [0.0, 0.0, 1.0],
                    0.0,
                ));
        }
        self.spawn_wedding_balloon(skill, src_gid, target_gid);
        if skill == SkillEnum::WeCallpartner {
            self.spawn_call_partner_balloon(src_gid);
        }
        self.queue_skill_sound(skill_use_sound(skill), target_gid);
    }

    /// The wedding skills shout a love line over the caster's head (the original
    /// has no generic skill-shout system, so this is WE-skills only). WE_MALE /
    /// WE_FEMALE address the auto-targeted partner by the target entity's name;
    /// WE_CALLPARTNER uses the stored couple name.
    fn spawn_wedding_balloon(&mut self, skill: SkillEnum, src_gid: u32, target_gid: u32) {
        let love_line = match skill {
            SkillEnum::WeMale => "I will protect you",
            SkillEnum::WeFemale => "I look up to you",
            _ => return,
        };
        let partner = self
            .game
            .world
            .entities
            .get(target_gid)
            .and_then(|e| e.name.clone())
            .unwrap_or_default();
        let message = format!("{partner} !!  {love_line}");
        self.game.world.entities.set_chat_bubble(src_gid, message);
    }

    /// WE_CALLPARTNER ("I miss You") balloon uses the couple name stored from
    /// ZC_COUPLENAME.
    pub(super) fn spawn_call_partner_balloon(&mut self, src_gid: u32) {
        let partner = self.game.character.partner_name.clone();
        if partner.is_empty() {
            return;
        }
        let message = format!("{partner} !!  I miss you");
        self.game.world.entities.set_chat_bubble(src_gid, message);
    }

    fn queue_skill_sound(&mut self, sound: Option<(&'static str, SkillSoundPos)>, target_gid: u32) {
        let Some((wav, pos)) = sound else { return };
        match pos {
            SkillSoundPos::NonPositional => self.sound_queue.ui(wav),
            SkillSoundPos::Depth(d) => self.sound_queue.ui_at_depth(wav, d),
            SkillSoundPos::TargetPositional => match self.entity_world_pos(target_gid) {
                Some(p) => self.sound_queue.world(wav, p),
                None => self.sound_queue.ui(wav),
            },
        }
    }

    pub(crate) fn start_autocounter_channel(&mut self, gid: u32) {
        let skill_id = SkillEnum::KnAutocounter.id() as u16;
        let is_player = self.game.world.entities.player_id() == Some(gid);
        let attack_target = if is_player {
            self.game.combat.attack_target_id.take()
        } else {
            None
        };
        let params = autocounter::channel_params(
            &self.game.character,
            is_player,
            self.game.combat.last_attacked_enemy,
            attack_target,
        );
        self.game.world.entities.apply_autocounter_channel(
            gid,
            params.face,
            skill_id,
            params.duration,
        );
        if is_player
            && params.face.is_some()
            && let Some(dir) = self.game.world.entities.player().map(|e| e.direction)
        {
            self.channel
                .send_packet(build_change_direction_packet(0, dir, self.active_packetver));
        }
    }

    pub(crate) fn dispel_autocounter(&mut self) {
        if autocounter::player_in_autocounter(&self.game.world.entities)
            && let Some(gid) = self.game.world.entities.player_id()
        {
            self.game.world.entities.apply_skill_cast_cancel(gid);
        }
    }

    pub(crate) fn fire_autocounter_on_cancel(&mut self, cancel_gid: u32) {
        let Some(player_gid) = self.game.world.entities.player_id() else {
            return;
        };
        if !autocounter::player_in_autocounter(&self.game.world.entities)
            || (cancel_gid != 0 && cancel_gid != player_gid)
        {
            return;
        }
        self.effect_queue
            .spawn_on(EffectId::Autocounter, player_gid);
    }

    pub(super) fn spawn_ground_skill_effects(
        &mut self,
        skill_id: u16,
        src_gid: u32,
        level: i16,
        x: i16,
        y: i16,
    ) {
        let skill = SkillEnum::from_id(skill_id as u32);
        let effects = ground_placed_effect(skill, level);
        if effects.is_empty() {
            return;
        }
        let (Some(gat), Some(coords)) = (
            self.game.session.gat.as_ref(),
            self.game.session.map_coords.as_ref(),
        ) else {
            return;
        };
        let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
        let (wx, _, wz) = coords.cell_to_world(cx, cy);
        let world = [wx, gat.get_height(cx, cy), wz];
        // Slim Potion Pitcher lobs the level's slim potion from the caster onto
        // the target cell before the splash lands.
        if let Some(potion) = potion_throw_index(skill, level)
            && let Some(caster) = self.game.world.entities.get(src_gid)
        {
            let (ccx, ccy) = caster.movement.cell_position();
            let (fx, _, fz) = coords.cell_to_world(ccx as f32 + 0.5, ccy as f32 + 0.5);
            let from = [
                fx,
                gat.get_height(ccx as f32 + 0.5, ccy as f32 + 0.5) - Self::PROJECTILE_CHEST_LIFT,
                fz,
            ];
            self.effect_queue
                .spawn_trail_with_count(EffectId::Throwitem2, from, world, potion);
        }
        for e in effects {
            self.effect_queue.spawn_at(*e, world);
        }
    }

    /// A (global or per-skill) cooldown is still running, so the skill cannot
    /// be cast yet. Targeting mode is still entered on cooldown so the cursor
    /// keeps showing the skill ring; only the actual cast is suppressed.
    pub(crate) fn skill_on_cooldown(&self, skill_id: u16) -> bool {
        let now = self.start_time.elapsed().as_secs_f32();
        self.game.character.cooldowns.is_on_cooldown(skill_id, now)
    }

    pub(super) fn handle_skill_failed(&mut self, skill_id: u16, cause: u8) {
        self.game.pending_casts.pending_skill_target = None;
        self.game.pending_casts.pending_skill_id = None;
        self.game.pending_casts.pending_skill_level = None;
        let msg = ragnarok_game::skill::skill_failure_message(cause).unwrap_or("Skill failed.");
        tracing::info!("Skill {skill_id} failed (cause: {cause}): {msg}");
        self.windows.chat_window.add_error(msg.to_string());
    }
}

impl App {
    pub(super) fn handle_star_place_request(&mut self, which: i8) {
        let Some(message) = self
            .game
            .data_table
            .msg_string
            .as_ref()
            .and_then(|t| t.get(FEEL_PLACE_CONFIRM_MSG))
            .map(str::to_string)
        else {
            return;
        };
        self.game
            .arm_confirm(&mut self.windows, &message, move |accept| {
                accept.then_some(GameEvent::RequestAgreeStarPlace { which })
            });
    }

    pub(super) fn handle_star_skill_notice(
        &mut self,
        map_name: String,
        monster_id: i32,
        star: u8,
        result: u8,
    ) {
        if result == TARGET_HP_RESULT {
            let text = format!("Target HP : {monster_id}");
            self.game.broadcast.poptip.push(text.clone());
            self.windows.chat_window.add_system(text);
            return;
        }
        let Some(notice) = star_notice(result, star) else {
            return;
        };
        let subject = match notice.subject {
            StarSubject::FeelPlace => self
                .game
                .data_table
                .map_name
                .as_ref()
                .and_then(|t| t.display_name(&map_name))
                .unwrap_or(&map_name)
                .to_string(),
            StarSubject::HateMonster | StarSubject::MissionProgress | StarSubject::Mission
                if map_name.is_empty() =>
            {
                job_class_name(monster_id as u16)
            }
            StarSubject::HateMonster | StarSubject::MissionProgress | StarSubject::Mission => {
                map_name.clone()
            }
            StarSubject::MissionItem => self
                .game
                .data_table
                .item_name
                .as_ref()
                .map(|t| t.get_name_or_id(monster_id as u16))
                .unwrap_or_default(),
            StarSubject::Nothing => String::new(),
        };
        let progress = star.to_string();
        let args: &[&str] = match notice.subject {
            StarSubject::FeelPlace | StarSubject::HateMonster => {
                &[&self.game.character.name, &subject]
            }
            StarSubject::MissionProgress => &[&subject, &progress],
            StarSubject::Mission | StarSubject::MissionItem => &[&subject],
            StarSubject::Nothing => &[],
        };
        let Some(text) = self
            .game
            .data_table
            .msg_string
            .as_ref()
            .and_then(|t| t.format(notice.msg_id, args))
        else {
            return;
        };
        self.game.broadcast.poptip.push(text.clone());
        self.windows.chat_window.add_system(text);

        if let (Some(effect), Some(player_gid)) =
            (notice.effect, self.game.world.entities.player_id())
        {
            self.effect_queue.spawn_on(effect, player_gid);
        }
        if notice.chime {
            self.sound_queue.ui("effect\\piring.wav");
        }
    }

    pub(super) fn handle_request_use_skill(&mut self, skill_id: u16, level: i16) {
        self.request_use_skill(skill_id, level, true);
    }

    /// A skill the server told us to run comes from an item, not from the skill
    /// bar, so the caster's own cooldown never gates it — a second Fly Wing has
    /// to fire while the first teleport's after-cast delay is still ticking.
    pub(super) fn handle_item_use_skill(&mut self, skill_id: u16, level: i16) {
        self.request_use_skill(skill_id, level, false);
    }

    fn request_use_skill(&mut self, skill_id: u16, level: i16, respect_cooldown: bool) {
        if self.player_hidden() && !hide_allows_skill(skill_id) {
            return;
        }
        if skill_id == SkillEnum::McChangecart.id() as u16 {
            if self.game.character.cart_design.is_some() {
                self.preload_cart_previews(&[1, 2, 3, 4, 5]);
                self.windows.cart_select_window.open();
            }
            return;
        }
        if skill_id == SkillEnum::AcMakingarrow.id() as u16
            || skill_id == SkillEnum::SaCreatecon.id() as u16
        {
            self.game.pending_casts.pending_list_skill = Some(skill_id);
        }
        let skill_target_type = self
            .game
            .resolve_cast_skill(skill_id)
            .map(|(target_type, _)| target_type)
            .unwrap_or(SkillTargetType::Target);
        match skill_target_type {
            SkillTargetType::MySelf => {
                if !respect_cooldown || !self.skill_on_cooldown(skill_id) {
                    let target_id = self.game.world.entities.player_id().unwrap_or(0);
                    self.channel.send_packet(build_use_skill_packet(
                        skill_id,
                        level,
                        target_id,
                        self.active_packetver,
                    ));
                }
            }
            SkillTargetType::Target | SkillTargetType::Friend => {
                self.game.pending_casts.pending_skill_target =
                    Some(PendingSkillTarget::Entity { skill_id, level });
                self.game.pending_casts.pending_skill_id = Some(skill_id);
                self.game.pending_casts.pending_skill_level = Some(level);
            }
            SkillTargetType::Ground => {
                self.game.pending_casts.pending_skill_target =
                    Some(PendingSkillTarget::Ground { skill_id, level });
            }
            SkillTargetType::Trap => {
                self.game.pending_casts.pending_skill_target =
                    Some(PendingSkillTarget::SkillUnit { skill_id, level });
            }
            _ => {
                tracing::debug!(
                    "Skill target type {:?} not yet supported for skill {skill_id}",
                    skill_target_type
                );
            }
        }
    }
}
