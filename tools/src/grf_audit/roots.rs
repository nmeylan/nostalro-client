//! Everything the client can name without being told by another file: the
//! registry constants, plus a path per row of every data table it reads.

use super::{Need, Options, Origin, normalize, server::ServerData};
use models::enums::EnumWithNumberValue;
use models::enums::class::JobName;
use models::enums::skill_enums::SkillEnum;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::data_table::accessory_table::AccessoryTable;
use ragnarok_game::data_table::item_resource_table::ItemResourceTable;
use ragnarok_game::data_table::name_table::NameTable;
use ragnarok_game::sound::tables;
use ragnarok_game::sprite_path as sp;
use ragnarok_resources as res;

/// Job ids run to 6046 (mercenaries); scan a little past that.
const MAX_JOB_ID: u16 = 6100;
/// Shield and weapon view ids are small, dense and unlisted.
const MAX_VIEW_ID: u16 = 32;
/// Headgear view ids as sent in the look packets.
const MAX_HEADGEAR_VIEW_ID: u16 = 2000;

/// Highest skill id the enum knows about.
const MAX_SKILL_ID: u32 = 1000;

/// `SkillEnum::from_id` panics on the gaps between id ranges, so probe it and
/// keep what comes back.
fn all_skills() -> Vec<SkillEnum> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let skills = (1..=MAX_SKILL_ID)
        .filter_map(|id| std::panic::catch_unwind(|| SkillEnum::from_id(id)).ok())
        .collect();
    std::panic::set_hook(previous);
    skills
}

pub fn has_identity_tables(grf: &GrfArchive) -> bool {
    [
        res::lua::JOB_IDENTITY_514_LUB,
        res::lua::NPC_IDENTITY_514_LUB,
        res::lua::JOB_IDENTITY_LUB,
        res::lua::NPC_IDENTITY_LUB,
        res::lua::JOB_IDENTITY_LUA,
        res::lua::NPC_IDENTITY_LUA,
    ]
    .iter()
    .any(|p| grf.file_exists(p))
}

pub fn collect(
    grf: &GrfArchive,
    opts: &Options,
    server: Option<&ServerData>,
) -> Vec<(String, Origin, Need)> {
    let mut out: Vec<(String, Origin, Need)> = Vec::new();
    macro_rules! need {
        ($path:expr, $origin:expr $(,)?) => {
            out.push((normalize(&$path), $origin, Need::Required))
        };
    }
    macro_rules! probe {
        ($path:expr, $origin:expr $(,)?) => {
            out.push((normalize(&$path), $origin, Need::Probed))
        };
    }

    // Some registry entries are alternates rather than requirements: `.lua` and
    // `.lub` are two encodings of one table, `num2*` is the pre-rename spelling
    // of `idnum2*`, and pet artwork is cosmetic. The archive path is not an
    // entry at all.
    let alternates: &[&str] = &[
        res::table::UNIDENTIFIED_ITEM_DESC,
        res::table::UNIDENTIFIED_ITEM_NAME,
        res::table::UNIDENTIFIED_ITEM_RESOURCE,
    ];
    for path in res::all_static_paths() {
        if res::grf::ALL.contains(&path) {
            continue;
        }
        if res::lua::ALL.contains(&path)
            || alternates.contains(&path)
            || path.starts_with("data/texture/유저인터페이스/illust/")
        {
            probe!(path.to_string(), Origin::Registry);
        } else {
            need!(path.to_string(), Origin::Registry);
        }
    }

    // -- effects ------------------------------------------------------------
    for path in ragnarok_effects::effect_texture_paths() {
        need!(path, Origin::Table("effect texture table"));
    }
    for path in ragnarok_effects::effect_spr_paths() {
        need!(path.to_string(), Origin::Table("effect sprite table"));
    }
    for path in ragnarok_effects::custom_effect_sprite_paths() {
        need!(path.to_string(), Origin::Table("effect sprite table"));
    }
    for id in 0..u16::MAX as usize {
        let Ok(effect) = models::enums::effect_id::EffectId::try_from_value(id) else {
            continue;
        };
        for alias in ragnarok_effects::str_aliases::str_aliases(effect) {
            need!(
                res::texture::effect::str_file(alias),
                Origin::Table("effect str alias table"),
            );
        }
    }

    // -- maps ---------------------------------------------------------------
    // Without a server list every map in the archive is a root, so nothing map
    // shaped is ever called unused.
    let maps: Vec<String> = match server {
        Some(s) => s.maps.clone(),
        None => grf
            .entry_names()
            .filter(|n| n.to_lowercase().ends_with(".rsw"))
            .filter_map(|n| {
                let n = normalize(n);
                let stem = n.strip_prefix("data/")?.strip_suffix(".rsw")?;
                (!stem.contains('/')).then(|| stem.to_string())
            })
            .collect(),
    };
    let map_origin = if server.is_some() {
        Origin::Table("server map list")
    } else {
        Origin::Table("archive map list")
    };
    for map in &maps {
        need!(res::map::rsw(map), map_origin.clone());
        need!(res::map::gnd(map), map_origin.clone());
        need!(res::map::gat(map), map_origin.clone());
        // Plenty of maps ship no minimap; the window just draws nothing.
        probe!(res::ui::minimap::of(map), map_origin.clone());
    }
    // -- items --------------------------------------------------------------
    let items = ItemResourceTable::load(grf);
    for id in 0..=u16::MAX {
        for identified in [true, false] {
            if let Some(name) = items.get_resource_name_for(id, identified) {
                need!(
                    res::ui::item::icon(name),
                    Origin::Table("item resource table")
                );
                probe!(
                    res::sprite::item::of(name),
                    Origin::Table("item resource table")
                );
                probe!(
                    res::ui::collection::named(name),
                    Origin::Table("item resource table"),
                );
            }
        }
    }

    // -- actors -------------------------------------------------------------
    let names = NameTable::load(grf);
    for job in 0..MAX_JOB_ID {
        if let Some(base) = sp::entity_sprite_base_path(&names, job) {
            need!(base, Origin::Table("job/npc identity table"));
        }
        // Mercenaries carry a weapon sprite and an imf keyed off their body name.
        if (sp::MERCENARY_JOB_MIN..=sp::MERCENARY_JOB_MAX).contains(&job)
            && let Some(name) = names.get_name(job)
        {
            let name = name.to_string();
            if let Some(imf) = sp::mercenary_imf_path(&name) {
                probe!(imf, Origin::Table("job/npc identity table"));
            }
            if let Some(weapon) = sp::mercenary_weapon_sprite_path(&name) {
                probe!(weapon, Origin::Table("job/npc identity table"));
            }
        }
    }

    // -- players ------------------------------------------------------------
    // A character is not drawn as its job id alone: mounts and costumes swap the
    // body for another one, each with its own sprite, palette set and imf.
    let mut jobs: Vec<u16> = (0..MAX_JOB_ID)
        .filter(|j| JobName::try_from_value(*j as usize).is_ok())
        .collect();
    for base in jobs.clone() {
        jobs.extend(sp::mounted_job(base));
        jobs.extend(sp::unmounted_job(base));
    }
    for option in [
        sp::OPTION_WEDDING,
        sp::OPTION_SANTA,
        sp::OPTION_SUMMER,
        sp::OPTION_RIDING,
    ] {
        jobs.extend(sp::costume_job(option));
        for base in jobs.clone() {
            jobs.push(sp::visual_job(base, option));
        }
    }
    jobs.sort_unstable();
    jobs.dedup();

    for &job in &jobs {
        for sex in 0u8..2 {
            probe!(sp::body_sprite_path(job, sex), Origin::Table("job list"));
            probe!(sp::imf_path(job, sex), Origin::Table("job list"));
            for palette in 0..opts.max_palette_id {
                probe!(
                    sp::body_palette_path(job, sex, palette),
                    Origin::Table("job list"),
                );
            }
            for view in 0..MAX_VIEW_ID {
                if let Some(weapon) = sp::weapon_view_id_to_type(view) {
                    probe!(
                        sp::weapon_sprite_path(job, sex, weapon),
                        Origin::Table("job list"),
                    );
                }
                if let Some(path) = sp::shield_sprite_path(view, job, sex) {
                    probe!(path, Origin::Table("job list"));
                }
            }
        }
    }
    for sex in 0u8..2 {
        probe!(sp::gm_body_sprite_path(sex), Origin::Registry);
        probe!(sp::gm_weapon_sprite_path(sex), Origin::Registry);
        for head in 0..opts.max_head_id {
            probe!(sp::head_sprite_path(head, sex), Origin::Table("head ids"));
            for palette in 0..opts.max_palette_id {
                probe!(
                    sp::head_palette_path(head, sex, palette),
                    Origin::Table("head ids"),
                );
            }
        }
    }

    // -- headgear -----------------------------------------------------------
    let accessories = AccessoryTable::load_from_grf(grf);
    for sex in 0u8..2 {
        for view in 0..MAX_HEADGEAR_VIEW_ID {
            if let Some(suffix) = accessories.get_suffix(view) {
                probe!(
                    sp::headgear_sprite_path(suffix, sex),
                    Origin::Table("accessory table"),
                );
            }
        }
    }

    // -- sounds -------------------------------------------------------------
    for skill in all_skills() {
        let wavs = [
            tables::skill_use_sound(skill).map(|(w, _)| w),
            tables::skill_cast_begin_sound(skill).map(|(w, _)| w),
            tables::skill_projectile_sound(skill),
        ];
        for wav in wavs.into_iter().flatten() {
            need!(res::sound::sfx(wav), Origin::Table("skill sound table"));
        }
    }

    // One path can arrive from several places; the strongest claim wins, so sort
    // Required ahead of Probed before collapsing duplicates.
    out.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then((a.2 == Need::Probed).cmp(&(b.2 == Need::Probed)))
    });
    out.dedup_by(|a, b| a.0 == b.0);
    out
}
