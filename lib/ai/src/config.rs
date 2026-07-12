use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::consts::FriendClass;
use crate::tactics::{
    PvpTactic, Tactic, default_merc_tactics, default_pvp_tactics, default_tactics,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[allow(non_snake_case)]
pub struct HomunConfig {
    // Basic
    pub AggroHP: i32,
    pub AggroSP: i32,
    pub OldHomunType: i32,
    pub UseSkillOnly: i32,
    pub UseAttackSkill: i32,
    pub OpportunisticTargeting: i32,
    pub DoNotChase: i32,
    pub UseDanceAttack: i32,
    pub SuperPassive: i32,
    pub RescueOwnerLowHP: i32,
    pub AssumeHomun: i32,
    pub AttackLastFullSP: i32,
    pub DanceMinSP: i32,
    pub TankMonsterLimit: i32,
    pub StationaryAggroDist: i32,
    pub MobileAggroDist: i32,
    pub UseAvoid: i32,
    pub DoNotAttackMoving: i32,
    pub LagReduction: i32,
    pub LiveMobID: i32,
    pub PainkillerFriends: i32,
    pub PainkillerFriendsSave: i32,

    // AutoSkill
    pub AttackSkillReserveSP: i32,
    pub AutoMobMode: i32,
    pub AutoMobCount: i32,
    pub AutoComboMode: i32,
    pub AutoComboSkill: i32,
    pub AutoComboSpheres: i32,
    pub UseHomunSSkillChase: i32,
    pub UseHomunSSkillAttack: i32,
    pub AutoSkillDelay: i32,
    pub AutoSkillLimit: i32,
    pub AoEMaximizeTargets: i32,
    pub AoEReserveSP: i32,
    pub AoEFixedLevel: i32,
    pub CastTimeRatio: f32,
    pub UseAutoPushback: i32,
    pub AutoPushbackThreshold: i32,
    pub AttackTimeLimit: i32,

    // Homun-S skills
    pub UseEiraSilentBreeze: i32,
    pub EiraSilentBreezeLevel: i32,
    pub UseEiraXenoSlasher: i32,
    pub EiraXenoSlasherLevel: i32,
    pub UseEiraEraseCutter: i32,
    pub EiraEraseCutterLevel: i32,
    pub UseEiraOveredBoost: i32,
    pub UseBayeriStahlHorn: i32,
    pub BayeriStahlHornLevel: i32,
    pub UseBayeriHailegeStar: i32,
    pub BayeriHailegeStarLevel: i32,
    pub UseBayeriAngriffModus: i32,
    pub UseBayeriGoldenPherze: i32,
    pub UseBayeriSteinWand: i32,
    pub BayeriSteinWandLevel: i32,
    pub UseSteinWandSelfMob: i32,
    pub UseSteinWandOwnerMob: i32,
    pub UseSteinWandTele: i32,
    pub StienWandTelePause: i32,
    pub UseSeraParalyze: i32,
    pub SeraParalyzeLevel: i32,
    pub UseSeraPoisonMist: i32,
    pub SeraPoisonMistLevel: i32,
    pub UseSeraCallLegion: i32,
    pub SeraCallLegionLevel: i32,
    pub UseSeraPainkiller: i32,
    pub UseEleanorSonicClaw: i32,
    pub EleanorSonicClawLevel: i32,
    pub EleanorSilverveinLevel: i32,
    pub EleanorMidnightLevel: i32,
    pub EleanorDoNotSwitchMode: i32,
    pub UseDieterLavaSlide: i32,
    pub DieterLavaSlideLevel: i32,
    pub UseDieterMagmaFlow: i32,
    pub UseDieterGraniticArmor: i32,
    pub UseDieterPyroclastic: i32,

    // Walk / Follow
    pub FollowStayBack: i32,
    pub RestXOff: i32,
    pub RestYOff: i32,
    pub DoNotUseRest: i32,
    pub SpawnDelay: i32,
    pub MoveSticky: i32,
    pub MoveStickyFight: i32,
    pub UseIdleWalk: i32,
    pub IdleWalkSP: i32,
    pub IdleWalkDistance: i32,
    pub UseCastleRoute: i32,
    pub RelativeRoute: i32,
    pub ChaseSPPause: i32,
    pub ChaseSPPauseSP: i32,
    pub ChaseSPPauseTime: i32,
    pub StationaryMoveBounds: i32,
    pub MobileMoveBounds: i32,

    // Autobuff / heal
    pub UseOffensiveBuff: i32,
    pub UseDefensiveBuff: i32,
    pub DefensiveBuffOwnerHP: i32,
    pub DefensiveBuffOwnerMobbed: i32,
    pub UseProvokeOwner: i32,
    pub ProvokeOwnerMobbed: i32,
    pub LifEscapeLevel: i32,
    pub FilirFlitLevel: i32,
    pub FilirAccelLevel: i32,
    pub AmiBulwarkLevel: i32,
    pub HealOwnerHP: i32,
    pub HealSelfHP: i32,
    pub HealOwnerBreeze: i32,
    pub UseAutoHeal: i32,
    pub LavaSlideMode: i32,
    pub PoisonMistMode: i32,
    pub UseCastleDefend: i32,
    pub CastleDefendThreshold: i32,
    pub UseSmartBulwark: i32,

    // Kiting
    pub KiteMonsters: i32,
    pub KiteStep: i32,
    pub KiteParanoidStep: i32,
    pub KiteThreshold: i32,
    pub KiteParanoidThreshold: i32,
    pub KiteBounds: i32,
    pub KiteParanoid: i32,
    pub ForceKite: i32,
    pub FleeHP: i32,

    // Friending
    pub StandbyFriending: i32,
    pub MirAIFriending: i32,

    // Standby
    pub DefendStandby: i32,
    pub StickyStandby: i32,

    // Berserk (Berzerk/Berserk reference spellings normalized to one)
    pub UseBerserkMobbed: i32,
    pub UseBerserkSkill: i32,
    pub UseBerserkAttack: i32,
    pub Berserk_SkillAlways: i32,
    pub Berserk_Dance: i32,
    pub Berserk_IgnoreMinSP: i32,
    pub Berserk_ComboAlways: i32,

    // PVP
    pub PVPmode: i32,
}

impl Default for HomunConfig {
    fn default() -> Self {
        HomunConfig {
            AggroHP: 60,
            AggroSP: 0,
            OldHomunType: 3,
            UseSkillOnly: -1,
            UseAttackSkill: 1,
            OpportunisticTargeting: 0,
            DoNotChase: 0,
            UseDanceAttack: 0,
            SuperPassive: 0,
            RescueOwnerLowHP: 0,
            AssumeHomun: 1,
            AttackLastFullSP: 0,
            DanceMinSP: 0,
            TankMonsterLimit: 4,
            StationaryAggroDist: 12,
            MobileAggroDist: 7,
            UseAvoid: 0,
            DoNotAttackMoving: 0,
            LagReduction: 0,
            LiveMobID: 0,
            PainkillerFriends: 0,
            PainkillerFriendsSave: 0,

            AttackSkillReserveSP: 0,
            AutoMobMode: 2,
            AutoMobCount: 2,
            AutoComboMode: 1,
            AutoComboSkill: 0,
            AutoComboSpheres: 10,
            UseHomunSSkillChase: 1,
            UseHomunSSkillAttack: 1,
            AutoSkillDelay: 400,
            AutoSkillLimit: 100,
            AoEMaximizeTargets: 0,
            AoEReserveSP: 1,
            AoEFixedLevel: 0,
            CastTimeRatio: 0.80,
            UseAutoPushback: 0,
            AutoPushbackThreshold: 2,
            AttackTimeLimit: 10000,

            UseEiraSilentBreeze: 0,
            EiraSilentBreezeLevel: 5,
            UseEiraXenoSlasher: 0,
            EiraXenoSlasherLevel: 0,
            UseEiraEraseCutter: 0,
            EiraEraseCutterLevel: 0,
            UseEiraOveredBoost: 0,
            UseBayeriStahlHorn: 1,
            BayeriStahlHornLevel: 5,
            UseBayeriHailegeStar: 1,
            BayeriHailegeStarLevel: 5,
            UseBayeriAngriffModus: 0,
            UseBayeriGoldenPherze: 0,
            UseBayeriSteinWand: 0,
            BayeriSteinWandLevel: 5,
            UseSteinWandSelfMob: 2,
            UseSteinWandOwnerMob: 2,
            UseSteinWandTele: 0,
            StienWandTelePause: 3000,
            UseSeraParalyze: 0,
            SeraParalyzeLevel: 5,
            UseSeraPoisonMist: 0,
            SeraPoisonMistLevel: 5,
            UseSeraCallLegion: 1,
            SeraCallLegionLevel: 5,
            UseSeraPainkiller: 0,
            UseEleanorSonicClaw: 1,
            EleanorSonicClawLevel: 5,
            EleanorSilverveinLevel: 5,
            EleanorMidnightLevel: 5,
            EleanorDoNotSwitchMode: 0,
            UseDieterLavaSlide: 1,
            DieterLavaSlideLevel: 5,
            UseDieterMagmaFlow: 0,
            UseDieterGraniticArmor: 0,
            UseDieterPyroclastic: 0,

            FollowStayBack: 2,
            RestXOff: 2,
            RestYOff: 0,
            DoNotUseRest: 0,
            SpawnDelay: 1000,
            MoveSticky: 0,
            MoveStickyFight: 0,
            UseIdleWalk: 0,
            IdleWalkSP: 80,
            IdleWalkDistance: 4,
            UseCastleRoute: 0,
            RelativeRoute: 1,
            ChaseSPPause: 0,
            ChaseSPPauseSP: -60,
            ChaseSPPauseTime: 3000,
            StationaryMoveBounds: 14,
            MobileMoveBounds: 9,

            UseOffensiveBuff: 1,
            UseDefensiveBuff: 1,
            DefensiveBuffOwnerHP: 0,
            DefensiveBuffOwnerMobbed: 0,
            UseProvokeOwner: 0,
            ProvokeOwnerMobbed: 3,
            LifEscapeLevel: 5,
            FilirFlitLevel: 1,
            FilirAccelLevel: 1,
            AmiBulwarkLevel: 5,
            HealOwnerHP: 50,
            HealSelfHP: 50,
            HealOwnerBreeze: 0,
            UseAutoHeal: 0,
            LavaSlideMode: 0,
            PoisonMistMode: 0,
            UseCastleDefend: 0,
            CastleDefendThreshold: 4,
            UseSmartBulwark: 0,

            KiteMonsters: 0,
            KiteStep: 5,
            KiteParanoidStep: 2,
            KiteThreshold: 3,
            KiteParanoidThreshold: 2,
            KiteBounds: 10,
            KiteParanoid: 0,
            ForceKite: 0,
            FleeHP: 0,

            StandbyFriending: 1,
            MirAIFriending: 1,

            DefendStandby: 0,
            StickyStandby: 1,

            UseBerserkMobbed: 0,
            UseBerserkSkill: 0,
            UseBerserkAttack: 0,
            Berserk_SkillAlways: 0,
            Berserk_Dance: 0,
            Berserk_IgnoreMinSP: 0,
            Berserk_ComboAlways: 0,

            PVPmode: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[allow(non_snake_case)]
pub struct MercConfig {
    // Basic
    pub AggroHP: i32,
    pub AggroSP: i32,
    pub UseSkillOnly: i32,
    pub UseAttackSkill: i32,
    pub OpportunisticTargeting: i32,
    pub DoNotChase: i32,
    pub UseDanceAttack: i32,
    pub SuperPassive: i32,
    pub AttackLastHPSP: i32,
    pub AssumeHomun: i32,
    pub RescueOwnerLowHP: i32,
    pub AttackLastFullSP: i32,
    pub TankMonsterLimit: i32,
    pub StationaryAggroDist: i32,
    pub MobileAggroDist: i32,
    pub AutoDetectPlant: i32,

    // AutoSkill
    pub AttackSkillReserveSP: i32,
    pub AutoMobMode: i32,
    pub AutoMobCount: i32,
    pub AutoComboMode: i32,
    pub AutoComboSkill: i32,
    pub AutoComboSpheres: i32,
    pub UseAutoPushback: i32,
    pub AutoPushbackThreshold: i32,
    pub AutoSkillDelay: i32,
    pub AutoSkillLimit: i32,
    pub AoEMaximizeTargets: i32,
    pub AoEReserveSP: i32,
    pub AoEFixedLevel: i32,
    pub CastTimeRatio: f32,
    pub AttackTimeLimit: i32,

    // Autobuff (merc-only additions)
    pub UseDefensiveBuff: i32,
    pub UseOffensiveBuff: i32,
    pub UseProvokeOwner: i32,
    pub UseProvokeSelf: i32,
    pub UseSacrificeOwner: i32,
    pub UseAutoMag: i32,
    pub UseAutoSight: i32,
    pub DefensiveBuffOwnerHP: i32,
    pub DefensiveBuffOwnerMobbed: i32,
    pub UseBlessingOwner: i32,
    pub UseBlessingSelf: i32,
    pub UseIncAgiSelf: i32,
    pub UseIncAgiOwner: i32,
    pub UseKyrieSelf: i32,
    pub UseKyrieOwner: i32,
    pub ProvokeOwnerMobbed: i32,

    // Walk / Follow
    pub FollowStayBack: i32,
    pub RestXOff: i32,
    pub RestYOff: i32,
    pub DoNotUseRest: i32,
    pub SpawnDelay: i32,
    pub MoveSticky: i32,
    pub MoveStickyFight: i32,
    pub UseIdleWalk: i32,
    pub IdleWalkSP: i32,
    pub IdleWalkDistance: i32,
    pub RelativeRoute: i32,
    pub ChaseSPPause: i32,
    pub ChaseSPPauseSP: i32,
    pub ChaseSPPauseTime: i32,
    pub StationaryMoveBounds: i32,
    pub MobileMoveBounds: i32,

    // Kiting
    pub KiteMonsters: i32,
    pub KiteStep: i32,
    pub KiteParanoidStep: i32,
    pub KiteThreshold: i32,
    pub KiteParanoidThreshold: i32,
    pub KiteBounds: i32,
    pub KiteParanoid: i32,
    pub ForceKite: i32,
    pub FleeHP: i32,

    // Friending / Standby
    pub StandbyFriending: i32,
    pub MirAIFriending: i32,
    pub DefendStandby: i32,
    pub StickyStandby: i32,

    // Berserk
    pub UseBerserkMobbed: i32,
    pub UseBerserkSkill: i32,
    pub UseBerserkAttack: i32,
    pub Berserk_SkillAlways: i32,
    pub Berserk_Dance: i32,
    pub Berserk_IgnoreMinSP: i32,

    // PVP
    pub PVPmode: i32,
}

impl Default for MercConfig {
    fn default() -> Self {
        MercConfig {
            AggroHP: 60,
            AggroSP: 0,
            UseSkillOnly: -1,
            UseAttackSkill: 1,
            OpportunisticTargeting: 0,
            DoNotChase: 0,
            UseDanceAttack: 0,
            SuperPassive: 0,
            AttackLastHPSP: 80,
            AssumeHomun: 1,
            RescueOwnerLowHP: 0,
            AttackLastFullSP: 0,
            TankMonsterLimit: 4,
            StationaryAggroDist: 12,
            MobileAggroDist: 7,
            AutoDetectPlant: 1,

            AttackSkillReserveSP: 0,
            AutoMobMode: 2,
            AutoMobCount: 2,
            AutoComboMode: 1,
            AutoComboSkill: 0,
            AutoComboSpheres: 10,
            UseAutoPushback: 0,
            AutoPushbackThreshold: 2,
            AutoSkillDelay: 400,
            AutoSkillLimit: 100,
            AoEMaximizeTargets: 0,
            AoEReserveSP: 0,
            AoEFixedLevel: 0,
            CastTimeRatio: 0.80,
            AttackTimeLimit: 10000,

            UseDefensiveBuff: 1,
            UseOffensiveBuff: 1,
            UseProvokeOwner: 0,
            UseProvokeSelf: 0,
            UseSacrificeOwner: 0,
            UseAutoMag: 0,
            UseAutoSight: 1,
            DefensiveBuffOwnerHP: 0,
            DefensiveBuffOwnerMobbed: 0,
            UseBlessingOwner: 1,
            UseBlessingSelf: 0,
            UseIncAgiSelf: 0,
            UseIncAgiOwner: 1,
            UseKyrieSelf: 0,
            UseKyrieOwner: 0,
            ProvokeOwnerMobbed: 0,

            FollowStayBack: 2,
            RestXOff: -2,
            RestYOff: 0,
            DoNotUseRest: 0,
            SpawnDelay: 1000,
            MoveSticky: 0,
            MoveStickyFight: 0,
            UseIdleWalk: 0,
            IdleWalkSP: 80,
            IdleWalkDistance: 4,
            RelativeRoute: 1,
            ChaseSPPause: 0,
            ChaseSPPauseSP: 0,
            ChaseSPPauseTime: 0,
            StationaryMoveBounds: 12,
            MobileMoveBounds: 9,

            KiteMonsters: 0,
            KiteStep: 5,
            KiteParanoidStep: 2,
            KiteThreshold: 3,
            KiteParanoidThreshold: 2,
            KiteBounds: 10,
            KiteParanoid: 0,
            ForceKite: 0,
            FleeHP: 0,

            StandbyFriending: 1,
            MirAIFriending: 1,
            DefendStandby: 0,
            StickyStandby: 1,

            UseBerserkMobbed: 0,
            UseBerserkSkill: 0,
            UseBerserkAttack: 0,
            Berserk_SkillAlways: 0,
            Berserk_Dance: 0,
            Berserk_IgnoreMinSP: 0,

            PVPmode: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanionAiConfig {
    #[serde(default)]
    pub homunculus: HomunConfig,
    #[serde(default)]
    pub mercenary: MercConfig,
    #[serde(default = "default_tactics")]
    pub homunculus_tactics: Vec<Tactic>,
    #[serde(default = "default_merc_tactics")]
    pub mercenary_tactics: Vec<Tactic>,
    #[serde(default = "default_pvp_tactics")]
    pub homunculus_pvp_tactics: Vec<PvpTactic>,
    #[serde(default = "default_pvp_tactics")]
    pub mercenary_pvp_tactics: Vec<PvpTactic>,
    #[serde(default)]
    pub friends: HashMap<u32, FriendClass>,
}

impl Default for CompanionAiConfig {
    fn default() -> Self {
        CompanionAiConfig {
            homunculus: HomunConfig::default(),
            mercenary: MercConfig::default(),
            homunculus_tactics: default_tactics(),
            mercenary_tactics: default_merc_tactics(),
            homunculus_pvp_tactics: default_pvp_tactics(),
            mercenary_pvp_tactics: default_pvp_tactics(),
            friends: HashMap::new(),
        }
    }
}

impl CompanionAiConfig {
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!("companion AI config parse failed, using defaults: {e}");
                CompanionAiConfig::default()
            }),
            Err(_) => CompanionAiConfig::default(),
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)
    }
}
