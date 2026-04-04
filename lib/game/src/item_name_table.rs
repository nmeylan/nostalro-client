use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

pub struct ItemNameTable {
    entries: HashMap<u16, String>,
}

const DISPLAY_NAME_PATHS: &[&str] = &[
    "data/idnum2itemdisplaynametable.txt",
    "data/num2itemdisplaynametable.txt",
];

impl ItemNameTable {
    pub fn load(grf: &GrfArchive) -> Self {
        for path in DISPLAY_NAME_PATHS {
            if let Ok(data) = grf.read_file(path) {
                let entries = lua_table::parse_item_name_table(&data);
                if !entries.is_empty() {
                    tracing::info!("Loaded item name table from GRF: {} entries", entries.len());
                    return Self { entries };
                }
            }
        }

        tracing::warn!("No item name table found in GRF");
        Self { entries: HashMap::new() }
    }

    pub fn get_name(&self, item_id: u16) -> Option<&str> {
        self.entries.get(&item_id).map(|s| s.as_str())
    }

    pub fn get_name_or_id(&self, item_id: u16) -> String {
        self.entries.get(&item_id)
            .cloned()
            .unwrap_or_else(|| format!("Item #{item_id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_name_or_id_fallback() {
        let table = ItemNameTable { entries: HashMap::new() };
        assert_eq!(table.get_name_or_id(501), "Item #501");
        assert!(table.get_name(501).is_none());
    }

    #[test]
    fn get_name_returns_entry() {
        let mut entries = HashMap::new();
        entries.insert(501, "Red Potion".to_string());
        let table = ItemNameTable { entries };
        assert_eq!(table.get_name(501), Some("Red Potion"));
        assert_eq!(table.get_name_or_id(501), "Red Potion");
    }
}
