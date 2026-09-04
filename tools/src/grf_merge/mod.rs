//! Flattens a priority-ordered GRF stack into one archive.
//!
//! Precedence matches `GrfArchive::open_layered`: the first archive declared
//! wins. That holds at both granularities — whole files, and the individual
//! records of the tables [`tables`] knows how to fold.

pub mod tables;

use ragnarok_formats::grf::GrfArchive;
use std::path::Path;

/// Folds every registry table each layer carries a copy of. A table only one
/// layer has needs no merge and is left out.
pub fn merge_tables(layers: &[GrfArchive]) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for (path, shape) in tables::MERGED {
        let copies: Vec<Vec<u8>> = layers
            .iter()
            .filter_map(|layer| layer.read_file(path).ok())
            .collect();
        if copies.len() < 2 {
            continue;
        }
        out.push((path.to_string(), tables::merge(*shape, &copies)));
    }
    out
}

/// Builds `out` from the entries `names` selects, read through `grf` so merged
/// tables land merged. Entries are recompressed rather than copied, so this is
/// the slow half of the tool.
///
/// Returns the entries no archive could produce bytes for. Recompressing means
/// an entry we cannot decompress cannot be carried over at all, and archives do
/// hold degenerate ones — `data/gpak.exe` in the stock `data.grf` claims a
/// one-byte deflate stream. Skipping them loses nothing the client could read.
pub fn write_merged(
    grf: &GrfArchive,
    names: &[String],
    out: &Path,
    progress: &mut dyn FnMut(usize),
) -> Result<Vec<String>, String> {
    if out.exists() {
        return Err(format!("{} already exists", out.display()));
    }
    let mut dest = GrfArchive::create(out).map_err(|e| format!("create {}: {e}", out.display()))?;
    let mut unreadable = Vec::new();

    for (written, name) in names.iter().enumerate() {
        match grf.read_file(name) {
            Ok(data) => dest
                .add_file(name, &data)
                .map_err(|e| format!("add {name}: {e}"))?,
            Err(_) => unreadable.push(name.clone()),
        }
        progress(written + 1);
    }

    dest.save().map_err(|e| format!("save: {e}"))?;
    Ok(unreadable)
}
