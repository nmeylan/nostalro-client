use models::enums::EnumWithNumberValue;
use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::effect_id::EffectId;

use crate::effects::body_buff;
use crate::sfx::SfxPos;

/// A wave the status plays the moment it turns on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusSound {
    pub wave: &'static str,
    pub pos: SfxPos,
    /// Heard only by the status bearer.
    pub local_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Visual,
    PushCart,
    /// Its overlay is picked from the bearer's own skill level, so the client
    /// resolves the effect instead of the table.
    DevilBlind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusReaction {
    /// Re-launched for the whole duration and despawned when the status ends.
    pub aura: &'static [EffectId],
    /// Which flavour the aura spawns with, for effects that have more than one.
    pub aura_count: Option<u8>,
    /// Played once the moment the status turns on.
    pub on_activate: &'static [EffectId],
    /// Played once the moment the status turns off.
    pub on_deactivate: &'static [EffectId],
    /// Non-visual consequence routed to a dedicated handler.
    pub kind: StatusKind,
    /// Darkens the world, for the bearer's eyes only.
    pub night_filter: bool,
    /// Ripples the whole screen, for the bearer's eyes only.
    pub screen_ripple: bool,
    pub on_activate_sound: Option<StatusSound>,
}

impl StatusReaction {
    const fn new() -> Self {
        Self {
            aura: &[],
            aura_count: None,
            on_activate: &[],
            on_deactivate: &[],
            kind: StatusKind::Visual,
            night_filter: false,
            screen_ripple: false,
            on_activate_sound: None,
        }
    }

    const fn sound(wave: &'static str, pos: SfxPos, local_only: bool) -> Self {
        Self {
            on_activate_sound: Some(StatusSound {
                wave,
                pos,
                local_only,
            }),
            ..Self::new()
        }
    }

    const fn with_sound(self, wave: &'static str, pos: SfxPos, local_only: bool) -> Self {
        Self {
            on_activate_sound: Some(StatusSound {
                wave,
                pos,
                local_only,
            }),
            ..self
        }
    }

    const fn screen_ripple() -> Self {
        Self {
            screen_ripple: true,
            ..Self::new()
        }
    }

    const fn night_filter() -> Self {
        Self {
            night_filter: true,
            ..Self::new()
        }
    }

    const fn with_night_filter(self) -> Self {
        Self {
            night_filter: true,
            ..self
        }
    }

    const fn aura(aura: &'static [EffectId]) -> Self {
        Self {
            aura,
            ..Self::new()
        }
    }

    const fn aura_with_count(aura: &'static [EffectId], count: u8) -> Self {
        Self {
            aura_count: Some(count),
            ..Self::aura(aura)
        }
    }

    const fn on_activate(ids: &'static [EffectId]) -> Self {
        Self {
            on_activate: ids,
            ..Self::new()
        }
    }

    const fn on_deactivate(ids: &'static [EffectId]) -> Self {
        Self {
            on_deactivate: ids,
            ..Self::new()
        }
    }

    const fn kind(kind: StatusKind) -> Self {
        Self {
            kind,
            ..Self::new()
        }
    }
}

/// The client-side consequences of a status effect changing. `aura` effects show for the
/// whole duration; every other buff is flashed once at cast (see
/// [`crate::skill_effects::caster_skill_effects`] /
/// [`crate::skill_effects::target_skill_effects`]) and only its status icon persists.
pub fn status_reaction(efst: ClientEffectIcon) -> Option<StatusReaction> {
    use ClientEffectIcon as I;
    use EffectId as E;
    let reaction = match efst {
        I::Berserk => StatusReaction::aura(&[E::Redbody]),
        I::Steelbody => StatusReaction::aura(&[E::Steelbody]),
        I::Energycoat => StatusReaction::aura(&[E::Energycoat]),
        I::Assumptio => StatusReaction::aura(&[E::Assumptio]),
        I::Propertyundead => StatusReaction::aura(&[E::Undeadbody]),
        I::Lkconcentration => StatusReaction::aura(&[E::Lkconcentration]),
        I::NjBunsinjyutsu => StatusReaction::aura(&[E::Bunsinjyutsu]),
        I::Twohandquicken | I::Onehandquicken => StatusReaction::aura(&[E::Twohandquicken]),
        I::Spearquicken => StatusReaction::aura(&[E::Spearquicken]),
        I::Overthrust | I::Overthrustmax => StatusReaction::aura(&[E::Makeblur]),
        I::Magicpower => StatusReaction::aura(&[E::Lightblade]),
        I::Aurablade => StatusReaction::aura(&[E::Aurablade2]),
        I::Kaite => StatusReaction::aura(&[E::Reflectbody]),
        I::Soullink => StatusReaction::aura(&[E::Asurabody]).with_night_filter(),
        I::Explosionspirits => StatusReaction::aura_with_count(
            &[E::Gumgang, E::Makeblur],
            body_buff::BLUR_EXPLOSION_SPIRITS,
        ),
        I::SgSunWarm => StatusReaction::aura(&[E::Doublegumgang, E::Redlightbody]),
        I::Mindbreaker => StatusReaction::on_activate(&[E::Magiccrasher2]),
        I::Ting => StatusReaction::on_activate(&[E::Quakebody]).with_sound(
            "effect\\t_벽튕김.wav",
            SfxPos::World,
            false,
        ),
        I::Chasewalk2 => StatusReaction::sound("lava_golem_move.wav", SfxPos::Ui(0.0), true),
        I::Run => StatusReaction::on_deactivate(&[E::Stopeffect]),
        I::Illusion => StatusReaction::screen_ripple(),
        I::OnPushCart => StatusReaction::kind(StatusKind::PushCart),
        _ => return None,
    };
    Some(reaction)
}

/// Statuses the shared icon enum has no variant for. Moon rides the Star
/// Gladiator's spirit sphere, and the Moon/Star warmth auras look the same as
/// the Sun one.
pub const EFST_MOON: i16 = 123;

/// Eclipse (the Star Gladiator's Demon of the Sun, Moon and Stars).
pub const EFST_DEVIL1: i16 = 152;
pub const EFST_SKE: i16 = 160;
pub const EFST_SG_MOON_WARM: i16 = 166;
pub const EFST_SG_STAR_WARM: i16 = 167;

/// [`status_reaction`] for statuses addressed by their raw id.
/// The auras a status keeps alive for its whole duration, resolved from either
/// reaction table. They outlive an effect-queue wipe, so a map change has to
/// re-launch them from the statuses still running.
pub fn persistent_aura(efst: i16) -> Option<(&'static [EffectId], Option<u8>)> {
    status_reaction_by_efst(efst)
        .or_else(|| {
            ClientEffectIcon::try_from_value(efst as usize)
                .ok()
                .and_then(status_reaction)
        })
        .map(|reaction| (reaction.aura, reaction.aura_count))
        .filter(|(aura, _)| !aura.is_empty())
}

pub fn status_reaction_by_efst(efst: i16) -> Option<StatusReaction> {
    use EffectId as E;
    match efst {
        EFST_MOON => Some(StatusReaction::aura(&[E::Spherewind2])),
        EFST_DEVIL1 => Some(StatusReaction::kind(StatusKind::DevilBlind).with_sound(
            "effect\\_blind.wav",
            SfxPos::Ui(-100.0),
            true,
        )),
        EFST_SKE => Some(StatusReaction::night_filter()),
        EFST_SG_MOON_WARM | EFST_SG_STAR_WARM => {
            Some(StatusReaction::aura(&[E::Doublegumgang, E::Redlightbody]))
        }
        _ => None,
    }
}

pub const DEVIL_BLIND_MAX_LEVEL: u8 = 10;

/// The blackout overlay for a Demon of the Sun, Moon and Stars of `level`.
pub fn devil_blind_effect(level: u8) -> Option<EffectId> {
    use EffectId as E;
    Some(match level {
        1 => E::Devil1,
        2 => E::Devil2,
        3 => E::Devil3,
        4 => E::Devil4,
        5 => E::Devil5,
        6 => E::Devil6,
        7 => E::Devil7,
        8 => E::Devil8,
        9 => E::Devil9,
        10 => E::Devil10,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devil_blind_effect_covers_every_level() {
        let ids: Vec<_> = (1..=DEVIL_BLIND_MAX_LEVEL)
            .map(|level| devil_blind_effect(level).unwrap())
            .collect();
        assert_eq!(ids.first(), Some(&EffectId::Devil1));
        assert_eq!(ids.last(), Some(&EffectId::Devil10));
        assert!(devil_blind_effect(0).is_none());
        assert!(devil_blind_effect(DEVIL_BLIND_MAX_LEVEL + 1).is_none());
        assert_eq!(
            status_reaction_by_efst(EFST_DEVIL1).unwrap().kind,
            StatusKind::DevilBlind
        );
    }

    #[test]
    fn star_gladiator_and_monk_spheres_are_persistent_auras() {
        use ClientEffectIcon as I;
        use EffectId as E;

        let fury = status_reaction(I::Explosionspirits).unwrap();
        assert_eq!(fury.aura, &[E::Gumgang, E::Makeblur]);
        assert_eq!(fury.aura_count, Some(body_buff::BLUR_EXPLOSION_SPIRITS));
        assert_eq!(
            status_reaction_by_efst(EFST_MOON).unwrap().aura,
            &[E::Spherewind2]
        );
        // The three warmth auras look the same, whichever heavenly body it is.
        let sun = status_reaction(I::SgSunWarm).unwrap().aura;
        assert_eq!(sun, &[E::Doublegumgang, E::Redlightbody]);
        for efst in [EFST_SG_MOON_WARM, EFST_SG_STAR_WARM] {
            assert_eq!(status_reaction_by_efst(efst).unwrap().aura, sun);
        }
        assert!(status_reaction_by_efst(EFST_MOON + 1).is_none());
    }

    #[test]
    fn only_ske_and_soul_link_darken_the_world() {
        use ClientEffectIcon as I;

        let soullink = status_reaction(I::Soullink).unwrap();
        assert!(soullink.night_filter);
        assert_eq!(soullink.aura, &[EffectId::Asurabody]);

        let ske = status_reaction_by_efst(EFST_SKE).unwrap();
        assert!(ske.night_filter);
        assert!(ske.aura.is_empty());

        assert!(!status_reaction(I::Berserk).unwrap().night_filter);
        assert!(!status_reaction_by_efst(EFST_MOON).unwrap().night_filter);
    }

    #[test]
    fn only_illusion_ripples_the_screen() {
        use ClientEffectIcon as I;

        let illusion = status_reaction(I::Illusion).unwrap();
        assert!(illusion.screen_ripple);
        assert!(illusion.aura.is_empty());
        assert!(!illusion.night_filter);

        assert!(!status_reaction(I::Soullink).unwrap().screen_ripple);
    }

    #[test]
    fn only_full_duration_auras_show_a_persistent_world_aura() {
        use ClientEffectIcon as I;

        let persistent: &[(I, &[EffectId])] = &[
            (I::Berserk, &[EffectId::Redbody]),
            (I::Steelbody, &[EffectId::Steelbody]),
            (I::Overthrust, &[EffectId::Makeblur]),
            (I::Spearquicken, &[EffectId::Spearquicken]),
            (I::Onehandquicken, &[EffectId::Twohandquicken]),
            (I::Twohandquicken, &[EffectId::Twohandquicken]),
            (I::Magicpower, &[EffectId::Lightblade]),
            (I::Soullink, &[EffectId::Asurabody]),
        ];
        for &(efst, aura) in persistent {
            assert_eq!(status_reaction(efst).unwrap().aura, aura);
        }

        // Split buffs keep only the persistent half; the burst is a one-shot at cast.
        assert_eq!(
            status_reaction(I::Aurablade).unwrap().aura,
            &[EffectId::Aurablade2]
        );
        assert_eq!(
            status_reaction(I::SgSunWarm).unwrap().aura,
            &[EffectId::Doublegumgang, EffectId::Redlightbody]
        );

        // One-shot-at-cast buffs (icon persists, no world aura): no reaction here.
        for efst in [
            I::Marionette,
            I::MarionetteMaster,
            I::Autoguard,
            I::Reflectshield,
            I::Defender,
            I::CrShrink,
            I::Adrenaline,
            I::Maximize,
            I::Provoke,
        ] {
            assert!(
                status_reaction(efst).is_none(),
                "{efst:?} is one-shot, not an aura"
            );
        }
    }

    #[test]
    fn persistent_aura_reaches_both_reaction_tables_and_skips_auraless_statuses() {
        use ClientEffectIcon as I;

        assert_eq!(
            persistent_aura(I::Berserk.value() as i16),
            Some((&[EffectId::Redbody][..], None))
        );
        assert_eq!(
            persistent_aura(I::Twohandquicken.value() as i16),
            Some((&[EffectId::Twohandquicken][..], None))
        );
        assert_eq!(
            persistent_aura(EFST_MOON),
            Some((&[EffectId::Spherewind2][..], None))
        );
        assert_eq!(persistent_aura(EFST_SKE), None);
        assert_eq!(persistent_aura(I::Adrenaline.value() as i16), None);
    }

    #[test]
    fn transition_bursts_and_subsystems_are_declared_not_auras() {
        use ClientEffectIcon as I;

        let mindbreaker = status_reaction(I::Mindbreaker).unwrap();
        assert_eq!(mindbreaker.on_activate, &[EffectId::Magiccrasher2]);
        assert!(mindbreaker.aura.is_empty());

        assert_eq!(
            status_reaction(I::Ting).unwrap().on_activate,
            &[EffectId::Quakebody]
        );
        assert_eq!(
            status_reaction(I::Chasewalk2).unwrap().on_activate_sound,
            Some(StatusSound {
                wave: "lava_golem_move.wav",
                pos: SfxPos::Ui(0.0),
                local_only: true,
            })
        );
        assert!(
            status_reaction_by_efst(EFST_DEVIL1)
                .unwrap()
                .on_activate_sound
                .is_some_and(|s| s.local_only && s.pos == SfxPos::Ui(-100.0))
        );
        assert_eq!(
            status_reaction(I::Run).unwrap().on_deactivate,
            &[EffectId::Stopeffect]
        );
        assert_eq!(
            status_reaction(I::OnPushCart).unwrap().kind,
            StatusKind::PushCart
        );
    }
}
