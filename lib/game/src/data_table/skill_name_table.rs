use std::collections::HashMap;

use models::enums::skill_enums::SkillEnum;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

const SKILL_NAME_PATH: &str = ragnarok_resources::table::SKILL_NAME;

pub struct SkillNameTable {
    entries: HashMap<String, String>,
}

pub fn format_skill_display_name<'a>(
    skill: &'a SkillEnum,
    table: Option<&'a SkillNameTable>,
) -> &'a str {
    let internal = skill.to_name();
    match table {
        Some(t) => t.display_name_or_internal(internal),
        None => internal,
    }
}

impl SkillNameTable {
    pub fn from_entries(entries: HashMap<String, String>) -> Self {
        Self { entries }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let entries = grf
            .read_file(SKILL_NAME_PATH)
            .map(|data| lua_table::parse_skill_name_table(&data))
            .unwrap_or_default();

        tracing::info!("Loaded skill name table: {} entries", entries.len());
        Self { entries }
    }

    pub fn display_name_or_internal<'a>(&'a self, internal_name: &'a str) -> &'a str {
        self.entries
            .get(internal_name)
            .map(String::as_str)
            .unwrap_or(internal_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> SkillNameTable {
        let mut entries = HashMap::new();
        entries.insert("SM_BASH".to_string(), "Bash".to_string());
        entries.insert("MC_MAMMONITE".to_string(), "Mammonite".to_string());
        SkillNameTable::from_entries(entries)
    }

    #[test]
    fn display_name_falls_back_to_the_internal_id() {
        let table = table();

        assert_eq!(
            format_skill_display_name(&SkillEnum::McMammonite, Some(&table)),
            "Mammonite"
        );
        assert_eq!(
            format_skill_display_name(&SkillEnum::AlHeal, Some(&table)),
            "AL_HEAL"
        );
        assert_eq!(
            format_skill_display_name(&SkillEnum::SmBash, None),
            "SM_BASH"
        );
    }
}
