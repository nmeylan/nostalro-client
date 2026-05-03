use crate::App;
use models::enums::action::ActionType;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::movement::direction_from_positions;
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
        tracing::info!("SkillDamage: skill_id={skill_id}, src_gid={src_gid}, count={count}, action={action:?}, effective_count={effective_count}");

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

        // Show chat bubble on caster (e.g., "SM_BASH !!")
        if let Some(name) = skill_name
            && let Some(entity) = self.game.entities.get_mut(src_gid)
                && entity.entity_type != ragnarok_game::entity::EntityType::Monster {
                    entity.chat_bubble = Some(ragnarok_game::entity::ChatBubbleState::new(
                        format!("{} !!", name),
                    ));
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

        let replays_caster = skill_id == SkillEnum::AsSonicblow.id() as u16
            || skill_id == SkillEnum::ChChaincrush.id() as u16
            || skill_id == SkillEnum::CgArrowvulcan.id() as u16;
        tracing::info!("SkillDamage replay check: replays_caster={replays_caster}, effective_count={effective_count}");
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

    pub(super) fn handle_skill_failed(&mut self, skill_id: u16, cause: u8) {
        self.game.pending_skill_target = None;
        self.game.pending_skill_id = None;
        self.game.pending_skill_level = None;
        let msg = ragnarok_game::skill::skill_failure_message(cause);
        tracing::info!("Skill {skill_id} failed (cause: {cause}): {msg}");
        self.game.chat_window.add_system(msg.to_string());
    }
}
