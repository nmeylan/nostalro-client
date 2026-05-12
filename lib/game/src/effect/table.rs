//! Lookup from `EffectId` to its `EffectSpec`. Hand-populated stub matching
//! the sample IDs in `id.rs`; full table will be code-generated from
//! the original's `durationTable` and effect classifications.

use super::id::EffectId;
use super::spec::{CustomFamily, EffectSpec};

pub fn effect_spec(id: EffectId) -> Option<EffectSpec> {
    Some(match id {
        EffectId::Torch => EffectSpec::Spr {
            sprite: "data/sprite/이팩트/불꽃",
            duration_ms: u32::MAX,
        },
        EffectId::ChimneySmoke => EffectSpec::Spr {
            sprite: "data/sprite/이팩트/연기",
            duration_ms: u32::MAX,
        },
        EffectId::Bubble => EffectSpec::Str {
            file: "bubble",
            duration_ms: 2000,
        },
        EffectId::GasPush => EffectSpec::Str {
            file: "gaspush",
            duration_ms: 1500,
        },
        EffectId::Spring => EffectSpec::Str {
            file: "spring",
            duration_ms: 1500,
        },

        EffectId::Hit1 | EffectId::Hit2 | EffectId::Hit3 => EffectSpec::Custom {
            family: CustomFamily::RadialBurst,
            duration_ms: 300,
        },

        EffectId::FireBolt => EffectSpec::Str {
            file: "fire_bolt",
            duration_ms: 800,
        },
        EffectId::ColdBolt => EffectSpec::Custom {
            family: CustomFamily::SpikeRow,
            duration_ms: 800,
        },
        EffectId::LightningBolt => EffectSpec::Str {
            file: "lightning_bolt",
            duration_ms: 800,
        },
        EffectId::IceWall => EffectSpec::Custom {
            family: CustomFamily::Wall,
            duration_ms: 20_000,
        },
        EffectId::EarthSpike => EffectSpec::Custom {
            family: CustomFamily::SpikeRow,
            duration_ms: 1200,
        },
        EffectId::GrimTooth => EffectSpec::Custom {
            family: CustomFamily::SpikeRow,
            duration_ms: 1000,
        },
        EffectId::MagnusExorcismus => EffectSpec::Custom {
            family: CustomFamily::CylinderPillar,
            duration_ms: 4000,
        },
        EffectId::GrandCross => EffectSpec::Custom {
            family: CustomFamily::CrossBeam,
            duration_ms: 1500,
        },
        EffectId::LordOfVermillion => EffectSpec::Custom {
            family: CustomFamily::Bespoke(EffectId::LordOfVermillion),
            duration_ms: 5000,
        },
        EffectId::StormGust => EffectSpec::Custom {
            family: CustomFamily::Bespoke(EffectId::StormGust),
            duration_ms: 4500,
        },

        EffectId::Level99 => EffectSpec::Custom {
            family: CustomFamily::Aura,
            duration_ms: u32::MAX,
        },
        EffectId::Lvup => EffectSpec::Str {
            file: "lvup",
            duration_ms: 2000,
        },
        EffectId::JobLvup => EffectSpec::Str {
            file: "joblvup",
            duration_ms: 2000,
        },
        EffectId::RefineOk => EffectSpec::Str {
            file: "refineok",
            duration_ms: 1500,
        },
        EffectId::RefineFail => EffectSpec::Str {
            file: "refinefail",
            duration_ms: 1500,
        },

        EffectId::Potion1 => EffectSpec::Str {
            file: "potion1",
            duration_ms: 600,
        },
        EffectId::Potion2 => EffectSpec::Str {
            file: "potion2",
            duration_ms: 600,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ids_resolve_to_specs() {
        assert!(matches!(
            effect_spec(EffectId::Level99),
            Some(EffectSpec::Custom {
                family: CustomFamily::Aura,
                ..
            })
        ));
        assert!(matches!(
            effect_spec(EffectId::IceWall),
            Some(EffectSpec::Custom {
                family: CustomFamily::Wall,
                ..
            })
        ));
        assert!(matches!(
            effect_spec(EffectId::Bubble),
            Some(EffectSpec::Str { .. })
        ));
        assert!(matches!(
            effect_spec(EffectId::Torch),
            Some(EffectSpec::Spr { .. })
        ));
    }

    #[test]
    fn bespoke_family_carries_id() {
        match effect_spec(EffectId::LordOfVermillion).unwrap() {
            EffectSpec::Custom {
                family: CustomFamily::Bespoke(id),
                ..
            } => assert_eq!(id, EffectId::LordOfVermillion),
            other => panic!("expected Bespoke, got {other:?}"),
        }
    }
}
