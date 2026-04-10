use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

const SKILL_NAME_PATH: &str = "data/skillnametable.txt";

pub struct SkillNameTable {
    entries: HashMap<String, String>,
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

    pub fn get_display_name(&self, internal_name: &str) -> Option<&str> {
        self.entries.get(internal_name).map(|s| s.as_str())
    }

    pub fn get_display_name_or_internal(&self, internal_name: &str) -> String {
        self.entries
            .get(internal_name)
            .cloned()
            .unwrap_or_else(|| internal_name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_display_name() {
        let mut entries = HashMap::new();
        entries.insert("SM_BASH".to_string(), "Bash".to_string());
        entries.insert("AL_HEAL".to_string(), "Heal".to_string());
        let table = SkillNameTable::from_entries(entries);

        assert_eq!(table.get_display_name("SM_BASH"), Some("Bash"));
        assert_eq!(table.get_display_name_or_internal("AL_HEAL"), "Heal".to_string());
        assert_eq!(
            table.get_display_name_or_internal("UNKNOWN_SKILL"),
            "UNKNOWN_SKILL".to_string()
        );
        assert!(table.get_display_name("MISSING").is_none());
    }
}
