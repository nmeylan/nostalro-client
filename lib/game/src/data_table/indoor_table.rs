use std::collections::HashSet;
use std::sync::OnceLock;

use ragnarok_formats::grf::GrfArchive;

static INDOOR_TABLE: OnceLock<HashSet<String>> = OnceLock::new();

pub fn indoor_table(grf: &GrfArchive) -> &'static HashSet<String> {
    INDOOR_TABLE.get_or_init(|| {
        let mut set = HashSet::new();
        match grf.read_file(ragnarok_resources::table::INDOOR_RSW) {
            Ok(data) => {
                let text = String::from_utf8_lossy(&data);
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with("//") {
                        continue;
                    }
                    let name = trimmed.trim_end_matches('#').trim();
                    if !name.is_empty() {
                        set.insert(name.to_ascii_lowercase());
                    }
                }
                tracing::info!("Loaded indoor rsw table ({} entries)", set.len());
            }
            Err(e) => tracing::info!("No indoor rsw table in GRF: {e}"),
        }
        set
    })
}
