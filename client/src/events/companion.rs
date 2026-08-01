use crate::App;
use ragnarok_game::companion::{HomunculusState, MercenaryState};
use ragnarok_game::cursor::{CompanionSkillTarget, PendingCompanionSkill};
use ragnarok_game::event::{HomunculusProperty, MercenaryInfo, SkillInfo};
use ragnarok_game::skill::SkillTargetType;

/// e_hom_state2: SP_ACK carries the companion GID, SP_INTIMATE / SP_HUNGRY update those meters.
const HOM_STATE_ACK: i8 = 0;
const HOM_STATE_INTIMACY: i8 = 1;
const HOM_STATE_HUNGRY: i8 = 2;

impl App {
    pub(super) fn handle_companion_state_changed(&mut self, state: i8, gid: u32, data: i32) {
        match state {
            HOM_STATE_ACK => {
                match &mut self.game.companions.homunculus {
                    Some(h) => {
                        h.gid = gid;
                        h.vaporized = false;
                    }
                    None => self.game.companions.homunculus = Some(HomunculusState::new(gid)),
                }
                self.windows.homunculus_window.set_visible(true);
            }
            HOM_STATE_INTIMACY => {
                if let Some(h) = &mut self.game.companions.homunculus {
                    h.intimacy = data as i16;
                }
            }
            HOM_STATE_HUNGRY => {
                if let Some(h) = &mut self.game.companions.homunculus {
                    h.hunger = data as i16;
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_homun_property(&mut self, p: HomunculusProperty) {
        let h = self
            .game
            .companions
            .homunculus
            .get_or_insert_with(|| HomunculusState::new(0));
        h.name = p.name;
        h.renamed = p.renamed;
        h.level = p.level;
        h.hunger = p.hunger;
        h.intimacy = p.intimacy;
        h.accessory = p.accessory;
        h.atk = p.atk;
        h.matk = p.matk;
        h.hit = p.hit;
        h.critical = p.critical;
        h.def = p.def;
        h.mdef = p.mdef;
        h.flee = p.flee;
        h.aspd = p.aspd;
        h.hp = p.hp;
        h.max_hp = p.max_hp;
        h.sp = p.sp;
        h.max_sp = p.max_sp;
        h.exp = p.exp;
        h.max_exp = p.max_exp;
        h.skill_points = p.skill_points;
        h.atk_range = p.atk_range;
        h.vaporized = p.vaporized;
        if p.vaporized {
            self.windows.homunculus_window.set_visible(false);
            self.windows.homun_skill_window.set_visible(false);
        }
    }

    pub(super) fn handle_homun_feed_result(&mut self, success: bool, item_id: u16) {
        let msg = if success {
            "You fed your homunculus.".to_string()
        } else {
            format!("Failed to feed homunculus (item {item_id}).")
        };
        self.windows.chat_window.add_system(msg);
    }

    pub(super) fn handle_mercenary_info(&mut self, info: MercenaryInfo, is_init: bool) {
        if is_init {
            let m = self
                .game
                .companions
                .mercenary
                .get_or_insert_with(|| MercenaryState::new(info.gid));
            m.gid = info.gid;
            m.name = info.name;
            m.level = info.level;
            m.atk = info.atk;
            m.matk = info.matk;
            m.hit = info.hit;
            m.critical = info.critical;
            m.def = info.def;
            m.mdef = info.mdef;
            m.flee = info.flee;
            m.aspd = info.aspd;
            m.atk_range = info.atk_range;
            m.hp = info.hp;
            m.max_hp = info.max_hp;
            m.sp = info.sp;
            m.max_sp = info.max_sp;
            m.expire_date = info.expire_date;
            m.faith = info.faith;
            m.calls = info.calls;
            m.kills = info.kills;
            self.windows.mercenary_window.set_visible(true);
        } else if let Some(m) = &mut self.game.companions.mercenary {
            // Property update (no GID / attack range): stats only.
            m.name = info.name;
            m.level = info.level;
            m.atk = info.atk;
            m.matk = info.matk;
            m.hit = info.hit;
            m.critical = info.critical;
            m.def = info.def;
            m.mdef = info.mdef;
            m.flee = info.flee;
            m.aspd = info.aspd;
            m.hp = info.hp;
            m.max_hp = info.max_hp;
            m.sp = info.sp;
            m.max_sp = info.max_sp;
            m.expire_date = info.expire_date;
            m.faith = info.faith;
            m.calls = info.calls;
            m.kills = info.kills;
        }
    }

    pub(super) fn handle_mercenary_param_changed(&mut self, var: u16, value: i32) {
        use models::enums::EnumWithNumberValue;
        use models::enums::status::StatusTypes;
        let Some(m) = &mut self.game.companions.mercenary else {
            return;
        };
        let Ok(status) = StatusTypes::try_from_value(var as usize) else {
            return;
        };
        match status {
            StatusTypes::Hp => m.hp = value as u32,
            StatusTypes::Maxhp => m.max_hp = value as u32,
            StatusTypes::Sp => m.sp = value as u32,
            StatusTypes::Maxsp => m.max_sp = value as u32,
            StatusTypes::Merckills => m.kills = value,
            StatusTypes::Mercfaith => m.faith = value as i16,
            _ => {}
        }
    }

    pub(super) fn handle_homun_param_changed(&mut self, var: u16, value: i32) {
        use models::enums::EnumWithNumberValue;
        use models::enums::status::StatusTypes;
        let Some(h) = &mut self.game.companions.homunculus else {
            return;
        };
        let Ok(status) = StatusTypes::try_from_value(var as usize) else {
            return;
        };
        match status {
            StatusTypes::Hp => h.hp = value.max(0) as u32,
            StatusTypes::Maxhp => h.max_hp = value.max(0) as u32,
            StatusTypes::Sp => h.sp = value.max(0) as u32,
            StatusTypes::Maxsp => h.max_sp = value.max(0) as u32,
            StatusTypes::Baseexp => h.exp = value,
            _ => {}
        }
    }

    pub(super) fn handle_homun_skill_list(&mut self, skills: Vec<SkillInfo>) {
        let icon_paths: Vec<String> = skills.iter().map(|s| s.icon_path()).collect();
        if let Some(h) = &mut self.game.companions.homunculus {
            h.skills = skills;
        }
        self.preload_item_icons(icon_paths);
    }

    pub(super) fn handle_homun_skill_update(
        &mut self,
        id: u16,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        upgradable: bool,
    ) {
        if let Some(h) = &mut self.game.companions.homunculus {
            update_skill(&mut h.skills, id, level, sp_cost, attack_range, upgradable);
        }
    }

    pub(super) fn handle_mercenary_skill_list(&mut self, skills: Vec<SkillInfo>) {
        let icon_paths: Vec<String> = skills.iter().map(|s| s.icon_path()).collect();
        if let Some(m) = &mut self.game.companions.mercenary {
            m.skills = skills;
        }
        self.preload_item_icons(icon_paths);
    }

    pub(super) fn handle_mercenary_skill_update(
        &mut self,
        id: u16,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        upgradable: bool,
    ) {
        if let Some(m) = &mut self.game.companions.mercenary {
            update_skill(&mut m.skills, id, level, sp_cost, attack_range, upgradable);
        }
    }
}

fn update_skill(
    skills: &mut Vec<SkillInfo>,
    id: u16,
    level: i16,
    sp_cost: i16,
    attack_range: i16,
    upgradable: bool,
) {
    if let Some(s) = skills.iter_mut().find(|s| s.id == id) {
        s.level = level;
        s.sp_cost = sp_cost;
        s.attack_range = attack_range;
        s.upgradable = upgradable;
    }
}

impl App {
    pub(super) fn handle_request_companion_use_skill(
        &mut self,
        is_mercenary: bool,
        skill_id: u16,
        level: i16,
    ) {
        let companion = if is_mercenary {
            self.game
                .companions
                .mercenary
                .as_ref()
                .map(|m| (m.gid, &m.skills))
        } else {
            self.game
                .companions
                .homunculus
                .as_ref()
                .map(|h| (h.gid, &h.skills))
        };
        let Some((gid, skills)) = companion else {
            tracing::info!("RequestCompanionUseSkill: no companion present — dropped");
            return;
        };
        let target_type = skills
            .iter()
            .find(|s| s.id == skill_id)
            .map(|s| s.skill_target_type)
            .unwrap_or(SkillTargetType::Target);
        tracing::info!(
            "RequestCompanionUseSkill: merc={is_mercenary} skill={skill_id} gid={gid} target_type={target_type:?}"
        );
        match target_type {
            SkillTargetType::Target | SkillTargetType::Friend => {
                self.game.pending_casts.pending_companion_skill = Some(PendingCompanionSkill {
                    is_mercenary,
                    skill_id,
                    level,
                    target: CompanionSkillTarget::Entity,
                });
            }
            SkillTargetType::Ground => {
                self.game.pending_casts.pending_companion_skill = Some(PendingCompanionSkill {
                    is_mercenary,
                    skill_id,
                    level,
                    target: CompanionSkillTarget::Ground,
                });
            }
            SkillTargetType::Trap => {
                self.game.pending_casts.pending_companion_skill = Some(PendingCompanionSkill {
                    is_mercenary,
                    skill_id,
                    level,
                    target: CompanionSkillTarget::SkillUnit,
                });
            }
            _ => {
                self.push_owner_command_to(
                    is_mercenary,
                    ragnarok_game::companion::OwnerCommand::skill_object(
                        skill_id,
                        level as u8,
                        gid,
                    ),
                    self.input.shift_pressed,
                );
            }
        }
    }
}
