use std::collections::{HashMap, HashSet};

use models::enums::skill_enums::SkillEnum;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

const PATH: &str = ragnarok_resources::table::SKILL_SP_AMOUNT;

pub struct SkillUseLevelTable {
    sp_per_level: HashMap<String, Vec<i16>>,
    forced: HashSet<String>,
}

impl SkillUseLevelTable {
    pub fn from_entries(sp_per_level: HashMap<String, Vec<i16>>) -> Self {
        Self {
            sp_per_level,
            forced: HashSet::new(),
        }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let sp_per_level = grf
            .read_file(PATH)
            .map(|data| lua_table::parse_level_use_skill_sp_table(&data))
            .unwrap_or_default();

        tracing::info!(
            "Loaded skill use level table: {} entries",
            sp_per_level.len()
        );
        Self {
            sp_per_level,
            forced: HashSet::new(),
        }
    }

    /// Give `skill` a level picker even though the table does not list it. Its SP
    /// cost stays the flat one the server sent, since no per-level column exists.
    pub fn force_level_select(&mut self, skill: SkillEnum) {
        self.forced.insert(skill.to_name().to_string());
    }

    pub fn supports_level_select(&self, skill: SkillEnum) -> bool {
        let name = skill.to_name();
        self.sp_per_level.contains_key(name) || self.forced.contains(name)
    }

    pub fn sp_at_level(&self, skill: SkillEnum, level: i16) -> Option<i16> {
        self.sp_per_level
            .get(skill.to_name())
            .and_then(|v| v.get((level - 1).max(0) as usize))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_level_select_and_sp_lookup() {
        let mut entries = HashMap::new();
        entries.insert(
            "SM_BASH".to_string(),
            vec![8, 8, 8, 8, 8, 15, 15, 15, 15, 15],
        );
        let table = SkillUseLevelTable::from_entries(entries);

        assert!(table.supports_level_select(SkillEnum::SmBash));
        assert!(!table.supports_level_select(SkillEnum::SmSword));
        assert_eq!(table.sp_at_level(SkillEnum::SmBash, 1), Some(8));
        assert_eq!(table.sp_at_level(SkillEnum::SmBash, 6), Some(15));
        assert_eq!(table.sp_at_level(SkillEnum::SmBash, 11), None);
        assert_eq!(table.sp_at_level(SkillEnum::SmSword, 1), None);
    }

    #[test]
    fn forcing_a_skill_adds_level_select_without_a_per_level_sp_column() {
        let mut table = SkillUseLevelTable::from_entries(HashMap::new());
        assert!(!table.supports_level_select(SkillEnum::AlTeleport));

        table.force_level_select(SkillEnum::AlTeleport);

        assert!(table.supports_level_select(SkillEnum::AlTeleport));
        assert!(!table.supports_level_select(SkillEnum::AlWarp));
        assert_eq!(table.sp_at_level(SkillEnum::AlTeleport, 1), None);
    }
}
