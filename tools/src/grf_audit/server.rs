//! Reads the map list out of a rathena checkout, so the audit can be told which
//! maps the server is actually able to send the client to.

use std::path::Path;

pub struct ServerData {
    pub maps: Vec<String>,
    /// Where the list came from, for the report header.
    pub source: String,
}

/// Candidate locations for the map index, newest layout first.
const MAP_INDEX: &[&str] = &["db/map_index.txt", "db/map_cache.dat", "conf/maps.conf"];

impl ServerData {
    /// Accepts either a rathena checkout or a plain list of map names, one per
    /// line. rathena ships the full official index, which narrows almost
    /// nothing, so the hand-written list is what actually shrinks an archive.
    pub fn load(path: &Path) -> Result<Self, String> {
        let index = if path.is_file() {
            path.to_path_buf()
        } else {
            MAP_INDEX
                .iter()
                .map(|rel| path.join(rel))
                .find(|p| p.is_file())
                .ok_or_else(|| {
                    format!(
                        "no map index under {} (looked for {})",
                        path.display(),
                        MAP_INDEX.join(", ")
                    )
                })?
        };

        let raw = std::fs::read(&index).map_err(|e| format!("{}: {e}", index.display()))?;
        let text = String::from_utf8_lossy(&raw);
        let maps = parse_map_index(&text);
        if maps.is_empty() {
            return Err(format!("{} listed no maps", index.display()));
        }
        Ok(Self {
            maps,
            source: index.display().to_string(),
        })
    }
}

/// `map_index.txt` is one map per line, optionally followed by an index number.
/// Comments start with `//`.
fn parse_map_index(text: &str) -> Vec<String> {
    let mut maps: Vec<String> = text
        .lines()
        .map(|l| l.split("//").next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .map(|m| m.to_lowercase())
        .filter(|m| !m.is_empty())
        .collect();
    maps.sort();
    maps.dedup();
    maps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_index_drops_comments_indices_and_duplicates() {
        let maps = parse_map_index(
            "// rAthena map index\nprontera\t1\ngeffen 2\n\n  payon  \n// comment\nprontera\n",
        );
        assert_eq!(maps, vec!["geffen", "payon", "prontera"]);
    }
}
