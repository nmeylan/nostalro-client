use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::effect_id::EffectId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Visual,
    PushCart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusReaction {
    /// Re-launched for the whole duration and despawned when the status ends.
    pub aura: &'static [EffectId],
    /// Played once the moment the status turns on.
    pub on_activate: &'static [EffectId],
    /// Played once the moment the status turns off.
    pub on_deactivate: &'static [EffectId],
    /// Non-visual consequence routed to a dedicated handler.
    pub kind: StatusKind,
    /// Darkens the world, for the bearer's eyes only.
    pub night_filter: bool,
}

impl StatusReaction {
    const fn new() -> Self {
        Self {
            aura: &[],
            on_activate: &[],
            on_deactivate: &[],
            kind: StatusKind::Visual,
            night_filter: false,
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
        I::Overthrust | I::Overthrustmax => StatusReaction::aura(&[E::Overthrust]),
        I::Magicpower => StatusReaction::aura(&[E::Lightblade]),
        I::Aurablade => StatusReaction::aura(&[E::Aurablade2]),
        I::Kaite => StatusReaction::aura(&[E::Reflectbody]),
        I::Soullink => StatusReaction::aura(&[E::Asurabody]).with_night_filter(),
        I::Explosionspirits => StatusReaction::aura(&[E::Gumgang]),
        I::SgSunWarm => StatusReaction::aura(&[E::Doublegumgang, E::Redlightbody]),
        I::Mindbreaker => StatusReaction::on_activate(&[E::Magiccrasher2]),
        I::Ting => StatusReaction::on_activate(&[E::Quakebody]),
        I::Run => StatusReaction::on_deactivate(&[E::Stopeffect]),
        I::OnPushCart => StatusReaction::kind(StatusKind::PushCart),
        _ => return None,
    };
    Some(reaction)
}

/// Statuses the shared icon enum has no variant for. Moon rides the Star
/// Gladiator's spirit sphere, and the Moon/Star warmth auras look the same as
/// the Sun one.
pub const EFST_MOON: i16 = 123;
pub const EFST_SKE: i16 = 160;
pub const EFST_SG_MOON_WARM: i16 = 166;
pub const EFST_SG_STAR_WARM: i16 = 167;

/// [`status_reaction`] for statuses addressed by their raw id.
pub fn status_reaction_by_efst(efst: i16) -> Option<StatusReaction> {
    use EffectId as E;
    match efst {
        EFST_MOON => Some(StatusReaction::aura(&[E::Spherewind2])),
        EFST_SKE => Some(StatusReaction::night_filter()),
        EFST_SG_MOON_WARM | EFST_SG_STAR_WARM => Some(StatusReaction::aura(&[
            E::Doublegumgang,
            E::Redlightbody,
        ])),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_gladiator_and_monk_spheres_are_persistent_auras() {
        use ClientEffectIcon as I;
        use EffectId as E;

        assert_eq!(
            status_reaction(I::Explosionspirits).unwrap().aura,
            &[E::Gumgang]
        );
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
    fn only_full_duration_auras_show_a_persistent_world_aura() {
        use ClientEffectIcon as I;

        let persistent: &[(I, &[EffectId])] = &[
            (I::Berserk, &[EffectId::Redbody]),
            (I::Steelbody, &[EffectId::Steelbody]),
            (I::Overthrust, &[EffectId::Overthrust]),
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
            status_reaction(I::Run).unwrap().on_deactivate,
            &[EffectId::Stopeffect]
        );
        assert_eq!(
            status_reaction(I::OnPushCart).unwrap().kind,
            StatusKind::PushCart
        );
    }
}
