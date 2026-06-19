use crate::App;
use models::enums::EnumWithNumberValue;
use models::enums::action::ActionType;
use models::enums::class::JobName;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;
use models::enums::weapon::WeaponType;
use ragnarok_game::effect::{derive_hit_effect, skill_effects};
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
        action: ActionType,
        skill_name: Option<String>,
    ) {
        let now = self.start_time.elapsed().as_secs_f32();

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
            let duration = (attack_mt as f32 / 1000.0).max(0.3);
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

        let double_attack_term = 0.2;
        if let Some(target) = self.game.entities.get_mut(target_gid) {
            for i in 0..effective_count {
                let hit_time = now + delay_time + (i as f32 * double_attack_term);
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

        self.spawn_skill_attack_effect(skill_id, src_gid, target_gid, effective_count);

        // Hit spark on the target (§2d derivation): per-skill spark, else the
        // generic EF_HIT2, suppressed for self-visual skills and self-targets.
        // Single spawn for now; B5 refines to one per hit at each hit's fire time.
        let skill = SkillEnum::from_id(skill_id as u32);
        let attacker_job = self
            .game
            .entities
            .get(src_gid)
            .and_then(|e| JobName::try_from_value(e.job as usize).ok())
            .unwrap_or(JobName::Novice);
        for hit in derive_hit_effect(Some(skill), false, attacker_job, src_gid == target_gid) {
            self.effect_queue.spawn_on(*hit, target_gid);
        }

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

    fn spawn_skill_attack_effect(
        &mut self,
        skill_id: u16,
        src_gid: u32,
        target_gid: u32,
        count: u16,
    ) {
        let effect_id = match skill_id {
            x if x == SkillEnum::MgSoulstrike.id() as u16 => EffectId::Soulstrike,
            x if x == SkillEnum::MgColdbolt.id() as u16 => EffectId::Icearrow,
            x if x == SkillEnum::MgFirebolt.id() as u16 => EffectId::Firearrow,
            _ => return,
        };
        let (Some(gat), Some(coords)) = (&self.game.gat, &self.game.map_coords) else {
            return;
        };
        let Some(src) = self.game.entities.get(src_gid) else {
            return;
        };
        let Some(dst) = self.game.entities.get(target_gid) else {
            return;
        };
        let (sx, sy) = src.movement.cell_position();
        let (dx, dy) = dst.movement.cell_position();

        let (wx, _, wz) = coords.cell_to_world(sx as f32 + 0.5, sy as f32 + 0.5);
        let wy = gat.get_height(sx as f32 + 0.5, sy as f32 + 0.5);
        let from = [wx, wy - 10.0, wz];

        let (wx, _, wz) = coords.cell_to_world(dx as f32 + 0.5, dy as f32 + 0.5);
        let wy = gat.get_height(dx as f32 + 0.5, dy as f32 + 0.5);
        let to = [wx, wy - 10.0, wz];

        match effect_id {
            // Soul Strike's bolts fly from the caster and converge on the target.
            EffectId::Soulstrike => {
                self.effect_queue
                    .spawn_trail_with_count(effect_id, from, to, count.min(5) as u8);
            }
            // Cold Bolt / Fire Bolt rain onto the target; the bolt count is the
            // number of hits (the spell level).
            _ => {
                self.effect_queue
                    .spawn_at_with_count(effect_id, to, count.min(10) as u8);
            }
        }
    }

    /// Begin-cast glyph on the caster — the `begin_cast` slot fired at
    /// `ZC_USESKILL_ACK` (cast starts). Element-colored begin-spell circles
    /// and special cast glyphs route through the per-skill table (§2c/§2d).
    pub(super) fn spawn_skill_begin_cast(&mut self, skill_id: u16, caster_gid: u32) {
        for e in skill_effects(SkillEnum::from_id(skill_id as u32)).begin_cast {
            self.effect_queue.spawn_on(*e, caster_gid);
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
        let fx = skill_effects(SkillEnum::from_id(skill_id as u32));
        for e in fx.cast {
            self.effect_queue.spawn_on(*e, src_gid);
        }
        for e in fx.on_target {
            self.effect_queue.spawn_on(*e, target_gid);
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
