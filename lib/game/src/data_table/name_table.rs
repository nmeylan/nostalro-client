use std::collections::HashMap;

use ragnarok_formats::builtin_name_table::BUILTIN_NAME_TABLE;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;
use ragnarok_formats::lub;

pub struct NameTable {
    entries: HashMap<u16, String>,
}

const IDENTITY_PATHS: &[&str] = &[
    ragnarok_resources::lua::JOB_IDENTITY_514_LUB,
    ragnarok_resources::lua::NPC_IDENTITY_514_LUB,
    ragnarok_resources::lua::JOB_IDENTITY_LUB,
    ragnarok_resources::lua::NPC_IDENTITY_LUB,
    ragnarok_resources::lua::JOB_IDENTITY_LUA,
    ragnarok_resources::lua::NPC_IDENTITY_LUA,
];

const JOB_NAME_PATHS: &[&str] = &[
    ragnarok_resources::lua::JOB_NAME_514_LUB,
    ragnarok_resources::lua::JOB_NAME_LUB,
];

impl NameTable {
    /// The GRF's identity tables name what that client knows; the builtin table
    /// stays underneath so ids the client version predates keep a sprite name.
    pub fn load(grf: &GrfArchive) -> Self {
        let mut entries: HashMap<u16, String> = BUILTIN_NAME_TABLE
            .iter()
            .map(|&(id, name)| (id, name.to_string()))
            .collect();
        let mut from_grf = 0;

        for path in IDENTITY_PATHS {
            if let Ok(data) = grf.read_file(path) {
                tracing::info!("Loading lua file {}", path);
                let identities = if lub::is_compiled_chunk(&data) {
                    match lua_table::parse_jt_identity_lub(&data) {
                        Ok(identities) => identities
                            .into_iter()
                            .map(|(id, name)| (id, sprite_name(&name)))
                            .collect(),
                        Err(error) => {
                            tracing::warn!("Compiled lua {path} unreadable: {error}");
                            HashMap::new()
                        }
                    }
                } else {
                    parse_jt_assignments(&lua_table::decode_euc_kr(&data))
                };
                from_grf += identities.len();
                entries.extend(identities);
            }
        }

        let from_job_name = load_job_names(grf);
        let job_name_count = from_job_name.len();
        entries.extend(from_job_name);

        tracing::info!(
            "Loaded name table: {} entries, {from_grf} from GRF identity lua, {job_name_count} from jobname lua",
            entries.len()
        );
        Self { entries }
    }

    pub fn get_name(&self, job_id: u16) -> Option<&str> {
        self.entries.get(&job_id).map(|s| s.as_str())
    }
}

fn parse_jt_assignments(content: &str) -> HashMap<u16, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("--")
            || line.is_empty()
            || line.starts_with("{")
            || line.starts_with("}")
        {
            continue;
        }
        if let Some((name_part, value_part)) = line.split_once('=') {
            let name = name_part
                .trim()
                .trim_end_matches(']')
                .trim_start_matches('[')
                .trim();
            let value_str = value_part.trim().trim_end_matches(',').trim();
            if let Ok(id) = value_str.parse::<u16>() {
                let sprite_name = sprite_name(name);
                if !sprite_name.is_empty() {
                    map.insert(id, sprite_name);
                }
            }
        }
    }
    map
}

/// The identity tables name the `JT_` constant, which is not always the sprite
/// file name: every homunculus constant carries a `MER_` prefix the sprite does
/// not. `jobname.lub` holds the real mapping, keyed by those same constants, so
/// the identity chunks are loaded first to define `jobtbl`.
fn load_job_names(grf: &GrfArchive) -> HashMap<u16, String> {
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    for path in IDENTITY_PATHS.iter().chain(JOB_NAME_PATHS) {
        if let Ok(data) = grf.read_file(path)
            && lub::is_compiled_chunk(&data)
        {
            chunks.push(data);
        }
    }
    let borrowed: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    match lua_table::parse_job_name_lub(&borrowed) {
        Ok(names) => names,
        Err(error) => {
            tracing::warn!("jobname lua unreadable: {error}");
            HashMap::new()
        }
    }
}

fn sprite_name(identity: &str) -> String {
    identity.strip_prefix("JT_").unwrap_or(identity).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_jt_assignments_extracts_names() {
        let content = "JT_PORING = 1002,\nJT_FABRE = 1007,\n";
        let map = parse_jt_assignments(content);
        assert_eq!(map.get(&1002).unwrap(), "PORING");
        assert_eq!(map.get(&1007).unwrap(), "FABRE");
    }

    #[test]
    fn builtin_table_has_common_entries() {
        let table = NameTable {
            entries: BUILTIN_NAME_TABLE
                .iter()
                .map(|&(id, name)| (id, name.to_string()))
                .collect(),
        };
        assert_eq!(table.get_name(1002), Some("Poring"));
        assert_eq!(table.get_name(46), Some("1_ETC_01"));
        assert_eq!(table.get_name(1885), Some("GOPINICH"));
        assert_eq!(table.get_name(566), Some("MYSTCASE"));
        assert!(table.get_name(60000).is_none());
    }
}
