use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

const SKILL_NAME_PATH: &str = ragnarok_resources::table::SKILL_NAME;

pub struct SkillNameTable {
    entries: HashMap<String, String>,
}

/// Human-facing skill label for every UI surface (skill tree, hotkey tooltip,
/// cast bubble, companion skill lists, …).
///
/// Prefers the GRF `skillnametable` entry. Underscores in either the table
/// display string or the internal fallback id are turned into spaces so callers
/// never render raw ids like `MC_MAMMONITE`.
pub fn format_skill_display_name(
    internal_name: &str,
    table: Option<&SkillNameTable>,
) -> String {
    table
        .map(|t| t.get_display_name_or_internal(internal_name))
        .unwrap_or_else(|| normalize_skill_label(internal_name))
}

fn normalize_skill_label(name: &str) -> String {
    name.replace('_', " ")
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
            .map(|s| normalize_skill_label(s))
            .unwrap_or_else(|| normalize_skill_label(internal_name))
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
        entries.insert(
            "AC_CONCENTRATION".to_string(),
            "Improve_Concentration".to_string(),
        );
        let table = SkillNameTable::from_entries(entries);

        assert_eq!(table.get_display_name("SM_BASH"), Some("Bash"));
        assert_eq!(
            table.get_display_name_or_internal("AL_HEAL"),
            "Heal".to_string()
        );
        assert_eq!(
            table.get_display_name_or_internal("UNKNOWN_SKILL"),
            "UNKNOWN SKILL".to_string()
        );
        assert_eq!(
            table.get_display_name_or_internal("AC_CONCENTRATION"),
            "Improve Concentration".to_string()
        );
        assert!(table.get_display_name("MISSING").is_none());
    }

    #[test]
    fn format_skill_display_name_shared() {
        let mut entries = HashMap::new();
        entries.insert("MC_MAMMONITE".to_string(), "Mammonite".to_string());
        let table = SkillNameTable::from_entries(entries);

        assert_eq!(
            format_skill_display_name("MC_MAMMONITE", Some(&table)),
            "Mammonite"
        );
        assert_eq!(
            format_skill_display_name("MC_MAMMONITE", None),
            "MC MAMMONITE"
        );
        assert_eq!(
            format_skill_display_name("SM_BASH", Some(&table)),
            "SM BASH"
        );
    }
}