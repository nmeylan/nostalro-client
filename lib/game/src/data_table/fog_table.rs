use std::sync::OnceLock;

use ragnarok_formats::fog_table::FogTable;
use ragnarok_formats::grf::GrfArchive;

static FOG_TABLE: OnceLock<Option<FogTable>> = OnceLock::new();

pub fn fog_table(grf: &GrfArchive) -> Option<&'static FogTable> {
    FOG_TABLE
        .get_or_init(|| match grf.read_file("data/fogparametertable.txt") {
            Ok(data) => match FogTable::parse(&data) {
                Ok(table) => {
                    tracing::info!("Loaded fog table ({} entries)", table.entries.len());
                    Some(table)
                }
                Err(e) => {
                    tracing::warn!("Failed to parse fog table: {e}");
                    None
                }
            },
            Err(e) => {
                tracing::info!("No fog table in GRF: {e}");
                None
            }
        })
        .as_ref()
}
