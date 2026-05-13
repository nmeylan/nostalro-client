//! Per-`EffectId` `EffectSpec` lookup.
//!
//! The default for every id is `EffectSpec::Str { file: <derived>, duration_ms: default_duration_ms(id) }`.
//! The match below overrides specific ids to point at:
//!   * a custom family (Aura, Wall, Cylinder, ...) → renders via `make_custom`
//!   * a different STR file name (when the lowercased EF_ identifier
//!     doesn't match the GRF file name)
//!   * an SPR-looping ambient sprite
//!
//! Add new overrides as effects get their visual classification confirmed
//! against the original game. Anything not listed here uses the data-driven default.

use super::effect_id::{
    classified_family, default_duration_ms, default_str_file, EffectId,
};
use super::spec::{CustomFamily, CustomFamilyParams, EffectBlend, EffectSpec, GroundRingParams};
use super::str_aliases::str_aliases;

pub fn effect_spec(id: EffectId) -> Option<EffectSpec> {
    Some(match id {
        // --- Custom families ---
        EffectId::Level99
        | EffectId::Level992
        | EffectId::Level993
        | EffectId::Level994
        | EffectId::Level995
        | EffectId::Level996 => EffectSpec::Custom {
            family: CustomFamily::Aura,
            params: CustomFamilyParams::Default,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Icewall => EffectSpec::Custom {
            family: CustomFamily::Wall,
            params: CustomFamilyParams::Default,
            duration_ms: default_duration_ms(id),
        },

        // --- GroundRing per-effect parameters (hand-curated, textures verified
        //     against the original game's effect source) ---
        // EF_WARP: yellow ring portal. The original game's Warp() emits a 3D circle
        // with innerSize=10 and growing radius, textured RING_YELLOW.TGA.
        EffectId::Warp => EffectSpec::Custom {
            family: CustomFamily::GroundRing,
            params: CustomFamilyParams::GroundRing(GroundRingParams {
                texture: "ring_yellow.tga",
                radius: 12.0,
                thickness: 12.0,
                rotation_deg_per_sec: 60.0,
                color: [1.0, 1.0, 1.0, 0.78],
                blend: EffectBlend::Additive,
                fade_in_ms: 100,
                fade_out_ms: 200,
            }),
            duration_ms: default_duration_ms(id),
        },
        // EF_MAGNUMBREAK: yellow expanding shockwave (original game's MagnumBreak()).
        EffectId::Magnumbreak => EffectSpec::Custom {
            family: CustomFamily::GroundRing,
            params: CustomFamilyParams::GroundRing(GroundRingParams {
                texture: "ring_yellow.tga",
                radius: 22.0,
                thickness: 22.0,
                rotation_deg_per_sec: 90.0,
                color: [1.0, 1.0, 1.0, 0.95],
                blend: EffectBlend::Additive,
                fade_in_ms: 50,
                fade_out_ms: 200,
            }),
            duration_ms: default_duration_ms(id),
        },
        // EF_LANDPROTECTOR: the original game routes to VOLCANO("effect\\ring_white.tga").
        EffectId::Landprotector => EffectSpec::Custom {
            family: CustomFamily::GroundRing,
            params: CustomFamilyParams::GroundRing(GroundRingParams {
                texture: "ring_white.tga",
                radius: 20.0,
                thickness: 20.0,
                rotation_deg_per_sec: 25.0,
                color: [1.0, 1.0, 1.0, 0.9],
                blend: EffectBlend::Additive,
                fade_in_ms: 200,
                fade_out_ms: 300,
            }),
            duration_ms: default_duration_ms(id),
        },
        // EF_BOTTOM_SANC: the original game routes to Bottom_Magnus("effect\\alpha_down.tga", 1).
        EffectId::BottomSanc => EffectSpec::Custom {
            family: CustomFamily::GroundRing,
            params: CustomFamilyParams::GroundRing(GroundRingParams {
                texture: "alpha_down.tga",
                radius: 24.0,
                thickness: 24.0,
                rotation_deg_per_sec: 15.0,
                color: [1.0, 1.0, 1.0, 0.85],
                blend: EffectBlend::Additive,
                fade_in_ms: 300,
                fade_out_ms: 400,
            }),
            duration_ms: default_duration_ms(id),
        },
        // EF_WARPZONE: the original game's WarpZone() emits a 3D circle textured
        // "effect\\alpha_down.tga".
        EffectId::Warpzone => EffectSpec::Custom {
            family: CustomFamily::GroundRing,
            params: CustomFamilyParams::GroundRing(GroundRingParams {
                texture: "alpha_down.tga",
                radius: 15.0,
                thickness: 15.0,
                rotation_deg_per_sec: 20.0,
                color: [1.0, 1.0, 1.0, 0.5],
                blend: EffectBlend::Additive,
                fade_in_ms: 150,
                fade_out_ms: 200,
            }),
            duration_ms: default_duration_ms(id),
        },
        // EF_WARPZONE2: the original game routes to WarpZone2("effect\\ring_blue.tga").
        EffectId::Warpzone2 => EffectSpec::Custom {
            family: CustomFamily::GroundRing,
            params: CustomFamilyParams::GroundRing(GroundRingParams {
                texture: "ring_blue.tga",
                radius: 15.0,
                thickness: 15.0,
                rotation_deg_per_sec: 30.0,
                color: [1.0, 1.0, 1.0, 0.85],
                blend: EffectBlend::Additive,
                fade_in_ms: 100,
                fade_out_ms: 200,
            }),
            duration_ms: default_duration_ms(id),
        },

        EffectId::Barrier => EffectSpec::Custom {
            family: CustomFamily::GroundRing,
            params: CustomFamilyParams::Default,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Beginspell
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
        | EffectId::Beginasura11 => EffectSpec::Custom {
            family: CustomFamily::CastCircle,
            params: CustomFamilyParams::Default,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Earthspike => EffectSpec::Custom {
            family: CustomFamily::SpikeRow,
            params: CustomFamilyParams::Default,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Grimtooth | EffectId::Grimtoothatk => EffectSpec::Custom {
            family: CustomFamily::SpikeRow,
            params: CustomFamilyParams::Default,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Magnus => EffectSpec::Custom {
            family: CustomFamily::CylinderPillar,
            params: CustomFamilyParams::Default,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Grandcross | EffectId::Grandcross2 => EffectSpec::Custom {
            family: CustomFamily::CrossBeam,
            params: CustomFamilyParams::Default,
            duration_ms: default_duration_ms(id),
        },
        // --- Map ambient SPR loops ---
        EffectId::Torch => EffectSpec::Spr {
            sprite: "data/sprite/이팩트/불꽃",
            duration_ms: u32::MAX,
        },

        // Hand-curated STR filename overrides (when the original game's STR file isn't
        // simply the lowercased EF_ name).
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
    match (primary, classified_family(id)) {
        (Some(file), Some(family)) => EffectSpec::StrHybrid {
            file,
            family,
            duration_ms,
        },
        (Some(file), None) => EffectSpec::Str { file, duration_ms },
        (None, Some(family)) => EffectSpec::Custom {
            family,
            params: CustomFamilyParams::Default,
            duration_ms,
        },
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
    fn lv99_resolves_to_aura_family() {
        assert!(matches!(
            effect_spec(EffectId::Level99),
            Some(EffectSpec::Custom {
                family: CustomFamily::Aura,
                ..
            })
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
    fn bash_falls_through_to_classified_custom() {
        // EF_BASH has no STR file in the original game but does have a
        // custom dispatch - the classifier picks a family, no hand
        // override needed.
        assert!(matches!(
            effect_spec(EffectId::Bash),
            Some(EffectSpec::Custom { .. })
        ));
    }

    #[test]
    fn warp_carries_ground_ring_params() {
        match effect_spec(EffectId::Warp) {
            Some(EffectSpec::Custom {
                family: CustomFamily::GroundRing,
                params: CustomFamilyParams::GroundRing(p),
                ..
            }) => {
                assert_eq!(p.texture, "magic_target.tga");
                assert!(p.radius > 0.0);
            }
            other => panic!("expected GroundRing params for Warp, got {other:?}"),
        }
    }
}
