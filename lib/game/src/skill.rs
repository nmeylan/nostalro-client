use std::collections::HashMap;

pub use models::enums::skill::SkillTargetType;
use models::enums::skill_enums::SkillEnum;

/// Player-facing message for a skill-use failure `cause` (`USESKILL_FAIL_*`), or
/// `None` when no message should be shown. Cause 0 (`USESKILL_FAIL_LEVEL`) is the
/// server's catch-all default — sent for placement failures like "trap too near"
/// — so the original game shows nothing for it, as we do for any unmapped cause.
pub fn skill_failure_message(cause: u8) -> Option<&'static str> {
    Some(match cause {
        1 => "Not enough SP",
        2 => "Not enough HP",
        3 => "Insufficient materials",
        4 => "Skill is on cooldown",
        5 => "Not enough Zeny",
        6 => "Cannot use with this weapon",
        7 => "Red Gemstone required",
        8 => "Blue Gemstone required",
        9 => "Overweight",
        10 => "Skill failed",
        11 => "Cannot use on this target",
        12 => "You cannot carry any more Ancilla",
        13 => "Holy Water required",
        14 => "Ancilla required",
        16 => "Need another skill first",
        17 => "Need a partner",
        18 => "You are facing the wrong direction",
        _ => return None,
    })
}

/// Ground skills whose cast carries a written message: the client collects the text
/// itself and sends it with the placement, so the server has nothing to prompt for.
pub fn skill_needs_talkbox(skill_id: u16) -> bool {
    skill_id == SkillEnum::HtTalkiebox.id() as u16 || skill_id == SkillEnum::RgGraffiti.id() as u16
}

pub const TALKBOX_MESSAGE_MAX_LEN: usize = 79;

/// The destination to answer a Teleport warp list with, when the list is the
/// single `Random` entry a level 1 cast produces. A level 2 cast adds the save
/// point, and any longer list has a real choice in it, so both return `None`.
pub fn teleport_lvl1_destination<'a>(skill_id: u16, destinations: &'a [String]) -> Option<&'a str> {
    if skill_id != SkillEnum::AlTeleport.id() as u16 {
        return None;
    }
    let [only] = destinations else {
        return None;
    };
    let name = only.strip_suffix(".gat").unwrap_or(only);
    name.eq_ignore_ascii_case("random").then_some(only.as_str())
}

pub struct SkillData {
    pub id: u16,
    pub name: String,
    pub level: i16,
    pub selected_level: i16,
    pub sp_cost: i16,
    pub attack_range: i16,
    pub upgradable: bool,
    pub skill_target_type: SkillTargetType,
}

impl SkillData {
    pub fn icon_path(&self) -> String {
        format!(
            "data/texture/유저인터페이스/item/{}.bmp",
            self.name.to_lowercase()
        )
    }

    pub fn use_level(&self) -> i16 {
        if self.level <= 0 {
            0
        } else {
            self.selected_level.clamp(1, self.level)
        }
    }

    pub fn decrement_use_level(&mut self) {
        if self.level > 0 {
            self.selected_level = (self.selected_level - 1).max(1);
        }
    }

    pub fn increment_use_level(&mut self) {
        if self.level > 0 {
            self.selected_level = (self.selected_level + 1).min(self.level);
        }
    }
}

/// Cast metadata for a skill granted by a consumable, learned from
/// ZC_AUTORUN_SKILL rather than from the character's skill list. Kept apart from
/// [`SkillList`] so it never shows up in the skill window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSkill {
    pub name: String,
    pub level: i16,
    pub sp_cost: i16,
    pub attack_range: i16,
    pub skill_target_type: SkillTargetType,
}

#[derive(Default)]
pub struct ItemSkills {
    skills: HashMap<u16, ItemSkill>,
}

impl ItemSkills {
    pub fn insert(&mut self, id: u16, skill: ItemSkill) {
        self.skills.insert(id, skill);
    }

    pub fn get(&self, id: u16) -> Option<&ItemSkill> {
        self.skills.get(&id)
    }

    pub fn clear(&mut self) {
        self.skills.clear();
    }
}

pub struct SkillList {
    skills: Vec<SkillData>,
    open: bool,
}

impl Default for SkillList {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillList {
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
            open: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn clear(&mut self) {
        self.skills.clear();
        self.open = false;
    }

    pub fn set_skills(&mut self, skills: Vec<SkillData>) {
        self.skills = skills;
    }

    pub fn update_skill(
        &mut self,
        id: u16,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        upgradable: bool,
    ) {
        if let Some(skill) = self.skills.iter_mut().find(|s| s.id == id) {
            skill.level = level;
            skill.sp_cost = sp_cost;
            skill.attack_range = attack_range;
            skill.upgradable = upgradable;
            if level <= 0 {
                skill.selected_level = 0;
            } else if skill.selected_level > level {
                skill.selected_level = level;
            } else if skill.selected_level < 1 {
                skill.selected_level = 1;
            }
        }
    }

    pub fn add_skill(&mut self, skill: SkillData) {
        if let Some(existing) = self.skills.iter_mut().find(|s| s.id == skill.id) {
            existing.level = skill.level;
            existing.sp_cost = skill.sp_cost;
            existing.attack_range = skill.attack_range;
            existing.upgradable = skill.upgradable;
            existing.skill_target_type = skill.skill_target_type;
            if skill.level <= 0 {
                existing.selected_level = 0;
            } else if existing.selected_level > skill.level {
                existing.selected_level = skill.level;
            } else if existing.selected_level < 1 {
                existing.selected_level = 1;
            }
        } else {
            self.skills.push(skill);
        }
    }

    pub fn apply_skill_list(&mut self, skills: Vec<crate::event::SkillInfo>) -> Vec<String> {
        self.skills = skills
            .into_iter()
            .map(|s| SkillData {
                id: s.id,
                selected_level: s.level,
                level: s.level,
                sp_cost: s.sp_cost,
                attack_range: s.attack_range,
                upgradable: s.upgradable,
                skill_target_type: s.skill_target_type,
                name: s.name,
            })
            .collect();
        self.skills.iter().map(|s| s.icon_path()).collect()
    }

    pub fn apply_skill_added(&mut self, skill: crate::event::SkillInfo) -> String {
        let icon_path = format!(
            "data/texture/유저인터페이스/item/{}.bmp",
            skill.name.to_lowercase()
        );
        self.add_skill(SkillData {
            id: skill.id,
            selected_level: skill.level,
            level: skill.level,
            sp_cost: skill.sp_cost,
            attack_range: skill.attack_range,
            upgradable: skill.upgradable,
            skill_target_type: skill.skill_target_type,
            name: skill.name,
        });
        icon_path
    }

    pub fn get_skill(&self, id: u16) -> Option<&SkillData> {
        self.skills.iter().find(|s| s.id == id)
    }

    pub fn get_skill_mut(&mut self, id: u16) -> Option<&mut SkillData> {
        self.skills.iter_mut().find(|s| s.id == id)
    }

    pub fn get_skill_by_name(&self, name: &str) -> Option<&SkillData> {
        self.skills.iter().find(|s| s.name == name)
    }

    pub fn skills(&self) -> &[SkillData] {
        &self.skills
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_lone_random_entry_on_teleport_is_answered_automatically() {
        let teleport = SkillEnum::AlTeleport.id() as u16;
        let random = || vec!["Random.gat".to_string()];

        assert_eq!(
            teleport_lvl1_destination(teleport, &random()),
            Some("Random.gat")
        );
        assert_eq!(
            teleport_lvl1_destination(
                teleport,
                &["Random.gat".to_string(), "prontera.gat".to_string()]
            ),
            None
        );
        assert_eq!(
            teleport_lvl1_destination(SkillEnum::AlWarp.id() as u16, &random()),
            None
        );
    }

    fn make_skill(id: u16, name: &str, level: i16) -> SkillData {
        SkillData {
            id,
            name: name.to_string(),
            level,
            selected_level: level,
            sp_cost: 10,
            attack_range: 1,
            upgradable: true,
            skill_target_type: SkillTargetType::Target,
        }
    }

    #[test]
    fn set_and_query_skills() {
        let mut list = SkillList::new();
        list.set_skills(vec![
            make_skill(1, "SM_SWORD", 10),
            make_skill(5, "SM_BASH", 5),
        ]);
        assert_eq!(list.skills().len(), 2);
        assert_eq!(list.get_skill(5).unwrap().level, 5);
        assert_eq!(list.get_skill_by_name("SM_SWORD").unwrap().id, 1);
        assert!(list.get_skill(99).is_none());
    }

    #[test]
    fn update_skill_modifies_existing() {
        let mut list = SkillList::new();
        list.set_skills(vec![make_skill(5, "SM_BASH", 5)]);
        list.update_skill(5, 6, 15, 1, true);
        let skill = list.get_skill(5).unwrap();
        assert_eq!(skill.level, 6);
        assert_eq!(skill.sp_cost, 15);
    }

    #[test]
    fn add_skill_inserts_or_updates() {
        let mut list = SkillList::new();
        list.add_skill(make_skill(5, "SM_BASH", 5));
        assert_eq!(list.skills().len(), 1);
        list.add_skill(make_skill(5, "SM_BASH", 7));
        assert_eq!(list.skills().len(), 1);
        assert_eq!(list.get_skill(5).unwrap().level, 7);
        list.add_skill(make_skill(10, "SM_MAGNUM", 3));
        assert_eq!(list.skills().len(), 2);
    }

    #[test]
    fn toggle_open_close() {
        let mut list = SkillList::new();
        assert!(!list.is_open());
        list.toggle();
        assert!(list.is_open());
        list.close();
        assert!(!list.is_open());
        list.open();
        assert!(list.is_open());
    }

    #[test]
    fn skill_failure_message_returns_known_causes() {
        assert_eq!(skill_failure_message(1), Some("Not enough SP"));
        assert_eq!(skill_failure_message(2), Some("Not enough HP"));
        assert_eq!(skill_failure_message(4), Some("Skill is on cooldown"));
        assert_eq!(skill_failure_message(7), Some("Red Gemstone required"));
        assert_eq!(skill_failure_message(16), Some("Need another skill first"));
        assert_eq!(skill_failure_message(17), Some("Need a partner"));
    }

    #[test]
    fn skill_failure_message_catch_all_and_unknown_causes_show_nothing() {
        assert_eq!(skill_failure_message(0), None);
        assert_eq!(skill_failure_message(200), None);
    }

    #[test]
    fn selected_level_defaults_to_learned() {
        let skill = make_skill(1, "SM_BASH", 10);
        assert_eq!(skill.use_level(), 10);
    }

    #[test]
    fn use_level_zero_for_unlearned() {
        let skill = make_skill(1, "SM_BASH", 0);
        assert_eq!(skill.use_level(), 0);
    }

    #[test]
    fn decrement_and_increment_use_level() {
        let mut skill = make_skill(1, "SM_BASH", 5);
        assert_eq!(skill.use_level(), 5);
        skill.decrement_use_level();
        assert_eq!(skill.use_level(), 4);
        skill.decrement_use_level();
        skill.decrement_use_level();
        skill.decrement_use_level();
        assert_eq!(skill.use_level(), 1);
        skill.decrement_use_level();
        assert_eq!(skill.use_level(), 1);
        skill.increment_use_level();
        assert_eq!(skill.use_level(), 2);
        skill.selected_level = 5;
        skill.increment_use_level();
        assert_eq!(skill.use_level(), 5);
    }

    #[test]
    fn update_skill_clamps_selected_level() {
        let mut list = SkillList::new();
        let mut skill = make_skill(5, "SM_BASH", 10);
        skill.selected_level = 8;
        list.set_skills(vec![skill]);
        list.update_skill(5, 5, 15, 1, true);
        assert_eq!(list.get_skill(5).unwrap().selected_level, 5);
    }

    #[test]
    fn add_skill_clamps_selected_level_on_update() {
        let mut list = SkillList::new();
        let mut skill = make_skill(5, "SM_BASH", 10);
        skill.selected_level = 8;
        list.set_skills(vec![skill]);
        list.add_skill(make_skill(5, "SM_BASH", 3));
        assert_eq!(list.get_skill(5).unwrap().selected_level, 3);
    }
}
