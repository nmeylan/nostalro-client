//! Maps a consumable item id to the visual effect the original game plays when
//! it is used. The authoritative source (`effectlist.xml`) is not present in the
//! classic GRF, so this reconstructs the original client's hardcoded item→effect
//! switch (basic potions + the named buff consumables). Items without their own
//! use effect return `None`. Extend as more consumables are identified.

use models::enums::effect_id::EffectId;

/// The effect to play on the entity that used consumable `item_id`, or `None`
/// for consumables that have no use effect in the original game.
pub fn consumable_use_effect(item_id: u32) -> Option<EffectId> {
    let id = match item_id {
        // Red / orange / yellow / white healing potions (+ their boxed 105xx ids).
        501 | 502 | 503 | 504 | 513 | 10501 | 10502 | 10503 | 10504 | 10513 => EffectId::Potion1,
        // Condensed red / yellow.
        509 | 510 | 10509 | 10510 => EffectId::Potion2,
        // Blue / SP / condensed white family.
        505 | 506 | 507 | 508 | 514 | 10505 | 10506 | 10507 | 10508 | 10514 => EffectId::Potion3,
        512 | 515 | 10512 | 10515 => EffectId::Potion4,
        // Concentration potion.
        645 | 14509 => EffectId::PotionCon,
        // Awakening potion.
        656 => EffectId::Potion,
        // Berserk potion.
        657 | 684 | 14511 => EffectId::PotionBerserk,
        // Speed potion.
        12016 => EffectId::Itemfast,
        _ => return None,
    };
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_consumables_and_ignores_the_rest() {
        assert_eq!(consumable_use_effect(501), Some(EffectId::Potion1)); // red potion
        assert_eq!(consumable_use_effect(505), Some(EffectId::Potion3)); // white potion
        assert_eq!(consumable_use_effect(645), Some(EffectId::PotionCon)); // concentration
        assert_eq!(consumable_use_effect(657), Some(EffectId::PotionBerserk)); // berserk
        assert_eq!(consumable_use_effect(12016), Some(EffectId::Itemfast)); // speed potion
        assert_eq!(consumable_use_effect(909), None); // jellopy — no use effect
    }
}
