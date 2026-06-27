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
        I::Twohandquicken | I::Onehandquicken => &[EffectId::Twohandquicken],
        I::Spearquicken => &[EffectId::Spearquicken],
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
        assert_eq!(
            buff_effect(ClientEffectIcon::Twohandquicken),
            Some(BuffEffect {
                body: &[EffectId::Twohandquicken]
            })
        );
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
        assert_eq!(buff_effect(ClientEffectIcon::Overthrust), None);
        assert_eq!(buff_effect(ClientEffectIcon::Provoke), None);
    }
}
