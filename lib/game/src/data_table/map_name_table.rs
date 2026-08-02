use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table::decode_euc_kr;

const MAP_NAME_PATH: &str = ragnarok_resources::table::MAP_NAME;

/// Display name per map, for the map-name labels.
#[derive(Default)]
pub struct MapNameTable {
    entries: HashMap<String, String>,
}

impl MapNameTable {
    pub fn parse(data: &[u8]) -> Self {
        let text = decode_euc_kr(data);
        let mut entries = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let mut fields = line.split('#');
            let Some(name) = fields.next().filter(|n| !n.is_empty()) else {
                continue;
            };
            let Some(display) = fields.next().map(str::trim).filter(|d| !d.is_empty()) else {
                continue;
            };
            entries.insert(crate::map_key(name), display.to_string());
        }
        Self { entries }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let table = grf
            .read_file(MAP_NAME_PATH)
            .map(|data| Self::parse(&data))
            .unwrap_or_default();
        tracing::info!("Loaded map name table: {} entries", table.entries.len());
        table
    }

    pub fn display_name(&self, map: &str) -> Option<&str> {
        self.entries.get(&crate::map_key(map)).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_display_names_and_ignores_comments() {
        let table = MapNameTable::parse(
            concat!(
                "// 2018 Halloween\n",
                "1@halo.rsw#Halloween Festival Hall#\n",
                "prontera.rsw#Prontera, Capital of Rune Midgard#\n",
            )
            .as_bytes(),
        );

        assert_eq!(
            table.display_name("prontera.gat"),
            Some("Prontera, Capital of Rune Midgard")
        );
        assert_eq!(
            table.display_name("1@halo"),
            Some("Halloween Festival Hall")
        );
        assert_eq!(table.display_name("prt_fild08"), None);
    }
}
