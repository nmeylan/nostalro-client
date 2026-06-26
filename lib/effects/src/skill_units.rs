//! Ground-skill unit (`e_skill_unit_id`) → [`EffectId`] map.
//!
//! The server sends one `ZC_SKILL_ENTRY` packet per occupied cell, carrying a
//! `job` byte that is the `e_skill_unit_id`; the client renders one effect at
//! that cell per packet (no client-side area expansion — Firewall = N blades =
//! N packets). On `ZC_SKILL_DISAPPEAR` the matching unit is removed by its
//! `aid`.
//!
//! Unit ids are rathena's `e_skill_unit_id` (`src/map/skill.hpp`, contiguous
//! from `0x7e`); rathena writes the raw enum value as the wire `job` byte
//! (`clif_getareachar_skillunit`). Effect choices follow robrowser's
//! `DB/Skills/SkillUnit.js`; the few `'NNN_ground'` composites there resolve to
//! the same-numbered `EffectId` (our enum is in `EF_*` numeric order).
//! Unrecognised / renewal / no-visual units return `None` (no spawn).

use models::enums::effect_id::EffectId;

// Classic-era `e_skill_unit_id` values (rathena `skill.hpp`).
const UNT_SAFETYWALL: u8 = 0x7e;
const UNT_FIREWALL: u8 = 0x7f;
const UNT_WARP_WAITING: u8 = 0x80;
const UNT_WARP_ACTIVE: u8 = 0x81;
const UNT_SANCTUARY: u8 = 0x83;
const UNT_MAGNUS: u8 = 0x84;
const UNT_PNEUMA: u8 = 0x85;
const UNT_FIREPILLAR_WAITING: u8 = 0x87;
const UNT_FIREPILLAR_ACTIVE: u8 = 0x88;
const UNT_ICEWALL: u8 = 0x8d;
const UNT_QUAGMIRE: u8 = 0x8e;
const UNT_BLASTMINE: u8 = 0x8f;
const UNT_VENOMDUST: u8 = 0x92;
const UNT_SHOCKWAVE: u8 = 0x94;
const UNT_SANDMAN: u8 = 0x95;
const UNT_FLASHER: u8 = 0x96;
const UNT_FREEZINGTRAP: u8 = 0x97;
const UNT_CLAYMORETRAP: u8 = 0x98;
const UNT_VOLCANO: u8 = 0x9a;
const UNT_DELUGE: u8 = 0x9b;
const UNT_VIOLENTGALE: u8 = 0x9c;
const UNT_LANDPROTECTOR: u8 = 0x9d;
const UNT_LULLABY: u8 = 0x9e;
const UNT_RICHMANKIM: u8 = 0x9f;
const UNT_ETERNALCHAOS: u8 = 0xa0;
const UNT_DRUMBATTLEFIELD: u8 = 0xa1;
const UNT_RINGNIBELUNGEN: u8 = 0xa2;
const UNT_ROKISWEIL: u8 = 0xa3;
const UNT_INTOABYSS: u8 = 0xa4;
const UNT_SIEGFRIED: u8 = 0xa5;
const UNT_DISSONANCE: u8 = 0xa6;
const UNT_WHISTLE: u8 = 0xa7;
const UNT_ASSASSINCROSS: u8 = 0xa8;
const UNT_POEMBRAGI: u8 = 0xa9;
const UNT_APPLEIDUN: u8 = 0xaa;
const UNT_UGLYDANCE: u8 = 0xab;
const UNT_HUMMING: u8 = 0xac;
const UNT_DONTFORGETME: u8 = 0xad;
const UNT_FORTUNEKISS: u8 = 0xae;
const UNT_SERVICEFORYOU: u8 = 0xaf;
const UNT_DEMONSTRATION: u8 = 0xb1;
const UNT_GOSPEL: u8 = 0xb3;
const UNT_BASILICA: u8 = 0xb4;
const UNT_FOGWALL: u8 = 0xb6;
const UNT_SPIDERWEB: u8 = 0xb7;
const UNT_GRAVITATION: u8 = 0xb8;
const UNT_HERMODE: u8 = 0xb9;
const UNT_SUITON: u8 = 0xbb;
const UNT_TATAMIGAESHI: u8 = 0xbc;
const UNT_KAEN: u8 = 0xbd;

/// Effect to render at a ground-skill unit cell, or `None` for units with no
/// client visual (hidden/`DUMMYSKILL`, unmapped traps, renewal-only units).
pub fn skill_unit_effect(unit_id: u8) -> Option<EffectId> {
    use EffectId as E;
    Some(match unit_id {
        UNT_SAFETYWALL => E::Glasswall2,
        UNT_FIREWALL => E::Firewall,
        UNT_WARP_WAITING => E::Readyportal2,
        UNT_WARP_ACTIVE => E::Portal2,
        UNT_SANCTUARY => E::BottomSanc,
        UNT_MAGNUS => E::BottomMag,
        UNT_PNEUMA => E::Pneuma,
        UNT_FIREPILLAR_WAITING | UNT_FIREPILLAR_ACTIVE => E::Firepillaron,
        UNT_ICEWALL => E::Icewall,
        UNT_QUAGMIRE => E::Quagmire,
        UNT_VENOMDUST => E::Venomdust2,
        UNT_VOLCANO => E::BottomVo,
        UNT_DELUGE => E::BottomDe,
        UNT_VIOLENTGALE => E::BottomVi,
        UNT_LANDPROTECTOR => E::BottomLa,
        UNT_DEMONSTRATION => E::Demonstration,
        UNT_GOSPEL => E::BottomGospel,
        UNT_BASILICA => E::BottomBasilica,
        UNT_FOGWALL => E::BottomFogwall,
        UNT_SPIDERWEB => E::BottomSpider,
        UNT_GRAVITATION => E::Gravitation,
        UNT_HERMODE => E::BottomHermode,
        UNT_SUITON => E::BottomSuiton,
        UNT_TATAMIGAESHI => E::Tatami,
        UNT_KAEN => E::Kaen,

        // Songs / dances — the ground "bottom" marker at the performer's cell.
        UNT_LULLABY => E::BottomLullaby,
        UNT_RICHMANKIM => E::BottomRichmankim,
        UNT_ETERNALCHAOS => E::BottomEternalchaos,
        UNT_DRUMBATTLEFIELD => E::BottomDrumbattlefield,
        UNT_RINGNIBELUNGEN => E::BottomRingnibelungen,
        UNT_ROKISWEIL => E::BottomRokisweil,
        UNT_INTOABYSS => E::BottomIntoabyss,
        UNT_SIEGFRIED => E::BottomSiegfried,
        UNT_DISSONANCE => E::BottomDissonance,
        UNT_WHISTLE => E::BottomWhistle,
        UNT_ASSASSINCROSS => E::BottomAssassincross,
        UNT_POEMBRAGI => E::BottomPoembragi,
        UNT_APPLEIDUN => E::BottomAppleidun,
        UNT_UGLYDANCE => E::BottomUglydance,
        UNT_HUMMING => E::BottomHumming,
        UNT_DONTFORGETME => E::BottomDontforgetme,
        UNT_FORTUNEKISS => E::BottomFortunekiss,
        UNT_SERVICEFORYOU => E::BottomServiceforyou,

        // Traps — only the types the original game draws client-side
        // (`CSkill::AM_SKILLSTANDENTRY`); the rest show no client visual.
        UNT_BLASTMINE => E::Blastminebomb,
        UNT_SANDMAN => E::Sandman,
        UNT_FLASHER => E::Flasher,
        UNT_FREEZINGTRAP => E::Freezing,
        UNT_CLAYMORETRAP => E::Claymore,
        UNT_SHOCKWAVE => E::Shockwave,

        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn representative_units_map_and_unknowns_are_none() {
        // Sociable spot-check across the families: a wall, a ground "bottom",
        // a trap, a song, plus hidden/unknown bytes that must not spawn.
        assert_eq!(skill_unit_effect(UNT_FIREWALL), Some(EffectId::Firewall));
        assert_eq!(skill_unit_effect(UNT_ICEWALL), Some(EffectId::Icewall));
        assert_eq!(skill_unit_effect(UNT_SANCTUARY), Some(EffectId::BottomSanc));
        assert_eq!(
            skill_unit_effect(UNT_LANDPROTECTOR),
            Some(EffectId::BottomLa)
        );
        assert_eq!(
            skill_unit_effect(UNT_FREEZINGTRAP),
            Some(EffectId::Freezing)
        );
        assert_eq!(
            skill_unit_effect(UNT_POEMBRAGI),
            Some(EffectId::BottomPoembragi)
        );
        // 0x86 = UNT_DUMMYSKILL (invisible), 0x00 unused → no visual.
        assert_eq!(skill_unit_effect(0x86), None);
        assert_eq!(skill_unit_effect(0x00), None);
    }
}
