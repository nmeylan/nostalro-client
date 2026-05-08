use std::sync::OnceLock;

use ragnarok_formats::fog_table::{FogEntry, FogTable};
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::RswFile;

use crate::map_coordinates::MapCoordinates;

pub struct MapData {
    pub rsw: RswFile,
    pub gnd: GndFile,
    pub gat: Option<GatFile>,
    pub coordinates: Option<MapCoordinates>,
    pub fog: Option<FogEntry>,
}

static FOG_TABLE: OnceLock<Option<FogTable>> = OnceLock::new();

fn fog_table(grf: &GrfArchive) -> Option<&'static FogTable> {
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

pub fn load_map_data(grf: &GrfArchive, map_name: &str) -> Option<MapData> {
    let rsw_path = format!("data/{map_name}.rsw");
    let rsw_data = match grf.read_file(&rsw_path) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to read RSW {rsw_path}: {e}");
            return None;
        }
    };
    let rsw = match RswFile::parse(&rsw_data) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to parse RSW: {e}");
            return None;
        }
    };

    let gnd_path = format!("data/{map_name}.gnd");
    let gnd_data = match grf.read_file(&gnd_path) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to read GND {gnd_path}: {e}");
            return None;
        }
    };
    let gnd = match GndFile::parse(&gnd_data) {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("Failed to parse GND: {e}");
            return None;
        }
    };

    println!(
        "Map: {map_name} ({}x{}, {} textures, {} surfaces, {} lightmaps)",
        gnd.width,
        gnd.height,
        gnd.textures.len(),
        gnd.surfaces.len(),
        gnd.lightmaps.len()
    );

    let mut gat_file = None;
    let mut coordinates = None;

    let gat_path = format!("data/{map_name}.gat");
    if let Ok(gat_data) = grf.read_file(&gat_path)
        && let Ok(gat) = GatFile::parse(&gat_data)
    {
        coordinates = Some(MapCoordinates::new(
            gnd.zoom, gat.width, gat.height, gnd.width, gnd.height,
        ));
        gat_file = Some(gat);
    }

    let fog = fog_table(grf).and_then(|table| table.get(&format!("{map_name}.rsw")));

    Some(MapData {
        rsw,
        gnd,
        gat: gat_file,
        coordinates,
        fog,
    })
}
