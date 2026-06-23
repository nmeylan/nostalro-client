//! EFST status → persistent body-buff effect mapping (`ZC_MSG_STATE_CHANGE`).
//!
//! A buff/special-body status the server toggles on an entity recolors the body
//! or lays a following aura, persisting until its `remain_ms` expires or an
//! off-packet clears it. The packet's `index` is an EFST code, decoded through
//! the server's own [`ClientEffectIcon`] enum so client and server numbering
//! can never drift.
//!
//! Only statuses whose visual **persists cleanly** for an arbitrary duration are
//! mapped here: flat body tints (Berserk/Marionette) and auras whose effect
//! holds steady once it ramps in (Steel Body, Energy Coat, Assumptio, Undead
//! property, LK Concentration, Bunsinjyutsu, the Quicken family). The short
//! *animated* auras
//! (Overthrust blur, Endure, Aura Blade, Reflect Shield, Soul Link) and the
//! fullscreen darken (Soul Link / SKE) are not mapped yet — they play a one-shot
//! animation that would flicker if held, and need per-effect looping support.

use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::effect_id::EffectId;

/// What a persistent status spawns on the affected entity. The effects are
/// body-attached, so they follow the entity and are killed together by the
/// status's owner key when it clears or expires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuffEffect {
    pub body: &'static [EffectId],
}

/// The persistent body visual for an EFST status, or `None` when the status has
/// no body visual we render (icon-only buffs, or the deferred animated auras).
pub fn buff_effect(efst: ClientEffectIcon) -> Option<BuffEffect> {
    use ClientEffectIcon as I;
    let body: &'static [EffectId] = match efst {
        // Flat body tints (P2a).
        I::Berserk => &[EffectId::Redbody],
        I::Marionette | I::MarionetteMaster => &[EffectId::Pinkbody],
        // Auras that hold steady once ramped (P2b).
        I::Steelbody => &[EffectId::Steelbody],
        I::Energycoat => &[EffectId::Energycoat],
        I::Assumptio => &[EffectId::Assumptio],
        I::Propertyundead => &[EffectId::Undeadbody],
        I::Lkconcentration => &[EffectId::Lkconcentration],
        I::NjBunsinjyutsu => &[EffectId::Bunsinjyutsu],
        // Quicken family — yellow body tint + looping twohand.str aura. One Hand
        // Quicken shares Two Hand Quicken's visual.
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
            Some(BuffEffect { body: &[EffectId::Redbody] })
        );
        // Both Marionette forms share the pink body.
        assert_eq!(
            buff_effect(ClientEffectIcon::Marionette),
            Some(BuffEffect { body: &[EffectId::Pinkbody] })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::MarionetteMaster),
            Some(BuffEffect { body: &[EffectId::Pinkbody] })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::Propertyundead),
            Some(BuffEffect { body: &[EffectId::Undeadbody] })
        );
        // Quicken family — Two Hand and One Hand share the same body visual.
        assert_eq!(
            buff_effect(ClientEffectIcon::Twohandquicken),
            Some(BuffEffect { body: &[EffectId::Twohandquicken] })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::Onehandquicken),
            Some(BuffEffect { body: &[EffectId::Twohandquicken] })
        );
        assert_eq!(
            buff_effect(ClientEffectIcon::Spearquicken),
            Some(BuffEffect { body: &[EffectId::Spearquicken] })
        );
        // An icon-only / deferred status maps to nothing — fire nothing.
        assert_eq!(buff_effect(ClientEffectIcon::Overthrust), None);
        assert_eq!(buff_effect(ClientEffectIcon::Provoke), None);
    }
}
