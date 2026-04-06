use std::collections::{HashMap, HashSet};

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

pub struct CardNameTable {
    prefix_names: HashMap<u16, String>,
    postfix_ids: HashSet<u16>,
}

const PREFIX_PATH: &str = "data/cardprefixnametable.txt";
const POSTFIX_PATH: &str = "data/cardpostfixnametable.txt";

impl CardNameTable {
    pub fn load(grf: &GrfArchive) -> Self {
        let prefix_names = grf
            .read_file(PREFIX_PATH)
            .map(|data| lua_table::parse_item_name_table(&data))
            .unwrap_or_default();
        let postfix_ids = grf
            .read_file(POSTFIX_PATH)
            .map(|data| lua_table::parse_id_set_table(&data))
            .unwrap_or_default();
        tracing::info!(
            "Loaded card name table: {} prefixes, {} postfixes",
            prefix_names.len(),
            postfix_ids.len()
        );
        Self { prefix_names, postfix_ids }
    }

    pub fn get_card_name(&self, card_id: u16) -> Option<&str> {
        self.prefix_names.get(&card_id).map(|s| s.as_str())
    }

    pub fn is_postfix(&self, card_id: u16) -> bool {
        self.postfix_ids.contains(&card_id)
    }

    #[cfg(test)]
    pub fn from_data(prefix_names: HashMap<u16, String>, postfix_ids: HashSet<u16>) -> Self {
        Self { prefix_names, postfix_ids }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_name_lookup_and_postfix_check() {
        let mut prefix_names = HashMap::new();
        prefix_names.insert(4001, "Bloody".to_string());
        prefix_names.insert(4002, "of Starlight".to_string());
        let mut postfix_ids = HashSet::new();
        postfix_ids.insert(4002);

        let table = CardNameTable::from_data(prefix_names, postfix_ids);

        assert_eq!(table.get_card_name(4001), Some("Bloody"));
        assert!(!table.is_postfix(4001));

        assert_eq!(table.get_card_name(4002), Some("of Starlight"));
        assert!(table.is_postfix(4002));

        assert_eq!(table.get_card_name(9999), None);
        assert!(!table.is_postfix(9999));
    }
}
