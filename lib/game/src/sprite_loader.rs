use ragnarok_formats::act::ActFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::spr::SprFile;

pub use ragnarok_formats::spr::SpriteData;

use models::enums::weapon::WeaponType;

use crate::accessory_table::AccessoryTable;
use crate::name_table::NameTable;
use crate::sprite_path::{body_sprite_path, head_sprite_path, weapon_sprite_path, entity_sprite_base_path};

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

pub fn load_sprite_data_from_spr(grf: &GrfArchive, spr_path: &str) -> Option<SpriteData> {
    let base = spr_path.strip_suffix(".spr").unwrap_or(spr_path);
    load_sprite_data(grf, spr_path, &format!("{base}.act"))
}

pub fn load_body_sprite(grf: &GrfArchive, job: u16, sex: u8) -> Option<SpriteData> {
    let base_path = body_sprite_path(job, sex);
    load_sprite_data(grf, &format!("{base_path}.spr"), &format!("{base_path}.act"))
}

pub fn load_head_sprite(grf: &GrfArchive, head_id: u16, sex: u8) -> Option<SpriteData> {
    let base_path = head_sprite_path(head_id, sex);
    load_sprite_data(grf, &format!("{base_path}.spr"), &format!("{base_path}.act"))
}

pub fn load_weapon_sprite(grf: &GrfArchive, job: u16, sex: u8, weapon_type: WeaponType) -> Option<SpriteData> {
    let base_path = weapon_sprite_path(job, sex, weapon_type);
    load_sprite_data(grf, &format!("{base_path}.spr"), &format!("{base_path}.act"))
}

pub fn load_headgear_sprite(grf: &GrfArchive, suffix: &str, sex: u8) -> Option<SpriteData> {
    let base_path = crate::sprite_path::headgear_sprite_path(suffix, sex);
    load_sprite_data(grf, &format!("{base_path}.spr"), &format!("{base_path}.act"))
}

pub fn load_shield_sprite(grf: &GrfArchive, view_id: u16, job: u16, sex: u8) -> Option<SpriteData> {
    let base_path = crate::sprite_path::shield_sprite_path(view_id, job, sex)?;
    load_sprite_data(grf, &format!("{base_path}.spr"), &format!("{base_path}.act"))
}

pub struct PlayerSpriteData {
    pub body: SpriteData,
    pub head: Option<SpriteData>,
    pub weapon: Option<SpriteData>,
    pub headgear_top: Option<SpriteData>,
    pub headgear_mid: Option<SpriteData>,
    pub headgear_bottom: Option<SpriteData>,
    pub shield: Option<SpriteData>,
    pub shadow: Option<SpriteData>,
}

fn load_headgear(grf: &GrfArchive, accessory_table: &AccessoryTable, view_id: u16, sex: u8) -> Option<SpriteData> {
    if view_id == 0 { return None; }
    let suffix = accessory_table.get_suffix(view_id)?;
    load_headgear_sprite(grf, suffix, sex)
}

pub fn load_player_sprite_data(
    grf: &GrfArchive,
    accessory_table: &AccessoryTable,
    job: u16,
    sex: u8,
    head_id: u16,
    weapon: Option<WeaponType>,
    head_top: u16,
    head_mid: u16,
    head_bottom: u16,
    shield_id: u16,
) -> Option<PlayerSpriteData> {
    let body = load_body_sprite(grf, job, sex)?;
    let head = load_head_sprite(grf, head_id, sex);
    let weapon = weapon.and_then(|wt| load_weapon_sprite(grf, job, sex, wt));
    let headgear_top = load_headgear(grf, accessory_table, head_top, sex);
    let headgear_mid = load_headgear(grf, accessory_table, head_mid, sex);
    let headgear_bottom = load_headgear(grf, accessory_table, head_bottom, sex);
    let shield = if shield_id > 0 { load_shield_sprite(grf, shield_id, job, sex) } else { None };
    let shadow = load_shadow_sprite(grf);
    Some(PlayerSpriteData { body, head, weapon, headgear_top, headgear_mid, headgear_bottom, shield, shadow })
}

pub fn load_cursor_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(grf, "data/sprite/cursors.spr", "data/sprite/cursors.act")
}

pub fn load_shadow_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(grf, "data/sprite/shadow.spr", "data/sprite/shadow.act")
}

pub struct SimpleEntitySpriteData {
    pub body: SpriteData,
    pub shadow: Option<SpriteData>,
}

pub fn load_entity_sprite_data(grf: &GrfArchive, name_table: &NameTable, job: u16) -> Option<SimpleEntitySpriteData> {
    let base_path = entity_sprite_base_path(name_table, job)?;
    let body = load_sprite_data(grf, &format!("{base_path}.spr"), &format!("{base_path}.act"))?;
    let shadow = load_shadow_sprite(grf);
    Some(SimpleEntitySpriteData { body, shadow })
}
