use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

pub struct ItemNameTable {
    identified_entries: HashMap<u16, String>,
    unidentified_entries: HashMap<u16, String>,
}

const IDENTIFIED_PATH: &str = "data/idnum2itemdisplaynametable.txt";
const UNIDENTIFIED_PATH: &str = "data/num2itemdisplaynametable.txt";

impl ItemNameTable {
    pub fn from_entries(
        identified_entries: HashMap<u16, String>,
        unidentified_entries: HashMap<u16, String>,
    ) -> Self {
        Self {
            identified_entries,
            unidentified_entries,
        }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let identified_entries = grf
            .read_file(IDENTIFIED_PATH)
            .map(|data| lua_table::parse_item_name_table(&data))
            .unwrap_or_default();
        let unidentified_entries = grf
            .read_file(UNIDENTIFIED_PATH)
            .map(|data| lua_table::parse_item_name_table(&data))
            .unwrap_or_default();

        tracing::info!(
            "Loaded item name tables from GRF: {} identified, {} unidentified",
            identified_entries.len(),
            unidentified_entries.len(),
        );
        if unidentified_entries.is_empty() {
            tracing::warn!(
                "{UNIDENTIFIED_PATH} is missing: unidentified items will show their real name"
            );
        }

        Self {
            identified_entries,
            unidentified_entries,
        }
    }

    pub fn get_name(&self, item_id: u16) -> Option<&str> {
        self.identified_entries.get(&item_id).map(|s| s.as_str())
    }

    pub fn get_name_or_id(&self, item_id: u16) -> String {
        self.identified_entries
            .get(&item_id)
            .cloned()
            .unwrap_or_else(|| format!("Item #{item_id}"))
    }

    pub fn get_name_or_id_for(&self, item_id: u16, is_identified: bool) -> String {
        if is_identified {
            self.identified_entries
                .get(&item_id)
                .cloned()
                .unwrap_or_else(|| format!("Item #{item_id}"))
        } else {
            self.unidentified_entries
                .get(&item_id)
                .or_else(|| self.identified_entries.get(&item_id))
                .cloned()
                .unwrap_or_else(|| format!("Item #{item_id}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table() -> ItemNameTable {
        let mut identified = HashMap::new();
        identified.insert(501, "Red Potion".to_string());
        identified.insert(1201, "Knife".to_string());
        let mut unidentified = HashMap::new();
        unidentified.insert(1201, "Unknown Weapon".to_string());
        ItemNameTable {
            identified_entries: identified,
            unidentified_entries: unidentified,
        }
    }

    #[test]
    fn get_name_returns_identified() {
        let table = make_table();
        assert_eq!(table.get_name(501), Some("Red Potion"));
        assert_eq!(table.get_name_or_id(501), "Red Potion");
        assert!(table.get_name(999).is_none());
        assert_eq!(table.get_name_or_id(999), "Item #999");
    }

    #[test]
    fn get_name_or_id_for_dispatches_by_identified() {
        let table = make_table();
        assert_eq!(table.get_name_or_id_for(1201, true), "Knife");
        assert_eq!(table.get_name_or_id_for(1201, false), "Unknown Weapon");
        assert_eq!(table.get_name_or_id_for(501, true), "Red Potion");
        assert_eq!(table.get_name_or_id_for(501, false), "Red Potion");
    }
}
