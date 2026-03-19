use ragnarok_formats::act::ActFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::spr::{RgbaImageData, SprFile};

use crate::sprite_path::body_sprite_path;

pub struct SpriteData {
    pub images: Vec<RgbaImageData>,
    pub indexed_count: usize,
    pub act: ActFile,
}

pub fn load_sprite_data(grf: &GrfArchive, spr_path: &str, act_path: &str) -> Option<SpriteData> {
    let spr_data = match grf.read_file(spr_path) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("Failed to read SPR {spr_path}: {e}");
            return None;
        }
    };
    let spr = match SprFile::parse(&spr_data) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to parse SPR {spr_path}: {e}");
            return None;
        }
    };
    let act_data = match grf.read_file(act_path) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("Failed to read ACT {act_path}: {e}");
            return None;
        }
    };
    let act = match ActFile::parse(&act_data) {
        Ok(a) => a,
        Err(e) => {
            tracing::warn!("Failed to parse ACT {act_path}: {e}");
            return None;
        }
    };

    let rgba_count = spr.rgba_sprites.len();
    let (images, indexed_count) = spr.to_rgba_images();

    tracing::info!("Loaded sprite: {spr_path} ({indexed_count} indexed + {rgba_count} rgba, {} actions)",
        act.actions.len());

    Some(SpriteData { images, indexed_count, act })
}

pub fn load_body_sprite(grf: &GrfArchive, job: u16, sex: u8) -> Option<SpriteData> {
    let base_path = body_sprite_path(job, sex);
    load_sprite_data(grf, &format!("{base_path}.spr"), &format!("{base_path}.act"))
}

pub fn load_cursor_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(grf, "data/sprite/cursors.spr", "data/sprite/cursors.act")
}
