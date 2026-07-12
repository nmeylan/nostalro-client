pub mod config;
pub mod consts;
pub mod context;
pub mod engine;
pub mod tactics;

pub use config::{CompanionAiConfig, HomunConfig, MercConfig};
pub use consts::*;
pub use context::{ActorView, AiContext, AiIntent, AiParams, Motion};
pub use engine::{AiState, CommandKind, CompanionAi, OwnerCommand};
pub use tactics::{PvpTactic, SkillUse, Tactic, TacticTable};

#[cfg(test)]
mod config_tests {
    use crate::config::CompanionAiConfig;
    use crate::consts::{BuffWhen, UseAutoHeal, UseIdleWalk, UseSkillOnly};
    use crate::tactics::SkillUse;

    #[test]
    fn config_json_round_trip_is_identity() {
        let cfg = CompanionAiConfig::default();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let back: CompanionAiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn partial_json_self_heals_to_defaults() {
        let json = r#"{ "homunculus": { "AggroHP": 42 } }"#;
        let cfg: CompanionAiConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.homunculus.AggroHP, 42);
        // Untouched fields fall back to the reference defaults.
        assert_eq!(cfg.homunculus.AggroSP, 0);
        assert_eq!(cfg.mercenary.AggroHP, 60);
        assert_eq!(cfg.homunculus_tactics.len(), 2);
    }

    #[test]
    fn enum_integers_match_reference_values() {
        assert_eq!(i32::from(UseSkillOnly::Chasing), -1);
        assert_eq!(UseSkillOnly::from(1), UseSkillOnly::SkillOnly);
        assert_eq!(i32::from(BuffWhen::Asap), 3);
        assert_eq!(BuffWhen::from(-2), BuffWhen::IdleLow);
        assert_eq!(i32::from(UseAutoHeal::IdleLow), 3);
        assert_eq!(i32::from(UseIdleWalk::RouteCircle), 6);
        // Unknown integers self-heal to the declared default variant.
        assert_eq!(UseSkillOnly::from(999), UseSkillOnly::default());
    }

    #[test]
    fn skill_use_encodes_count_and_once_at_level() {
        assert_eq!(SkillUse::from(0), SkillUse::Never);
        assert_eq!(SkillUse::from(100), SkillUse::Always);
        assert_eq!(SkillUse::from(3), SkillUse::Times(3));
        assert_eq!(SkillUse::from(-4), SkillUse::OnceAtLevel(4));
        assert_eq!(i32::from(SkillUse::OnceAtLevel(4)), -4);
    }
}
