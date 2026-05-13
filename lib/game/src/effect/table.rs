//! Per-`EffectId` `EffectSpec` lookup.
//!
//! The default for every id is `EffectSpec::Str { file: <derived>, duration_ms: default_duration_ms(id) }`.
//! The match below overrides specific ids to point at:
//!   * a `Custom` payload → dispatched by [`super::factory::make_effect`]
//!   * a different STR file name (when the lowercased EF_ identifier
//!     doesn't match the GRF file name)
//!   * an SPR-looping ambient sprite

use super::effect_id::{
    classified_family, default_duration_ms, default_str_file, EffectId,
};
use super::effects::{bottom_sanc, land_protector, magnum_break, warp};
use super::spec::{CustomFamily, EffectSpec};
use super::str_aliases::str_aliases;

pub fn effect_spec(id: EffectId) -> Option<EffectSpec> {
    Some(match id {
        // EF_WARP runs longer than the default duration table claims: original game's
        // parent emitter dies at frame 80 but it keeps spawning rings until
        // then, and each ring lives 80 frames on its own — so the last ring
        // doesn't finish fading until ~140 frames after spawn.
        EffectId::Warp => EffectSpec::Custom {
            duration_ms: warp::TOTAL_DURATION_MS,
        },

        // Magnum Break's visible explosion runs ~700 ms; the duration table
        // value (300 ms) cuts the cone off before the ring finishes growing.
        EffectId::Magnumbreak => EffectSpec::Custom {
            duration_ms: magnum_break::TOTAL_DURATION_MS,
        },

        // Bottom Sanctuary is sustained — the parent emitter lives until the
        // skill cell expires (table value already 99990 ms, but pin it via
        // the effect module so it stays load-bearing on the constant).
        EffectId::BottomSanc => EffectSpec::Custom {
            duration_ms: bottom_sanc::TOTAL_DURATION_MS,
        },

        // LandProtector's visible burst is one cycle of the four GI ring
        // emitters (~110 frames @ 60fps ≈ 1.83s); the table value 9990ms
        // outlives the animation and leaves a dead spawn lingering.
        EffectId::Landprotector => EffectSpec::Custom {
            duration_ms: land_protector::TOTAL_DURATION_MS,
        },

        // --- Factory-dispatched custom effects ---
        // The factory picks the concrete implementation; the spec only
        // carries the lifetime.
        EffectId::Warpzone
        | EffectId::Warpzone2
        | EffectId::Level99
        | EffectId::Level992
        | EffectId::Level993
        | EffectId::Level994
        | EffectId::Level995
        | EffectId::Level996
        | EffectId::Icewall
        | EffectId::Beginspell
        | EffectId::Beginspell2
        | EffectId::Beginspell3
        | EffectId::Beginspell4
        | EffectId::Beginspell5
        | EffectId::Beginspell6
        | EffectId::Beginspell7
        | EffectId::Beginspell8
        | EffectId::Beginspellred
        | EffectId::Beginspellwhite
        | EffectId::BeginspellN
        | EffectId::Beginasura
        | EffectId::Beginasura1
        | EffectId::Beginasura2
        | EffectId::Beginasura3
        | EffectId::Beginasura4
        | EffectId::Beginasura5
        | EffectId::Beginasura6
        | EffectId::Beginasura7
        | EffectId::Beginasura11
        | EffectId::Earthspike
        | EffectId::Grimtooth
        | EffectId::Grimtoothatk
        | EffectId::Magnus
        | EffectId::Grandcross
        | EffectId::Grandcross2
        | EffectId::Barrier => EffectSpec::Custom {
            duration_ms: default_duration_ms(id),
        },

        // --- Map ambient SPR loops ---
        EffectId::Torch => EffectSpec::Spr {
            sprite: "data/sprite/이팩트/불꽃",
            duration_ms: u32::MAX,
        },

        // Hand-curated STR filename overrides (when the original game's STR file isn't
        // simply the lowercased EF_ identifier).
        EffectId::Springtrap => EffectSpec::Str {
            file: "spring",
            duration_ms: default_duration_ms(id),
        },

        _ => default_spec(id),
    })
}

fn default_spec(id: EffectId) -> EffectSpec {
    let duration_ms = default_duration_ms(id);
    let primary = str_aliases(id).first().copied();
    let family = classified_family(id)
        .filter(|f| !matches!(f, CustomFamily::Bespoke(_)));
    match (primary, family) {
        (Some(file), Some(family)) => EffectSpec::StrHybrid {
            file,
            family,
            duration_ms,
        },
        (Some(file), None) => EffectSpec::Str { file, duration_ms },
        (None, Some(_)) => EffectSpec::Custom { duration_ms },
        (None, None) => EffectSpec::Str {
            file: default_str_file(id),
            duration_ms,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lv99_resolves_to_custom_factory_path() {
        assert!(matches!(
            effect_spec(EffectId::Level99),
            Some(EffectSpec::Custom { .. })
        ));
    }

    #[test]
    fn known_str_files_resolve() {
        assert!(matches!(
            effect_spec(EffectId::Bubble),
            Some(EffectSpec::Str { file: "bubble1", .. })
        ));
        assert!(matches!(
            effect_spec(EffectId::Lvup),
            Some(EffectSpec::Str {
                file: "LevelUP",
                ..
            })
        ));
    }

    #[test]
    fn torch_is_an_spr_loop() {
        assert!(matches!(
            effect_spec(EffectId::Torch),
            Some(EffectSpec::Spr { .. })
        ));
    }

    #[test]
    fn stormgust_is_str_hybrid() {
        assert!(matches!(
            effect_spec(EffectId::Stormgust),
            Some(EffectSpec::StrHybrid {
                family: CustomFamily::SpikeRow,
                ..
            })
        ));
    }

    #[test]
    fn warp_routes_to_factory_via_custom() {
        assert!(matches!(
            effect_spec(EffectId::Warp),
            Some(EffectSpec::Custom { .. })
        ));
    }
}
