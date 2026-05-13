use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::effect::{
    ALL_EFFECT_IDS, CustomFamily, EffectSpec, effect_spec, str_aliases,
};

pub fn run(grf_path: &Path) -> ExitCode {
    let grf = match GrfArchive::open(grf_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("failed to open GRF {}: {e}", grf_path.display());
            return ExitCode::FAILURE;
        }
    };

    let mut str_present = 0usize;
    let mut str_missing = 0usize;
    let mut hybrid_present = 0usize;
    let mut hybrid_missing = 0usize;
    let mut custom_impl = 0usize;
    let mut custom_no_impl = 0usize;
    let mut spr_present = 0usize;
    let mut spr_missing = 0usize;
    let mut no_spec = 0usize;
    let mut family_counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for &id in ALL_EFFECT_IDS {
        match effect_spec(id) {
            None => no_spec += 1,
            Some(EffectSpec::Str { file, .. }) => {
                if resolves_in_grf(&grf, file, str_aliases(id)) {
                    str_present += 1;
                } else {
                    str_missing += 1;
                }
            }
            Some(EffectSpec::StrHybrid { file, family, .. }) => {
                if resolves_in_grf(&grf, file, str_aliases(id)) {
                    hybrid_present += 1;
                } else {
                    hybrid_missing += 1;
                }
                let key = format!("{} (hybrid)", family_label(family));
                let entry = family_counts.entry(key).or_insert((0, 0));
                if family_has_impl(family) {
                    entry.0 += 1;
                } else {
                    entry.1 += 1;
                }
            }
            Some(EffectSpec::Spr { sprite, .. }) => {
                let path = format!("{sprite}.spr");
                if grf.file_exists(&path) {
                    spr_present += 1;
                } else {
                    spr_missing += 1;
                }
            }
            Some(EffectSpec::Custom { family, .. }) => {
                let key = family_label(family);
                let entry = family_counts.entry(key).or_insert((0, 0));
                if family_has_impl(family) {
                    entry.0 += 1;
                    custom_impl += 1;
                } else {
                    entry.1 += 1;
                    custom_no_impl += 1;
                }
            }
        }
    }

    let total = ALL_EFFECT_IDS.len();
    println!("# Effect coverage audit");
    println!();
    println!("GRF: `{}`", grf_path.display());
    println!("Total effects: {total}");
    println!();
    println!("## Summary");
    println!();
    println!("| Spec | Status | Count |");
    println!("|---|---|---:|");
    println!("| Str | present | {str_present} |");
    println!("| Str | missing | {str_missing} |");
    println!("| StrHybrid | str present | {hybrid_present} |");
    println!("| StrHybrid | str missing | {hybrid_missing} |");
    println!("| Custom | impl | {custom_impl} |");
    println!("| Custom | not impl | {custom_no_impl} |");
    println!("| Spr | present | {spr_present} |");
    println!("| Spr | missing | {spr_missing} |");
    println!("| (none) | no spec | {no_spec} |");
    println!();
    println!("## Custom families");
    println!();
    println!("| Family | Impl | No impl |");
    println!("|---|---:|---:|");
    for (name, (yes, no)) in &family_counts {
        println!("| {name} | {yes} | {no} |");
    }

    ExitCode::SUCCESS
}

fn resolves_in_grf(grf: &GrfArchive, primary: &str, aliases: &[&str]) -> bool {
    let probe = |n: &str| grf.file_exists(&format!("data/texture/effect/{n}.str"));
    probe(primary) || aliases.iter().any(|a| probe(a))
}

fn family_has_impl(family: CustomFamily) -> bool {
    match family {
        CustomFamily::Aura
        | CustomFamily::GroundRing
        | CustomFamily::CastCircle
        | CustomFamily::SpikeRow
        | CustomFamily::Wall
        | CustomFamily::CylinderPillar
        | CustomFamily::CrossBeam
        | CustomFamily::SplineProjectile
        | CustomFamily::RadialBurst
        | CustomFamily::ScreenFlash
        | CustomFamily::FlatQuad
        | CustomFamily::HealBurst
        | CustomFamily::MeleeImpact
        | CustomFamily::AirSwirl
        | CustomFamily::StatusOrb
        | CustomFamily::FloatingSpirit
        | CustomFamily::Waterfall => true,
        CustomFamily::Bespoke(_) => false,
    }
}

fn family_label(family: CustomFamily) -> String {
    match family {
        CustomFamily::Aura => "Aura".into(),
        CustomFamily::GroundRing => "GroundRing".into(),
        CustomFamily::CastCircle => "CastCircle".into(),
        CustomFamily::SpikeRow => "SpikeRow".into(),
        CustomFamily::Wall => "Wall".into(),
        CustomFamily::CylinderPillar => "CylinderPillar".into(),
        CustomFamily::CrossBeam => "CrossBeam".into(),
        CustomFamily::SplineProjectile => "SplineProjectile".into(),
        CustomFamily::RadialBurst => "RadialBurst".into(),
        CustomFamily::ScreenFlash => "ScreenFlash".into(),
        CustomFamily::FlatQuad => "FlatQuad".into(),
        CustomFamily::HealBurst => "HealBurst".into(),
        CustomFamily::MeleeImpact => "MeleeImpact".into(),
        CustomFamily::AirSwirl => "AirSwirl".into(),
        CustomFamily::StatusOrb => "StatusOrb".into(),
        CustomFamily::FloatingSpirit => "FloatingSpirit".into(),
        CustomFamily::Waterfall => "Waterfall".into(),
        CustomFamily::Bespoke(id) => format!("Bespoke({id:?})"),
    }
}
