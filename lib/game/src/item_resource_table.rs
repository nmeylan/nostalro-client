use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

pub struct ItemResourceTable {
    entries: HashMap<u16, String>,
}

const RESOURCE_NAME_PATHS: &[&str] = &[
    "data/idnum2itemresnametable.txt",
    "data/num2itemresnametable.txt",
];

impl ItemResourceTable {
    pub fn load(grf: &GrfArchive) -> Self {
        for path in RESOURCE_NAME_PATHS {
            if let Ok(data) = grf.read_file(path) {
                let entries = lua_table::parse_item_name_table(&data);
                if !entries.is_empty() {
                    tracing::info!("Loaded item resource table from GRF: {} entries", entries.len());
                    return Self { entries };
                }
            }
        }

        tracing::warn!("No item resource table found in GRF");
        Self { entries: HashMap::new() }
    }

    pub fn get_resource_name(&self, item_id: u16) -> Option<&str> {
        self.entries.get(&item_id).map(|s| s.as_str())
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
