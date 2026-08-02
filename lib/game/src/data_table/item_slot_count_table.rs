use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

pub struct ItemSlotCountTable {
    entries: HashMap<u16, u8>,
}

const SLOT_COUNT_PATH: &str = ragnarok_resources::table::ITEM_SLOT_COUNT;

impl ItemSlotCountTable {
    pub fn from_entries(entries: HashMap<u16, u8>) -> Self {
        Self { entries }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        if let Ok(data) = grf.read_file(SLOT_COUNT_PATH) {
            let raw = lua_table::parse_item_name_table(&data);
            let entries: HashMap<u16, u8> = raw
                .into_iter()
                .filter_map(|(id, val)| val.parse::<u8>().ok().map(|v| (id, v)))
                .collect();
            tracing::info!("Loaded item slot count table: {} entries", entries.len());
            return Self { entries };
        }
        tracing::warn!("No item slot count table found in GRF");
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get_slot_count(&self, item_id: u16) -> u8 {
        self.entries.get(&item_id).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_slot_count_returns_value_or_zero() {
        let mut entries = HashMap::new();
        entries.insert(1101, 3);
        entries.insert(1201, 2);
        let table = ItemSlotCountTable { entries };
        assert_eq!(table.get_slot_count(1101), 3);
        assert_eq!(table.get_slot_count(1201), 2);
        assert_eq!(table.get_slot_count(9999), 0);
    }
}
