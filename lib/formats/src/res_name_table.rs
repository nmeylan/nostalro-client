use std::collections::HashMap;

use crate::lua_table::decode_euc_kr;

/// Bases the table's keys are relative to, longest first so `data/texture/`
/// wins over `data/`.
const BASES: &[&str] = &[
    ragnarok_resources::dir::TEXTURE,
    ragnarok_resources::dir::DATA,
];

/// `resnametable.txt`: `key#value#` pairs naming the resource to open when the
/// requested one is absent. `pvp_n_2-2` has no geometry of its own, so its
/// `.rsw`/`.gnd`/`.gat` and minimap image all redirect to `job_hunter`.
#[derive(Default)]
pub struct ResNameTable {
    entries: HashMap<String, String>,
}

impl ResNameTable {
    pub fn parse(data: &[u8]) -> Self {
        let text = decode_euc_kr(data);
        let mut entries = HashMap::new();
        let mut key: Option<String> = None;
        let mut token = String::new();
        for ch in text.chars() {
            match ch {
                '#' => match key.take() {
                    None => key = Some(std::mem::take(&mut token)),
                    Some(k) => {
                        let value = std::mem::take(&mut token);
                        if !k.is_empty() && !value.is_empty() {
                            entries.insert(normalize(&k), normalize(&value));
                        }
                    }
                },
                '\r' | '\n' => {
                    key = None;
                    token.clear();
                }
                _ => token.push(ch),
            }
        }
        Self { entries }
    }

    /// Keeps what is already here; a later archive only supplies pairs the
    /// earlier one lacks.
    pub fn extend_from(&mut self, other: Self) {
        for (key, value) in other.entries {
            self.entries.entry(key).or_insert(value);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn resolve(&self, path: &str) -> Option<String> {
        let normalized = normalize(path);
        BASES.iter().find_map(|base| {
            let target = self.entries.get(normalized.strip_prefix(base)?)?;
            Some(format!("{base}{target}"))
        })
    }
}

fn normalize(name: &str) -> String {
    name.to_lowercase().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirects_map_geometry_and_minimap_past_comments() {
        let (euc_kr, _, _) = encoding_rs::EUC_KR.encode(concat!(
            "//pvp\n",
            "pvp_n_2-2.rsw#job_hunter.rsw#\n",
            "pvp_n_2-2.gat#job_hunter.gat#\n",
            "유저인터페이스\\map\\pvp_n_2-2.bmp#유저인터페이스\\map\\job_hunter.bmp#\n",
        ));
        let table = ResNameTable::parse(&euc_kr);

        assert_eq!(
            table.resolve("data/pvp_n_2-2.rsw").as_deref(),
            Some("data/job_hunter.rsw")
        );
        assert_eq!(
            table.resolve("data\\PVP_N_2-2.gat").as_deref(),
            Some("data/job_hunter.gat")
        );
        assert_eq!(
            table
                .resolve("data/texture/유저인터페이스/map/pvp_n_2-2.bmp")
                .as_deref(),
            Some("data/texture/유저인터페이스/map/job_hunter.bmp")
        );
        assert_eq!(table.resolve("data/prontera.rsw"), None);
    }

    #[test]
    fn earlier_archive_wins_when_merging() {
        let mut table = ResNameTable::parse(b"pvp_n_1-1.rsw#kokotewa.rsw#\n");
        table.extend_from(ResNameTable::parse(
            b"pvp_n_1-1.rsw#prt_maze02.rsw#\npvp_n_2-1.rsw#prt_maze02.rsw#\n",
        ));

        assert_eq!(
            table.resolve("data/pvp_n_1-1.rsw").as_deref(),
            Some("data/kokotewa.rsw")
        );
        assert_eq!(
            table.resolve("data/pvp_n_2-1.rsw").as_deref(),
            Some("data/prt_maze02.rsw")
        );
    }
}
