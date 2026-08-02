use std::collections::HashMap;
use std::path::Path;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

pub struct CardIllustrationTable {
    entries: HashMap<u16, String>,
}

const TABLE_PATH: &str = ragnarok_resources::table::CARD_ILLUSTRATION_NAME;

impl CardIllustrationTable {
    pub fn from_entries(entries: HashMap<u16, String>) -> Self {
        Self { entries }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let data = grf
            .read_file(TABLE_PATH)
            .ok()
            .or_else(|| std::fs::read(Path::new(TABLE_PATH)).ok());
        let Some(data) = data else {
            tracing::warn!(
                "Card illustration table not found in GRF or filesystem: {}",
                TABLE_PATH
            );
            return Self {
                entries: HashMap::new(),
            };
        };

        let entries = lua_table::parse_item_res_table(&data);
        tracing::info!("Loaded card illustration table: {} entries", entries.len());

        Self { entries }
    }

    pub fn get(&self, card_id: u16) -> Option<&str> {
        self.entries.get(&card_id).map(|s| s.as_str())
    }

    pub fn illustration_path(&self, card_id: u16) -> Option<String> {
        self.get(card_id)
            .map(|name| ragnarok_resources::ui::cardbmp::named(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_entry() {
        let mut entries = HashMap::new();
        entries.insert(4001, "카드일러스트1".to_string());
        entries.insert(4002, "카드일러스트2".to_string());
        let table = CardIllustrationTable::from_entries(entries);
        assert_eq!(table.get(4001), Some("카드일러스트1"));
        assert_eq!(table.get(4002), Some("카드일러스트2"));
        assert_eq!(table.get(9999), None);
    }

    #[test]
    fn illustration_path_constructs_correct_grf_path() {
        let mut entries = HashMap::new();
        entries.insert(4001, "카드일러스트1".to_string());
        let table = CardIllustrationTable::from_entries(entries);
        assert_eq!(
            table.illustration_path(4001),
            Some("data/texture/유저인터페이스/cardbmp/카드일러스트1.bmp".to_string()),
        );
        assert_eq!(table.illustration_path(9999), None);
    }
}
