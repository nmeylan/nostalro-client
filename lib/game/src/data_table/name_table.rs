use std::collections::HashMap;

use ragnarok_formats::builtin_name_table::BUILTIN_NAME_TABLE;

pub struct NameTable {
    entries: HashMap<u16, String>,
}

impl NameTable {
    pub fn load() -> Self {
        Self {
            entries: BUILTIN_NAME_TABLE
                .iter()
                .map(|&(id, name)| (id, name.to_string()))
                .collect(),
        }
    }

    pub fn get_name(&self, job_id: u16) -> Option<&str> {
        self.entries.get(&job_id).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_table_has_common_entries() {
        let table = NameTable::load();
        assert_eq!(table.get_name(1002), Some("Poring"));
        assert_eq!(table.get_name(46), Some("1_ETC_01"));
        assert_eq!(table.get_name(1885), Some("GOPINICH"));
        assert_eq!(table.get_name(566), Some("MYSTCASE"));
        assert_eq!(table.get_name(6001), Some("LIF"));
        assert!(table.get_name(60000).is_none());
    }
}
