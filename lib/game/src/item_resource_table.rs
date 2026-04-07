use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

pub struct ItemResourceTable {
    identified_entries: HashMap<u16, String>,
    unidentified_entries: HashMap<u16, String>,
}

const IDENTIFIED_PATH: &str = "data/idnum2itemresnametable.txt";
const UNIDENTIFIED_PATH: &str = "data/num2itemresnametable.txt";

impl ItemResourceTable {
    pub fn load(grf: &GrfArchive) -> Self {
        let identified_entries = grf.read_file(IDENTIFIED_PATH)
            .map(|data| lua_table::parse_item_name_table(&data))
            .unwrap_or_default();
        let unidentified_entries = grf.read_file(UNIDENTIFIED_PATH)
            .map(|data| lua_table::parse_item_name_table(&data))
            .unwrap_or_default();

        tracing::info!(
            "Loaded item resource tables from GRF: {} identified, {} unidentified",
            identified_entries.len(),
            unidentified_entries.len(),
        );

        Self { identified_entries, unidentified_entries }
    }

    pub fn get_resource_name(&self, item_id: u16) -> Option<&str> {
        self.identified_entries.get(&item_id).map(|s| s.as_str())
    }

    pub fn get_resource_name_for(&self, item_id: u16, is_identified: bool) -> Option<&str> {
        let entries = if is_identified { &self.identified_entries } else { &self.unidentified_entries };
        entries.get(&item_id).map(|s| s.as_str())
    }

    pub fn item_icon_path(&self, item_id: u16) -> Option<String> {
        self.get_resource_name(item_id)
            .map(|name| format!("data/texture/유저인터페이스/item/{name}.bmp"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_resource_name_returns_entry() {
        let mut entries = HashMap::new();
        entries.insert(501, "빨간포션".to_string());
        let table = ItemResourceTable { entries };
        assert_eq!(table.get_resource_name(501), Some("빨간포션"));
        assert!(table.get_resource_name(999).is_none());
    }

    #[test]
    fn item_icon_path_builds_grf_path() {
        let mut entries = HashMap::new();
        entries.insert(501, "빨간포션".to_string());
        let table = ItemResourceTable { entries };
        assert_eq!(
            table.item_icon_path(501).unwrap(),
            "data/texture/유저인터페이스/item/빨간포션.bmp"
        );
        assert!(table.item_icon_path(999).is_none());
    }
}
