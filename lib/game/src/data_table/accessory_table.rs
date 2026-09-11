use std::collections::HashMap;

use ragnarok_formats::builtin_accessory_table::BUILTIN_ACCESSORY_TABLE;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;
use ragnarok_formats::lub;

pub struct AccessoryTable {
    entries: HashMap<u16, String>,
}

const ACCESSORY_ID_PATHS: &[&str] = &[
    ragnarok_resources::lua::ACCESSORY_ID_514_LUB,
    ragnarok_resources::lua::ACCESSORY_ID_LUA,
    ragnarok_resources::lua::ACCESSORY_ID_LUB,
];

const ACCNAME_PATHS: &[&str] = &[
    ragnarok_resources::lua::ACCESSORY_NAME_514_LUB,
    ragnarok_resources::lua::ACCESSORY_NAME_LUA,
    ragnarok_resources::lua::ACCESSORY_NAME_LUB,
];

impl AccessoryTable {
    pub fn empty() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn load_from_grf(grf: &GrfArchive) -> Self {
        let id_data = ACCESSORY_ID_PATHS
            .iter()
            .find_map(|path| grf.read_file(path).ok());

        let name_data = ACCNAME_PATHS
            .iter()
            .find_map(|path| grf.read_file(path).ok());

        if let (Some(ids), Some(names)) = (id_data, name_data) {
            let table = if lub::is_compiled_chunk(&ids) || lub::is_compiled_chunk(&names) {
                lua_table::build_accessory_table_from_lub(&ids, &names).unwrap_or_else(|error| {
                    tracing::warn!("Compiled accessory lua unreadable: {error}");
                    HashMap::new()
                })
            } else {
                lua_table::build_accessory_table(
                    &lua_table::decode_euc_kr(&ids),
                    &lua_table::decode_euc_kr(&names),
                )
            };
            if !table.is_empty() {
                tracing::info!("Loaded accessory table from lua: {} entries", table.len());
                return Self { entries: table };
            }
        }

        tracing::info!(
            "No lua files in GRF, using built-in accessory table ({} entries)",
            BUILTIN_ACCESSORY_TABLE.len()
        );
        let entries = BUILTIN_ACCESSORY_TABLE
            .iter()
            .map(|&(id, suffix)| (id, suffix.to_string()))
            .collect();
        Self { entries }
    }

    pub fn get_suffix(&self, view_id: u16) -> Option<&str> {
        self.entries.get(&view_id).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn sorted_entries(&self) -> Vec<(u16, &str)> {
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .map(|(&id, s)| (id, s.as_str()))
            .collect();
        entries.sort_by_key(|(id, _)| *id);
        entries
    }

    pub fn next_id(&self, current: u16) -> u16 {
        let sorted = self.sorted_entries();
        if sorted.is_empty() {
            return 0;
        }
        if current == 0 {
            return sorted[0].0;
        }
        sorted
            .iter()
            .find(|(id, _)| *id > current)
            .map(|(id, _)| *id)
            .unwrap_or(0)
    }

    pub fn prev_id(&self, current: u16) -> u16 {
        let sorted = self.sorted_entries();
        if sorted.is_empty() {
            return 0;
        }
        if current == 0 {
            return sorted.last().map(|(id, _)| *id).unwrap_or(0);
        }
        sorted
            .iter()
            .rev()
            .find(|(id, _)| *id < current)
            .map(|(id, _)| *id)
            .unwrap_or(0)
    }
}
