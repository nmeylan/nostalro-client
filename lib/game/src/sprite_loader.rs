use ragnarok_formats::act::ActFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::imf::{ImfFile, ImfLayerOrder};
use ragnarok_formats::pal::PalFile;
use ragnarok_formats::spr::SprFile;

pub use ragnarok_formats::spr::SpriteData;

use crate::data_table::accessory_table::AccessoryTable;
use crate::data_table::name_table::NameTable;
use crate::sprite_path::{
    body_palette_path, body_sprite_path, entity_sprite_base_path, head_palette_path,
    head_sprite_path, imf_fallback_job, imf_path, mercenary_imf_path, mercenary_sprite_path,
    mercenary_weapon_sprite_path, weapon_sprite_path,
};
use models::enums::weapon::WeaponType;

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

    if ragnarok_profiling::debug::trace_texture_load() {
        tracing::info!(
            "Loaded sprite: {spr_path} ({indexed_count} indexed + {rgba_count} rgba, {} actions)",
            act.actions.len()
        );
    }

    Some(SpriteData {
        images,
        indexed_count,
        act,
    })
}

pub fn load_sprite_data_from_spr(grf: &GrfArchive, spr_path: &str) -> Option<SpriteData> {
    let base = spr_path.strip_suffix(".spr").unwrap_or(spr_path);
    load_sprite_data(grf, spr_path, &format!("{base}.act"))
}

pub fn read_layer_order(grf: &GrfArchive, path: &str) -> Option<ImfLayerOrder> {
    let data = grf.read_file(path).ok()?;
    match ImfFile::parse(&data) {
        Ok(imf) => ImfLayerOrder::from_file(&imf),
        Err(e) => {
            tracing::warn!("Failed to parse IMF {path}: {e}");
            None
        }
    }
}

pub fn load_layer_order(grf: &GrfArchive, job: u16, sex: u8) -> Option<ImfLayerOrder> {
    let mut job = job;
    loop {
        if let Some(order) = read_layer_order(grf, &imf_path(job, sex)) {
            return Some(order);
        }
        job = imf_fallback_job(job)?;
    }
}

pub fn load_body_sprite(
    grf: &GrfArchive,
    job: u16,
    sex: u8,
    cloth_color: u16,
) -> Option<SpriteData> {
    let base_path = body_sprite_path(job, sex);
    let spr_path = format!("{base_path}.spr");
    let act_path = format!("{base_path}.act");

    let spr_data = match grf.read_file(&spr_path) {
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
    let act_data = match grf.read_file(&act_path) {
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

    let override_palette = if cloth_color > 0 {
        let pal_path = body_palette_path(job, sex, cloth_color);
        match grf.read_file(&pal_path) {
            Ok(pal_data) => match PalFile::parse(&pal_data) {
                Ok(pal) => Some(pal.colors),
                Err(e) => {
                    tracing::warn!("Failed to parse palette {pal_path}: {e}");
                    None
                }
            },
            Err(_) => {
                tracing::warn!("Body palette not found: {pal_path}");
                None
            }
        }
    } else {
        None
    };

    let rgba_count = spr.rgba_sprites.len();
    let (images, indexed_count) = spr.to_rgba_images_with_palette(override_palette.as_ref());

    if ragnarok_profiling::debug::trace_texture_load() {
        tracing::info!(
            "Loaded sprite: {spr_path} ({indexed_count} indexed + {rgba_count} rgba, {} actions)",
            act.actions.len()
        );
    }

    Some(SpriteData {
        images,
        indexed_count,
        act,
    })
}

pub fn load_head_sprite(
    grf: &GrfArchive,
    head_id: u16,
    sex: u8,
    hair_color: u16,
    orc_face: bool,
) -> Option<SpriteData> {
    let base_path = if orc_face {
        crate::sprite_path::ORCFACE_SPRITE_PATH.to_string()
    } else {
        head_sprite_path(head_id, sex)
    };
    let hair_color = if orc_face { 0 } else { hair_color };
    let spr_path = format!("{base_path}.spr");
    let act_path = format!("{base_path}.act");

    let spr_data = match grf.read_file(&spr_path) {
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
    let act_data = match grf.read_file(&act_path) {
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

    let override_palette = if hair_color > 0 {
        let pal_path = head_palette_path(head_id, sex, hair_color);
        match grf.read_file(&pal_path) {
            Ok(pal_data) => match PalFile::parse(&pal_data) {
                Ok(pal) => Some(pal.colors),
                Err(e) => {
                    tracing::warn!("Failed to parse palette {pal_path}: {e}");
                    None
                }
            },
            Err(_) => {
                tracing::warn!("Head palette not found: {pal_path}");
                None
            }
        }
    } else {
        None
    };

    let rgba_count = spr.rgba_sprites.len();
    let (images, indexed_count) = spr.to_rgba_images_with_palette(override_palette.as_ref());

    if ragnarok_profiling::debug::trace_texture_load() {
        tracing::info!(
            "Loaded sprite: {spr_path} ({indexed_count} indexed + {rgba_count} rgba, {} actions)",
            act.actions.len()
        );
    }

    Some(SpriteData {
        images,
        indexed_count,
        act,
    })
}

pub fn load_weapon_sprite(
    grf: &GrfArchive,
    job: u16,
    sex: u8,
    weapon_type: WeaponType,
) -> Option<SpriteData> {
    let base_path = weapon_sprite_path(job, sex, weapon_type);
    let result = load_sprite_data(
        grf,
        &format!("{base_path}.spr"),
        &format!("{base_path}.act"),
    );
    if result.is_some() {
        return result;
    }
    if let Some(base_job) = crate::sprite_path::unmounted_job(job) {
        return load_weapon_sprite(grf, base_job, sex, weapon_type);
    }
    if let Some(base_job) = crate::sprite_path::transcendent_to_base_class(job) {
        use models::enums::EnumWithNumberValue;
        let fallback_path = weapon_sprite_path(base_job.value() as u16, sex, weapon_type);
        return load_sprite_data(
            grf,
            &format!("{fallback_path}.spr"),
            &format!("{fallback_path}.act"),
        );
    }
    None
}

pub fn load_weapon_trail_sprite(
    grf: &GrfArchive,
    job: u16,
    sex: u8,
    weapon_type: WeaponType,
) -> Option<SpriteData> {
    let base_path = crate::sprite_path::weapon_trail_sprite_path(job, sex, weapon_type)?;
    let result = load_sprite_data(
        grf,
        &format!("{base_path}.spr"),
        &format!("{base_path}.act"),
    );
    if result.is_some() {
        return result;
    }
    if let Some(base_job) = crate::sprite_path::unmounted_job(job) {
        return load_weapon_trail_sprite(grf, base_job, sex, weapon_type);
    }
    if let Some(base_job) = crate::sprite_path::transcendent_to_base_class(job) {
        use models::enums::EnumWithNumberValue;
        let fallback_path =
            crate::sprite_path::weapon_trail_sprite_path(base_job.value() as u16, sex, weapon_type)?;
        return load_sprite_data(
            grf,
            &format!("{fallback_path}.spr"),
            &format!("{fallback_path}.act"),
        );
    }
    None
}

pub fn load_headgear_sprite(grf: &GrfArchive, suffix: &str, sex: u8) -> Option<SpriteData> {
    let base_path = crate::sprite_path::headgear_sprite_path(suffix, sex);
    load_sprite_data(
        grf,
        &format!("{base_path}.spr"),
        &format!("{base_path}.act"),
    )
}

pub fn load_shield_sprite(grf: &GrfArchive, view_id: u16, job: u16, sex: u8) -> Option<SpriteData> {
    if let Some(base_path) = crate::sprite_path::shield_sprite_path(view_id, job, sex) {
        let result = load_sprite_data(
            grf,
            &format!("{base_path}.spr"),
            &format!("{base_path}.act"),
        );
        if result.is_some() {
            tracing::debug!("load_shield_sprite: view_id={view_id} path={base_path}");
            return result;
        }
    }
    let base_path = crate::sprite_path::shield_sprite_path_numeric(view_id, job, sex);
    let result = load_sprite_data(
        grf,
        &format!("{base_path}.spr"),
        &format!("{base_path}.act"),
    );
    if result.is_some() {
        tracing::debug!(
            "load_shield_sprite: view_id={view_id} numeric_path={base_path} loaded={}",
            result.is_some()
        );
        return result;
    }
    if let Some(base_job) = crate::sprite_path::unmounted_job(job) {
        return load_shield_sprite(grf, view_id, base_job, sex);
    }
    None
}

pub struct PlayerSpriteData {
    pub body: SpriteData,
    pub head: Option<SpriteData>,
    pub weapon: Option<SpriteData>,
    pub weapon_trail: Option<SpriteData>,
    pub headgear_top: Option<SpriteData>,
    pub headgear_mid: Option<SpriteData>,
    pub headgear_bottom: Option<SpriteData>,
    pub shield: Option<SpriteData>,
    pub shadow: Option<SpriteData>,
    pub layer_order: Option<ImfLayerOrder>,
}

fn load_headgear(
    grf: &GrfArchive,
    accessory_table: &AccessoryTable,
    view_id: u16,
    sex: u8,
) -> Option<SpriteData> {
    if view_id == 0 {
        return None;
    }
    let suffix = accessory_table.get_suffix(view_id)?;
    load_headgear_sprite(grf, suffix, sex)
}

pub fn load_player_sprite_data(
    grf: &GrfArchive,
    accessory_table: &AccessoryTable,
    job: u16,
    sex: u8,
    head_id: u16,
    hair_color: u16,
    cloth_color: u16,
    weapon: Option<WeaponType>,
    head_top: u16,
    head_mid: u16,
    head_bottom: u16,
    shield_id: u16,
    orc_face: bool,
    is_gm: bool,
) -> Option<PlayerSpriteData> {
    let head = load_head_sprite(grf, head_id, sex, hair_color, orc_face);
    if is_gm {
        let body = load_sprite_data_from_spr(
            grf,
            &format!("{}.spr", crate::sprite_path::gm_body_sprite_path(sex)),
        )?;
        let weapon = load_sprite_data_from_spr(
            grf,
            &format!("{}.spr", crate::sprite_path::gm_weapon_sprite_path(sex)),
        );
        return Some(PlayerSpriteData {
            body,
            head,
            weapon,
            weapon_trail: None,
            headgear_top: load_headgear(grf, accessory_table, head_top, sex),
            headgear_mid: load_headgear(grf, accessory_table, head_mid, sex),
            headgear_bottom: load_headgear(grf, accessory_table, head_bottom, sex),
            shield: None,
            shadow: load_shadow_sprite(grf),
            layer_order: None,
        });
    }
    let body = load_body_sprite(grf, job, sex, cloth_color)?;
    // Costume bodies never render a weapon or shield.
    let (weapon, shield_id) = if crate::sprite_path::is_costume_job(job) {
        (None, 0)
    } else {
        (weapon, shield_id)
    };
    let weapon_type = weapon;
    let weapon = weapon_type.and_then(|wt| load_weapon_sprite(grf, job, sex, wt));
    let weapon_trail = weapon
        .as_ref()
        .and_then(|_| weapon_type)
        .and_then(|wt| load_weapon_trail_sprite(grf, job, sex, wt));
    let headgear_top = load_headgear(grf, accessory_table, head_top, sex);
    let headgear_mid = load_headgear(grf, accessory_table, head_mid, sex);
    let headgear_bottom = load_headgear(grf, accessory_table, head_bottom, sex);
    let shield = if shield_id > 0 {
        load_shield_sprite(grf, shield_id, job, sex)
    } else {
        None
    };
    let shadow = load_shadow_sprite(grf);
    Some(PlayerSpriteData {
        body,
        head,
        weapon,
        weapon_trail,
        headgear_top,
        headgear_mid,
        headgear_bottom,
        shield,
        shadow,
        layer_order: load_layer_order(grf, job, sex),
    })
}

pub fn load_cursor_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(grf, "data/sprite/cursors.spr", "data/sprite/cursors.act")
}

/// Mercenaries are player-style composites: a mercenary body plus a regular
/// character head. The name table entry carries the sex/type sub-path (e.g.
/// `여\활용병`); the head index follows the original client's `(job % 23) + 1`.
pub fn load_mercenary_sprite_data(
    grf: &GrfArchive,
    name_table: &NameTable,
    job: u16,
) -> Option<PlayerSpriteData> {
    let name = name_table.get_name(job)?;
    let sex = if name.starts_with('여') { 0 } else { 1 };
    let base = mercenary_sprite_path(name);
    let body = load_sprite_data(grf, &format!("{base}.spr"), &format!("{base}.act"))?;
    let head_id = (job % 23) + 1;
    let head = load_head_sprite(grf, head_id, sex, 0, false);
    let shadow = load_shadow_sprite(grf);
    let weapon_base = mercenary_weapon_sprite_path(name);
    let weapon = weapon_base
        .as_deref()
        .and_then(|w| load_sprite_data(grf, &format!("{w}.spr"), &format!("{w}.act")));
    // Bow mercenaries have no weapon-trail sprite; the load simply returns None.
    let weapon_trail = weapon
        .as_ref()
        .and(weapon_base.as_deref())
        .and_then(|w| load_sprite_data(grf, &format!("{w}_검광.spr"), &format!("{w}_검광.act")));
    Some(PlayerSpriteData {
        body,
        head,
        weapon,
        weapon_trail,
        headgear_top: None,
        headgear_mid: None,
        headgear_bottom: None,
        shield: None,
        shadow,
        layer_order: mercenary_imf_path(name).and_then(|p| read_layer_order(grf, &p)),
    })
}

pub fn load_shadow_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(grf, "data/sprite/shadow.spr", "data/sprite/shadow.act")
}

pub fn load_emotion_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(
        grf,
        "data/sprite/이팩트/emotion.spr",
        "data/sprite/이팩트/emotion.act",
    )
}

pub fn load_status_overlay_sprite(
    grf: &GrfArchive,
    overlay: crate::ailment::AilmentOverlay,
) -> Option<SpriteData> {
    let (base, _) = overlay.sprite();
    load_sprite_data(grf, &format!("{base}.spr"), &format!("{base}.act"))
}

pub fn load_damage_number_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(
        grf,
        "data/sprite/이팩트/숫자.spr",
        "data/sprite/이팩트/숫자.act",
    )
}

pub fn load_rank_font_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(
        grf,
        "data/sprite/이팩트/rankfont.spr",
        "data/sprite/이팩트/rankfont.act",
    )
}

pub fn load_time_font_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(
        grf,
        "data/sprite/이팩트/timefont.spr",
        "data/sprite/이팩트/timefont.act",
    )
}

pub fn load_damage_miss_msg_sprite(grf: &GrfArchive) -> Option<SpriteData> {
    load_sprite_data(
        grf,
        "data/sprite/이팩트/msg.spr",
        "data/sprite/이팩트/msg.act",
    )
}

pub struct SimpleEntitySpriteData {
    pub body: SpriteData,
    pub shadow: Option<SpriteData>,
}

pub fn load_entity_sprite_data(
    grf: &GrfArchive,
    name_table: &NameTable,
    job: u16,
) -> Option<SimpleEntitySpriteData> {
    let base_path = entity_sprite_base_path(name_table, job)?;
    let body = load_sprite_data(
        grf,
        &format!("{base_path}.spr"),
        &format!("{base_path}.act"),
    )?;
    let shadow = load_shadow_sprite(grf);
    Some(SimpleEntitySpriteData { body, shadow })
}
