use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::effect_id::EffectId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuffEffect {
    pub body: &'static [EffectId],
}

/// The status effects that show a body aura for the whole duration the status is
/// active. Every other buff is flashed once at cast (see
/// [`crate::skill_effects::caster_skill_effects`] /
/// [`crate::skill_effects::target_skill_effects`]) and only its status icon
/// persists — not a world aura.
pub fn buff_effect(efst: ClientEffectIcon) -> Option<BuffEffect> {
    use ClientEffectIcon as I;
    let body: &'static [EffectId] = match efst {
        I::Berserk => &[EffectId::Redbody],
        I::Steelbody => &[EffectId::Steelbody],
        I::Energycoat => &[EffectId::Energycoat],
        I::Assumptio => &[EffectId::Assumptio],
        I::Propertyundead => &[EffectId::Undeadbody],
        I::Lkconcentration => &[EffectId::Lkconcentration],
        I::NjBunsinjyutsu => &[EffectId::Bunsinjyutsu],
        I::Onehandquicken => &[EffectId::Twohandquicken],
        I::Spearquicken => &[EffectId::Spearquicken],
        I::Overthrust | I::Overthrustmax => &[EffectId::Overthrust],
        I::Magicpower => &[EffectId::Lightblade],
        I::Aurablade => &[EffectId::Aurablade2],
        I::Kaite => &[EffectId::Reflectbody],
        I::Soullink => &[EffectId::Asurabody],
        I::SgSunWarm => &[EffectId::Doublegumgang, EffectId::Redlightbody],
        _ => return None,
    };
    Some(BuffEffect { body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_full_duration_auras_map_others_are_one_shot() {
        use ClientEffectIcon as I;

        // Genuinely persistent auras the original re-launches for the whole status.
        let persistent: &[(I, &[EffectId])] = &[
            (I::Berserk, &[EffectId::Redbody]),
            (I::Steelbody, &[EffectId::Steelbody]),
            (I::Overthrust, &[EffectId::Overthrust]),
            (I::Spearquicken, &[EffectId::Spearquicken]),
            (I::Onehandquicken, &[EffectId::Twohandquicken]),
            (I::Magicpower, &[EffectId::Lightblade]),
            (I::Soullink, &[EffectId::Asurabody]),
        ];
        for &(efst, body) in persistent {
            assert_eq!(buff_effect(efst), Some(BuffEffect { body }));
        }

        // Split buffs keep only the persistent half; the burst is a one-shot at cast.
        assert_eq!(
            buff_effect(I::Aurablade),
            Some(BuffEffect {
                body: &[EffectId::Aurablade2]
            })
        );
        assert_eq!(
            buff_effect(I::Kaite),
            Some(BuffEffect {
                body: &[EffectId::Reflectbody]
            })
        );
        assert_eq!(
            buff_effect(I::SgSunWarm),
            Some(BuffEffect {
                body: &[EffectId::Doublegumgang, EffectId::Redlightbody]
            })
        );

        // One-shot-at-cast buffs (icon persists, no world aura): not mapped here.
        for efst in [
            I::Marionette,
            I::MarionetteMaster,
            I::Autoguard,
            I::Reflectshield,
            I::Defender,
            I::CrShrink,
            I::Adrenaline,
            I::Adrenaline2,
            I::Maximize,
            I::Shout,
            I::Meltdown,
            I::Cartboost,
            I::Kaupe,
            I::Kaahi,
            I::Kaizel,
            I::Twohandquicken,
            I::Endure,
            I::Provoke,
        ] {
            assert_eq!(buff_effect(efst), None, "{efst:?} is one-shot, not an aura");
        }
    }
}
