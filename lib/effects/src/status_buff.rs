use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::effect_id::EffectId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuffEffect {
    pub body: &'static [EffectId],
}

pub fn buff_effect(efst: ClientEffectIcon) -> Option<BuffEffect> {
    use ClientEffectIcon as I;
    let body: &'static [EffectId] = match efst {
        I::Berserk => &[EffectId::Redbody],
        I::Marionette | I::MarionetteMaster => &[EffectId::Pinkbody],
        I::Steelbody => &[EffectId::Steelbody],
        I::Energycoat => &[EffectId::Energycoat],
        I::Assumptio => &[EffectId::Assumptio],
        I::Propertyundead => &[EffectId::Undeadbody],
        I::Lkconcentration => &[EffectId::Lkconcentration],
        I::NjBunsinjyutsu => &[EffectId::Bunsinjyutsu],
        I::Onehandquicken => &[EffectId::Twohandquicken],
        I::Spearquicken => &[EffectId::Spearquicken],
        I::Aurablade => &[EffectId::Aurablade, EffectId::Aurablade2],
        I::Autoguard => &[EffectId::Guard],
        I::Reflectshield => &[EffectId::Reflectshield],
        I::Defender => &[EffectId::Defender],
        I::CrShrink => &[EffectId::Shrink],
        I::Adrenaline | I::Adrenaline2 => &[EffectId::Hasteup],
        I::Overthrust | I::Overthrustmax => &[EffectId::Overthrust],
        I::Maximize => &[EffectId::Maxpower],
        I::Shout => &[EffectId::Loud],
        I::Meltdown => &[EffectId::Meltdown],
        I::Cartboost => &[EffectId::Cartboost],
        I::Magicpower => &[EffectId::Lightblade],
        I::Kaupe => &[EffectId::Bluebody],
        I::Kaite => &[EffectId::Reflectbody, EffectId::Bluebody],
        I::Kaahi => &[EffectId::Hated],
        I::Kaizel => &[EffectId::Kaizel],
        _ => return None,
    };
    Some(BuffEffect { body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_persistent_buffs_and_ignores_unmapped() {
        assert_eq!(
            buff_effect(ClientEffectIcon::Berserk),
            Some(BuffEffect {
                body: &[EffectId::Redbody]
            })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::Marionette),
            Some(BuffEffect {
                body: &[EffectId::Pinkbody]
            })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::MarionetteMaster),
            Some(BuffEffect {
                body: &[EffectId::Pinkbody]
            })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::Propertyundead),
            Some(BuffEffect {
                body: &[EffectId::Undeadbody]
            })
        );
        // Two-Hand Quicken is one-shot at cast (its EFST has no persistent aura in
        // the original); only One-Hand Quicken keeps a persistent aura.
        assert_eq!(buff_effect(ClientEffectIcon::Twohandquicken), None);
        assert_eq!(
            buff_effect(ClientEffectIcon::Onehandquicken),
            Some(BuffEffect {
                body: &[EffectId::Twohandquicken]
            })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::Spearquicken),
            Some(BuffEffect {
                body: &[EffectId::Spearquicken]
            })
        );
        // Endure flashes once at cast, not a full-duration aura.
        assert_eq!(buff_effect(ClientEffectIcon::Endure), None);
        assert_eq!(
            buff_effect(ClientEffectIcon::Overthrust),
            Some(BuffEffect {
                body: &[EffectId::Overthrust]
            })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::Autoguard),
            Some(BuffEffect {
                body: &[EffectId::Guard]
            })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::Aurablade),
            Some(BuffEffect {
                body: &[EffectId::Aurablade, EffectId::Aurablade2]
            })
        );
        assert_eq!(buff_effect(ClientEffectIcon::Provoke), None);
    }
}
