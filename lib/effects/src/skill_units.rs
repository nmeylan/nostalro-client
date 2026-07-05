use models::enums::effect_id::EffectId;

const UNT_SAFETYWALL: u8 = 0x7e;
const UNT_FIREWALL: u8 = 0x7f;
const UNT_WARP_WAITING: u8 = 0x80;
const UNT_WARP_ACTIVE: u8 = 0x81;
const UNT_SANCTUARY: u8 = 0x83;
const UNT_MAGNUS: u8 = 0x84;
const UNT_PNEUMA: u8 = 0x85;
const UNT_FIREPILLAR_WAITING: u8 = 0x87;
const UNT_FIREPILLAR_ACTIVE: u8 = 0x88;
/// A trap that has just been sprung — the server changes the unit's look to this
/// (`clif_changetraplook`) at the moment a monster triggers it.
pub const UNT_USED_TRAPS: u8 = 0x8c;
const UNT_ICEWALL: u8 = 0x8d;
const UNT_QUAGMIRE: u8 = 0x8e;
const UNT_BLASTMINE: u8 = 0x8f;
const UNT_SKIDTRAP: u8 = 0x90;
const UNT_ANKLESNARE: u8 = 0x91;
const UNT_VENOMDUST: u8 = 0x92;
const UNT_LANDMINE: u8 = 0x93;
const UNT_SHOCKWAVE: u8 = 0x94;
const UNT_SANDMAN: u8 = 0x95;
const UNT_FLASHER: u8 = 0x96;
const UNT_FREEZINGTRAP: u8 = 0x97;
const UNT_CLAYMORETRAP: u8 = 0x98;
const UNT_TALKIEBOX: u8 = 0x99;
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

        _ => return None,
    })
}

/// The RSM model a deployed trap shows on the ground (path relative to
/// `data\model\`), or `None` for a non-trap unit. The trigger burst (freeze,
/// blast, …) fires separately via [`trap_trigger_effect`].
pub fn trap_model_name(unit_id: u8) -> Option<&'static str> {
    Some(match unit_id {
        UNT_ANKLESNARE => "외부소품\\트랩01.rsm",
        UNT_SKIDTRAP => "외부소품\\트랩02.rsm",
        UNT_LANDMINE => "외부소품\\트랩03.rsm",
        UNT_FREEZINGTRAP => "외부소품\\트랩03_2.rsm",
        UNT_BLASTMINE => "외부소품\\트랩03_3.rsm",
        UNT_SANDMAN => "외부소품\\트랩03_4.rsm",
        UNT_FLASHER => "외부소품\\트랩03_5.rsm",
        UNT_SHOCKWAVE => "외부소품\\트랩03_6.rsm",
        UNT_CLAYMORETRAP => "외부소품\\트랩04.rsm",
        UNT_TALKIEBOX => "외부소품\\트랩05.rsm",
        _ => return None,
    })
}

/// The one-shot burst a trap plays when a monster springs it (the trap unit
/// becomes [`UNT_USED_TRAPS`]). Traps that only hold or teleport — Skid Trap,
/// Ankle Snare, Land Mine, Talkie Box, Shockwave — have no trigger burst.
pub fn trap_trigger_effect(unit_id: u8) -> Option<EffectId> {
    use EffectId as E;
    Some(match unit_id {
        UNT_BLASTMINE => E::Blastminebomb,
        UNT_FREEZINGTRAP => E::Freezing,
        UNT_SANDMAN => E::Sandman,
        UNT_FLASHER => E::Flasher,
        UNT_CLAYMORETRAP => E::Claymore,
        _ => return None,
    })
}

/// Every sprite a ground skill unit can render — its placement sprite plus any
/// trigger burst — for effects backed by a `Spr`/`SprBurst` spec. These are not
/// referenced by any RSW or effect-module sprite list, so they must be collected
/// here to be preloaded (Str-backed units are already covered by `effect_str_names`).
pub fn skill_unit_sprite_paths() -> Vec<&'static str> {
    use crate::spec::EffectSpec;
    use crate::table::effect_spec;
    let mut out = Vec::new();
    for unit_id in 0u8..=0xff {
        for id in [skill_unit_effect(unit_id), trap_trigger_effect(unit_id)]
            .into_iter()
            .flatten()
        {
            match effect_spec(id) {
                Some(EffectSpec::Spr { sprite, .. }) | Some(EffectSpec::SprBurst { sprite, .. }) => {
                    out.push(sprite)
                }
                _ => {}
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn representative_units_map_and_unknowns_are_none() {
        assert_eq!(skill_unit_effect(UNT_FIREWALL), Some(EffectId::Firewall));
        assert_eq!(skill_unit_effect(UNT_ICEWALL), Some(EffectId::Icewall));
        assert_eq!(skill_unit_effect(UNT_SANCTUARY), Some(EffectId::BottomSanc));
        assert_eq!(
            skill_unit_effect(UNT_LANDPROTECTOR),
            Some(EffectId::BottomLa)
        );
        assert_eq!(
            skill_unit_effect(UNT_POEMBRAGI),
            Some(EffectId::BottomPoembragi)
        );
        assert_eq!(skill_unit_effect(0x86), None);
        assert_eq!(skill_unit_effect(0x00), None);
    }

    #[test]
    fn traps_show_a_model_at_placement_and_burst_only_when_sprung() {
        // Placement: an RSM model, not a sprite effect.
        assert_eq!(skill_unit_effect(UNT_FREEZINGTRAP), None);
        assert_eq!(skill_unit_effect(UNT_ANKLESNARE), None);
        assert_eq!(trap_model_name(UNT_ANKLESNARE), Some("외부소품\\트랩01.rsm"));
        assert_eq!(trap_model_name(UNT_BLASTMINE), Some("외부소품\\트랩03_3.rsm"));
        assert_eq!(trap_model_name(UNT_SAFETYWALL), None);
        // Trigger: the burst fires for explosive traps, not for holders.
        assert_eq!(trap_trigger_effect(UNT_FREEZINGTRAP), Some(EffectId::Freezing));
        assert_eq!(trap_trigger_effect(UNT_BLASTMINE), Some(EffectId::Blastminebomb));
        assert_eq!(trap_trigger_effect(UNT_ANKLESNARE), None);
        assert_eq!(trap_trigger_effect(UNT_SKIDTRAP), None);
    }
}
