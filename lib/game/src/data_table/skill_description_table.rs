use std::collections::HashMap;

use models::enums::skill_enums::SkillEnum;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

const SKILL_DESC_PATH: &str = ragnarok_resources::table::SKILL_DESC;

pub struct SkillDescriptionTable {
    entries: HashMap<String, Vec<String>>,
}

impl SkillDescriptionTable {
    pub fn from_entries(entries: HashMap<String, Vec<String>>) -> Self {
        Self { entries }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let entries = grf
            .read_file(SKILL_DESC_PATH)
            .map(|data| lua_table::parse_skill_description_table(&data))
            .unwrap_or_default();

        tracing::info!("Loaded skill description table: {} entries", entries.len());
        Self { entries }
    }

    pub fn get_description(&self, skill: SkillEnum) -> Option<&[String]> {
        self.entries.get(skill.to_name()).map(|v| v.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_description() {
        let mut entries = HashMap::new();
        entries.insert(
            "SM_BASH".to_string(),
            vec![
                "Hits a single target.".to_string(),
                "Lv 1: ATK 130%".to_string(),
            ],
        );
        let table = SkillDescriptionTable::from_entries(entries);

        let desc = table.get_description(SkillEnum::SmBash).unwrap();
        assert_eq!(desc.len(), 2);
        assert_eq!(desc[0], "Hits a single target.");
        assert!(table.get_description(SkillEnum::AlHeal).is_none());
    }
}
