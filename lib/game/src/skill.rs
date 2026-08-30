use std::collections::HashMap;

pub use models::enums::skill::SkillTargetType;
pub use models::enums::skill_enums::SkillEnum;

/// `msgstringtable` id for a skill-use failure `cause` (`USESKILL_FAIL_*`), or
/// `None` when no message should be shown. Cause 0 reads the skill and, for
/// Basic Skill, the `btype` sub-code to pick which requirement was not met.
///
/// Cause 71 is not here: its entry is a template needing the item name and
/// amount, so the caller formats it.
pub fn skill_failure_msg_id(cause: u8, skill: SkillEnum, btype: u16) -> Option<u16> {
    Some(match cause {
        0 => match skill {
            SkillEnum::NvBasic => match btype {
                0 => 159,
                1 => 160,
                2 => 161,
                3 => 162,
                4 => 163,
                5 => 164,
                6 => 165,
                7 => 383,
                8 => 1304,
                _ => return None,
            },
            SkillEnum::AlWarp => 214,
            SkillEnum::TfSteal => 205,
            SkillEnum::TfPoison => 207,
            _ => 204,
        },
        1 => 202,
        2 => 203,
        3 => 808,
        4 => 219,
        5 => 233,
        6 => 239,
        7 => 246,
        8 => 247,
        9 => 580,
        10 => 285,
        11..=16 => 1396 + (cause as u16 - 11),
        17..=23 => 1411 + (cause as u16 - 17),
        24..=27 => 1425 + (cause as u16 - 24),
        34 => 1436,
        84 => 2466,
        _ => return None,
    })
}

pub const USESKILL_FAIL_NEED_ITEM: u8 = 71;

/// Cause 71: `"[%s] required '%d' amount."`, filled with the item name and the
/// amount the skill needs.
pub const MSI_USESKILL_FAIL_NEED_ITEM: u16 = 1536;

/// Ground skills whose cast carries a written message: the client collects the text
/// itself and sends it with the placement, so the server has nothing to prompt for.
pub fn skill_needs_talkbox(skill: SkillEnum) -> bool {
    matches!(skill, SkillEnum::HtTalkiebox | SkillEnum::RgGraffiti)
}

/// The skill window / hotkey icon for `skill`, named after its internal id.
pub fn skill_icon_path(skill: SkillEnum) -> String {
    ragnarok_resources::ui::item::icon(&skill.to_name().to_lowercase())
}

pub const TALKBOX_MESSAGE_MAX_LEN: usize = 79;

/// The destination to answer a Teleport warp list with, when the list is the
/// single `Random` entry a level 1 cast produces. A level 2 cast adds the save
/// point, and any longer list has a real choice in it, so both return `None`.
pub fn teleport_lvl1_destination<'a>(
    skill: SkillEnum,
    destinations: &'a [String],
) -> Option<&'a str> {
    if skill != SkillEnum::AlTeleport {
        return None;
    }
    let [only] = destinations else {
        return None;
    };
    let name = only.strip_suffix(".gat").unwrap_or(only);
    name.eq_ignore_ascii_case("random").then_some(only.as_str())
}

pub struct SkillData {
    pub skill: SkillEnum,
    pub level: i16,
    pub selected_level: i16,
    pub sp_cost: i16,
    pub attack_range: i16,
    pub upgradable: bool,
    pub skill_target_type: SkillTargetType,
}

impl SkillData {
    pub fn icon_path(&self) -> String {
        skill_icon_path(self.skill)
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
#[derive(Debug, Clone, PartialEq)]
pub struct ItemSkill {
    pub level: i16,
    pub sp_cost: i16,
    pub attack_range: i16,
    pub skill_target_type: SkillTargetType,
}

#[derive(Default)]
pub struct ItemSkills {
    skills: HashMap<u32, ItemSkill>,
}

impl ItemSkills {
    pub fn insert(&mut self, skill: SkillEnum, granted: ItemSkill) {
        self.skills.insert(skill.id(), granted);
    }

    pub fn get(&self, skill: SkillEnum) -> Option<&ItemSkill> {
        self.skills.get(&skill.id())
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
        id: SkillEnum,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        upgradable: bool,
    ) {
        if let Some(skill) = self.skills.iter_mut().find(|s| s.skill == id) {
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
        if let Some(existing) = self.skills.iter_mut().find(|s| s.skill == skill.skill) {
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
                skill: s.skill,
                selected_level: s.level,
                level: s.level,
                sp_cost: s.sp_cost,
                attack_range: s.attack_range,
                upgradable: s.upgradable,
                skill_target_type: s.skill_target_type,
            })
            .collect();
        self.skills.iter().map(|s| s.icon_path()).collect()
    }

    pub fn apply_skill_added(&mut self, skill: crate::event::SkillInfo) -> String {
        let icon_path = skill_icon_path(skill.skill);
        self.add_skill(SkillData {
            skill: skill.skill,
            selected_level: skill.level,
            level: skill.level,
            sp_cost: skill.sp_cost,
            attack_range: skill.attack_range,
            upgradable: skill.upgradable,
            skill_target_type: skill.skill_target_type,
        });
        icon_path
    }

    pub fn get_skill(&self, id: SkillEnum) -> Option<&SkillData> {
        self.skills.iter().find(|s| s.skill == id)
    }

    pub fn get_skill_mut(&mut self, id: SkillEnum) -> Option<&mut SkillData> {
        self.skills.iter_mut().find(|s| s.skill == id)
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
        let teleport = SkillEnum::AlTeleport;
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
            teleport_lvl1_destination(SkillEnum::AlWarp, &random()),
            None
        );
    }

    fn make_skill(skill: SkillEnum, level: i16) -> SkillData {
        SkillData {
            skill,
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
            make_skill(SkillEnum::SmSword, 10),
            make_skill(SkillEnum::SmBash, 5),
        ]);
        assert_eq!(list.skills().len(), 2);
        assert_eq!(list.get_skill(SkillEnum::SmBash).unwrap().level, 5);
        assert!(list.get_skill(SkillEnum::AlHeal).is_none());
    }

    #[test]
    fn update_skill_modifies_existing() {
        let mut list = SkillList::new();
        list.set_skills(vec![make_skill(SkillEnum::SmBash, 5)]);
        list.update_skill(SkillEnum::SmBash, 6, 15, 1, true);
        let skill = list.get_skill(SkillEnum::SmBash).unwrap();
        assert_eq!(skill.level, 6);
        assert_eq!(skill.sp_cost, 15);
    }

    #[test]
    fn add_skill_inserts_or_updates() {
        let mut list = SkillList::new();
        list.add_skill(make_skill(SkillEnum::SmBash, 5));
        assert_eq!(list.skills().len(), 1);
        list.add_skill(make_skill(SkillEnum::SmBash, 7));
        assert_eq!(list.skills().len(), 1);
        assert_eq!(list.get_skill(SkillEnum::SmBash).unwrap().level, 7);
        list.add_skill(make_skill(SkillEnum::SmMagnum, 3));
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
    fn skill_failure_msg_id_maps_every_reachable_cause() {
        let basic = SkillEnum::NvBasic;
        assert_eq!(skill_failure_msg_id(0, basic, 3), Some(162));
        assert_eq!(skill_failure_msg_id(0, basic, 7), Some(383));
        assert_eq!(skill_failure_msg_id(0, basic, 9), None);
        assert_eq!(skill_failure_msg_id(0, SkillEnum::AlWarp, 0), Some(214));
        assert_eq!(skill_failure_msg_id(0, SkillEnum::TfSteal, 0), Some(205));
        assert_eq!(skill_failure_msg_id(0, SkillEnum::TfPoison, 0), Some(207));
        assert_eq!(skill_failure_msg_id(0, SkillEnum::SmBash, 0), Some(204));

        assert_eq!(skill_failure_msg_id(1, SkillEnum::SmBash, 0), Some(202));
        assert_eq!(skill_failure_msg_id(6, SkillEnum::AcDouble, 0), Some(239));
        assert_eq!(skill_failure_msg_id(9, SkillEnum::SmBash, 0), Some(580));
        assert_eq!(skill_failure_msg_id(10, SkillEnum::SmBash, 0), Some(285));

        assert_eq!(skill_failure_msg_id(11, SkillEnum::SmBash, 0), Some(1396));
        assert_eq!(skill_failure_msg_id(16, SkillEnum::SmBash, 0), Some(1401));
        assert_eq!(skill_failure_msg_id(17, SkillEnum::SmBash, 0), Some(1411));
        assert_eq!(skill_failure_msg_id(23, SkillEnum::SmBash, 0), Some(1417));
        assert_eq!(skill_failure_msg_id(24, SkillEnum::SmBash, 0), Some(1425));
        assert_eq!(skill_failure_msg_id(27, SkillEnum::SmBash, 0), Some(1428));

        assert_eq!(skill_failure_msg_id(34, SkillEnum::SmBash, 0), Some(1436));
        assert_eq!(skill_failure_msg_id(84, SkillEnum::SmBash, 0), Some(2466));

        assert_eq!(skill_failure_msg_id(30, SkillEnum::SmBash, 0), None);
        assert_eq!(skill_failure_msg_id(200, SkillEnum::SmBash, 0), None);
    }

    #[test]
    fn selected_level_defaults_to_learned() {
        let skill = make_skill(SkillEnum::SmBash, 10);
        assert_eq!(skill.use_level(), 10);
    }

    #[test]
    fn use_level_zero_for_unlearned() {
        let skill = make_skill(SkillEnum::SmBash, 0);
        assert_eq!(skill.use_level(), 0);
    }

    #[test]
    fn decrement_and_increment_use_level() {
        let mut skill = make_skill(SkillEnum::SmBash, 5);
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
        let mut skill = make_skill(SkillEnum::SmBash, 10);
        skill.selected_level = 8;
        list.set_skills(vec![skill]);
        list.update_skill(SkillEnum::SmBash, 5, 15, 1, true);
        assert_eq!(list.get_skill(SkillEnum::SmBash).unwrap().selected_level, 5);
    }

    #[test]
    fn add_skill_clamps_selected_level_on_update() {
        let mut list = SkillList::new();
        let mut skill = make_skill(SkillEnum::SmBash, 10);
        skill.selected_level = 8;
        list.set_skills(vec![skill]);
        list.add_skill(make_skill(SkillEnum::SmBash, 3));
        assert_eq!(list.get_skill(SkillEnum::SmBash).unwrap().selected_level, 3);
    }
}
