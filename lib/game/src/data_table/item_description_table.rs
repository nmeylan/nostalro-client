use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

pub struct ItemDescriptionTable {
    identified: HashMap<u16, Vec<String>>,
    unidentified: HashMap<u16, Vec<String>>,
}

const IDENTIFIED_PATH: &str = ragnarok_resources::table::IDENTIFIED_ITEM_DESC;
const UNIDENTIFIED_PATH: &str = ragnarok_resources::table::UNIDENTIFIED_ITEM_DESC;

impl ItemDescriptionTable {
    pub fn from_entries(
        identified: HashMap<u16, Vec<String>>,
        unidentified: HashMap<u16, Vec<String>>,
    ) -> Self {
        Self {
            identified,
            unidentified,
        }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let identified = grf
            .read_file(IDENTIFIED_PATH)
            .map(|data| lua_table::parse_item_description_table(&data))
            .unwrap_or_default();
        let unidentified = grf
            .read_file(UNIDENTIFIED_PATH)
            .map(|data| lua_table::parse_item_description_table(&data))
            .unwrap_or_default();

        tracing::info!(
            "Loaded item description tables: {} identified, {} unidentified",
            identified.len(),
            unidentified.len(),
        );

        Self {
            identified,
            unidentified,
        }
    }

    pub fn get(&self, item_id: u16, is_identified: bool) -> Option<&[String]> {
        if is_identified {
            self.identified.get(&item_id).map(|v| v.as_slice())
        } else {
            self.unidentified
                .get(&item_id)
                .or_else(|| self.identified.get(&item_id))
                .map(|v| v.as_slice())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table() -> ItemDescriptionTable {
        let mut identified = HashMap::new();
        identified.insert(
            501,
            vec![
                "A red potion.".to_string(),
                "Class:^0000FF Restorative^000000".to_string(),
            ],
        );
        identified.insert(1201, vec!["A dagger.".to_string()]);
        let mut unidentified = HashMap::new();
        unidentified.insert(1201, vec!["An unknown weapon.".to_string()]);
        ItemDescriptionTable {
            identified,
            unidentified,
        }
    }

    #[test]
    fn get_identified_description() {
        let table = make_table();
        let desc = table.get(501, true).unwrap();
        assert_eq!(desc.len(), 2);
        assert_eq!(desc[0], "A red potion.");
    }

    #[test]
    fn get_unidentified_falls_back_to_identified() {
        let table = make_table();
        assert_eq!(table.get(1201, false).unwrap()[0], "An unknown weapon.");
        assert_eq!(table.get(501, false).unwrap()[0], "A red potion.");
    }

    #[test]
    fn get_missing_returns_none() {
        let table = make_table();
        assert!(table.get(999, true).is_none());
    }
}
