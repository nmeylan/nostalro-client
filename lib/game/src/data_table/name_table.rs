use std::collections::HashMap;

use ragnarok_formats::builtin_name_table::BUILTIN_NAME_TABLE;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

pub struct NameTable {
    entries: HashMap<u16, String>,
}

const IDENTITY_PATHS: &[&str] = &[
    "data/luafiles514/lua files/datainfo/jobidentity.lub",
    "data/luafiles514/lua files/datainfo/npcidentity.lub",
    "data/lua files/datainfo/jobidentity.lub",
    "data/lua files/datainfo/npcidentity.lub",
    "data/lua files/datainfo/jobidentity.lua",
    "data/lua files/datainfo/npcidentity.lua",
];

impl NameTable {
    pub fn load(grf: &GrfArchive) -> Self {
        let mut entries = HashMap::new();

        for path in IDENTITY_PATHS {
            if let Ok(data) = grf.read_file(path) {
                // Skip compiled Lua bytecode (starts with \x1bLua)
                if data.starts_with(b"\x1bLua") {
                    continue;
                }
                tracing::info!("Loading lua file {}", path);
                let content = lua_table::decode_euc_kr(&data);
                let assignments = parse_jt_assignments(&content);
                entries.extend(assignments);
            }
        }

        if !entries.is_empty() {
            tracing::info!("Loaded name table from GRF: {} entries", entries.len());
            return Self { entries };
        }

        tracing::info!(
            "No identity lua in GRF, using builtin name table ({} entries)",
            BUILTIN_NAME_TABLE.len()
        );
        let entries = BUILTIN_NAME_TABLE
            .iter()
            .map(|&(id, name)| (id, name.to_string()))
            .collect();
        Self { entries }
    }

    pub fn get_name(&self, job_id: u16) -> Option<&str> {
        self.entries.get(&job_id).map(|s| s.as_str())
    }
}

fn parse_jt_assignments(content: &str) -> HashMap<u16, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("--")
            || line.is_empty()
            || line.starts_with("{")
            || line.starts_with("}")
        {
            continue;
        }
        if let Some((name_part, value_part)) = line.split_once('=') {
            let name = name_part
                .trim()
                .trim_end_matches(']')
                .trim_start_matches('[')
                .trim();
            let value_str = value_part.trim().trim_end_matches(',').trim();
            if let Ok(id) = value_str.parse::<u16>() {
                let sprite_name = name.strip_prefix("JT_").unwrap_or(name);
                if !sprite_name.is_empty() {
                    map.insert(id, sprite_name.to_string());
                }
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_jt_assignments_extracts_names() {
        let content = "JT_PORING = 1002,\nJT_FABRE = 1007,\n";
        let map = parse_jt_assignments(content);
        assert_eq!(map.get(&1002).unwrap(), "PORING");
        assert_eq!(map.get(&1007).unwrap(), "FABRE");
    }

    #[test]
    fn builtin_table_has_common_entries() {
        let table = NameTable {
            entries: BUILTIN_NAME_TABLE
                .iter()
                .map(|&(id, name)| (id, name.to_string()))
                .collect(),
        };
        assert_eq!(table.get_name(1002), Some("Poring"));
        assert_eq!(table.get_name(46), Some("1_ETC_01"));
        assert_eq!(table.get_name(1885), Some("GOPINICH"));
        assert_eq!(table.get_name(566), Some("MYSTCASE"));
        assert!(table.get_name(60000).is_none());
    }
}
