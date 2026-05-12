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

use super::generated::{EffectId, default_duration_ms, default_str_file};
use super::spec::{CustomFamily, EffectSpec};

pub fn effect_spec(id: EffectId) -> Option<EffectSpec> {
    Some(match id {
        // --- Custom families ---
        EffectId::Level99 | EffectId::Level992 | EffectId::Level993 | EffectId::Level994
        | EffectId::Level995 | EffectId::Level996 => EffectSpec::Custom {
            family: CustomFamily::Aura,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Icewall => EffectSpec::Custom {
            family: CustomFamily::Wall,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Earthspike => EffectSpec::Custom {
            family: CustomFamily::SpikeRow,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Grimtooth | EffectId::Grimtoothatk => EffectSpec::Custom {
            family: CustomFamily::SpikeRow,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Magnus => EffectSpec::Custom {
            family: CustomFamily::CylinderPillar,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Grandcross | EffectId::Grandcross2 => EffectSpec::Custom {
            family: CustomFamily::CrossBeam,
            duration_ms: default_duration_ms(id),
        },
        EffectId::Stormgust => EffectSpec::Custom {
            family: CustomFamily::Bespoke(EffectId::Stormgust),
            duration_ms: default_duration_ms(id),
        },

        // --- Map ambient SPR loops ---
        EffectId::Torch => EffectSpec::Spr {
            sprite: "data/sprite/이팩트/불꽃",
            duration_ms: u32::MAX,
        },

        // Hand-curated STR filename overrides (when the original STR file isn't
        // simply the lowercased EF_ name).
        EffectId::Springtrap => EffectSpec::Str {
            file: "spring",
            duration_ms: default_duration_ms(id),
        },

        // --- Everything else: default to STR with a name derived from the
        // id. If the GRF doesn't contain that file, StrEffectCache::load
        // logs a warning and the effect doesn't render — that's the
        // graceful failure mode for unmapped IDs. ---
        _ => EffectSpec::Str {
            file: default_str_file(id),
            duration_ms: default_duration_ms(id),
        },
    })
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
            Some(EffectSpec::Str { file: "bubble", .. })
        ));
        assert!(matches!(
            effect_spec(EffectId::Lvup),
            Some(EffectSpec::Str { file: "lvup", .. })
        ));
    }

    #[test]
    fn torch_is_an_spr_loop() {
        assert!(matches!(
            effect_spec(EffectId::Torch),
            Some(EffectSpec::Spr { .. })
        ));
    }
}
