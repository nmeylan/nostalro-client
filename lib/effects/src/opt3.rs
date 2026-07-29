use models::enums::client_effect_icon::ClientEffectIcon;
use models::enums::effect_id::EffectId;

pub const OPT3_QUICKEN: i32 = 0x0000_0001;
pub const OPT3_OVERTHRUST: i32 = 0x0000_0002;
pub const OPT3_ENERGYCOAT: i32 = 0x0000_0004;
pub const OPT3_EXPLOSIONSPIRITS: i32 = 0x0000_0008;
pub const OPT3_STEELBODY: i32 = 0x0000_0010;
pub const OPT3_BLADESTOP: i32 = 0x0000_0020;
pub const OPT3_AURABLADE: i32 = 0x0000_0040;
pub const OPT3_BERSERK: i32 = 0x0000_0080;
pub const OPT3_LIGHTBLADE: i32 = 0x0000_0100;
pub const OPT3_MOON: i32 = 0x0000_0200;
pub const OPT3_MARIONETTE: i32 = 0x0000_0400;
pub const OPT3_ASSUMPTIO: i32 = 0x0000_0800;
pub const OPT3_WARM: i32 = 0x0000_1000;
pub const OPT3_KAITE: i32 = 0x0000_2000;
pub const OPT3_BUNSIN: i32 = 0x0000_4000;
pub const OPT3_SOULLINK: i32 = 0x0000_8000;
pub const OPT3_UNDEAD: i32 = 0x0001_0000;
pub const OPT3_CONTRACT: i32 = 0x0002_0000;

/// Body-change buffs are mutually exclusive: the highest priority one set wins and
/// the rest are ignored, so two body tints can never stack.
const BODY_CHANGE_ORDER: [i32; 4] = [OPT3_QUICKEN, OPT3_OVERTHRUST, OPT3_ENERGYCOAT, OPT3_BUNSIN];

const ALL_BITS: [i32; 18] = [
    OPT3_QUICKEN,
    OPT3_OVERTHRUST,
    OPT3_ENERGYCOAT,
    OPT3_EXPLOSIONSPIRITS,
    OPT3_STEELBODY,
    OPT3_BLADESTOP,
    OPT3_AURABLADE,
    OPT3_BERSERK,
    OPT3_LIGHTBLADE,
    OPT3_MOON,
    OPT3_MARIONETTE,
    OPT3_ASSUMPTIO,
    OPT3_WARM,
    OPT3_KAITE,
    OPT3_BUNSIN,
    OPT3_SOULLINK,
    OPT3_UNDEAD,
    OPT3_CONTRACT,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opt3Reaction {
    /// Relaunched for as long as the bit is set, despawned when it clears.
    pub aura: &'static [EffectId],
    /// Played once the moment the bit clears.
    pub on_clear: &'static [EffectId],
    /// Freezes the actor's motion and greys its sprite.
    pub grip: bool,
}

impl Opt3Reaction {
    const fn aura(aura: &'static [EffectId]) -> Self {
        Self {
            aura,
            on_clear: &[],
            grip: false,
        }
    }

    const fn aura_with_clear(aura: &'static [EffectId], on_clear: &'static [EffectId]) -> Self {
        Self {
            aura,
            on_clear,
            grip: false,
        }
    }

    const fn grip() -> Self {
        Self {
            aura: &[],
            on_clear: &[],
            grip: true,
        }
    }
}

/// The opt3 bit a status icon stands for. opt3 only ever arrives as a whole word on
/// spawn, so the icon is the one signal that tells us a single one of those bits
/// started or stopped.
pub fn opt3_bit_for_icon(icon: ClientEffectIcon) -> Option<i32> {
    use ClientEffectIcon as I;
    let bit = match icon {
        I::Twohandquicken | I::Onehandquicken | I::Spearquicken | I::Lkconcentration => {
            OPT3_QUICKEN
        }
        I::Overthrust | I::Overthrustmax | I::Swoo => OPT3_OVERTHRUST,
        I::Energycoat => OPT3_ENERGYCOAT,
        I::Explosionspirits => OPT3_EXPLOSIONSPIRITS,
        I::Steelbody => OPT3_STEELBODY,
        I::Bladestop => OPT3_BLADESTOP,
        I::Aurablade => OPT3_AURABLADE,
        I::Berserk => OPT3_BERSERK,
        I::Marionette | I::MarionetteMaster => OPT3_MARIONETTE,
        I::Assumptio => OPT3_ASSUMPTIO,
        I::SgSunWarm => OPT3_WARM,
        I::Kaite => OPT3_KAITE,
        I::NjBunsinjyutsu => OPT3_BUNSIN,
        I::Soullink => OPT3_SOULLINK,
        I::Propertyundead => OPT3_UNDEAD,
        _ => return None,
    };
    Some(bit)
}

pub fn player_opt3_reaction(bit: i32) -> Option<Opt3Reaction> {
    use EffectId as E;
    let reaction = match bit {
        OPT3_QUICKEN => Opt3Reaction::aura(&[E::Twohandquicken]),
        OPT3_OVERTHRUST => Opt3Reaction::aura(&[E::Overthrust]),
        OPT3_ENERGYCOAT => Opt3Reaction::aura(&[E::Energycoat]),
        OPT3_BUNSIN => Opt3Reaction::aura(&[E::Bunsinjyutsu]),
        OPT3_EXPLOSIONSPIRITS => Opt3Reaction::aura(&[E::Gumgang]),
        OPT3_STEELBODY => Opt3Reaction::aura(&[E::Steelbody]),
        OPT3_WARM => Opt3Reaction::aura(&[E::Doublegumgang, E::Redlightbody]),
        OPT3_KAITE => Opt3Reaction::aura(&[E::Reflectbody]),
        OPT3_SOULLINK => Opt3Reaction::aura(&[E::Asurabody]),
        OPT3_CONTRACT => Opt3Reaction::aura(&[E::M04]),
        OPT3_UNDEAD => Opt3Reaction::aura(&[E::Undeadbody]),
        OPT3_AURABLADE => Opt3Reaction::aura(&[E::Aurablade2]),
        OPT3_LIGHTBLADE => Opt3Reaction::aura(&[E::Lightblade]),
        OPT3_BERSERK => Opt3Reaction::aura(&[E::Redbody]),
        OPT3_MOON => Opt3Reaction::aura(&[E::Spherewind2]),
        OPT3_MARIONETTE => Opt3Reaction::aura(&[E::Pinkbody]),
        OPT3_ASSUMPTIO => Opt3Reaction::aura(&[E::Assumptio]),
        OPT3_BLADESTOP => Opt3Reaction::grip(),
        _ => return None,
    };
    Some(reaction)
}

pub fn monster_opt3_reaction(bit: i32) -> Option<Opt3Reaction> {
    use EffectId as E;
    let reaction = match bit {
        OPT3_OVERTHRUST => Opt3Reaction::aura_with_clear(&[E::Babybody], &[E::BabybodyBack]),
        OPT3_ENERGYCOAT => Opt3Reaction::aura(&[E::AsurabodyMonster]),
        OPT3_STEELBODY => Opt3Reaction::aura(&[E::Steelbody]),
        OPT3_UNDEAD => Opt3Reaction::aura_with_clear(&[E::Undeadbody], &[E::UndeadbodyDel]),
        OPT3_SOULLINK => Opt3Reaction::aura(&[E::DaSpace]),
        OPT3_BLADESTOP => Opt3Reaction::grip(),
        _ => return None,
    };
    Some(reaction)
}

pub fn opt3_bits(opt3: i32) -> Vec<i32> {
    let body_change = BODY_CHANGE_ORDER
        .iter()
        .copied()
        .find(|bit| opt3 & bit != 0);
    ALL_BITS
        .iter()
        .copied()
        .filter(|bit| opt3 & bit != 0)
        .filter(|bit| !BODY_CHANGE_ORDER.contains(bit) || Some(*bit) == body_change)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_change_buffs_do_not_stack() {
        assert_eq!(
            opt3_bits(OPT3_OVERTHRUST | OPT3_ENERGYCOAT | OPT3_STEELBODY),
            vec![OPT3_OVERTHRUST, OPT3_STEELBODY]
        );
        assert_eq!(
            opt3_bits(OPT3_ENERGYCOAT | OPT3_BUNSIN),
            vec![OPT3_ENERGYCOAT]
        );
    }

    /// Several icons share one bit, and every bit an icon can name must be one the
    /// aura table knows, or its aura could never be handed over.
    #[test]
    fn status_icons_collapse_onto_bits_that_have_a_reaction() {
        use ClientEffectIcon as I;
        for family in [
            [I::Twohandquicken, I::Onehandquicken, I::Lkconcentration],
            [I::Overthrust, I::Overthrustmax, I::Swoo],
            [I::Marionette, I::MarionetteMaster, I::MarionetteMaster],
        ] {
            let bits: Vec<i32> = family
                .iter()
                .map(|i| opt3_bit_for_icon(*i).expect("icon maps to a bit"))
                .collect();
            assert!(bits.windows(2).all(|w| w[0] == w[1]), "{bits:?}");
            assert!(player_opt3_reaction(bits[0]).is_some());
        }
        assert_eq!(opt3_bit_for_icon(I::Bladestop), Some(OPT3_BLADESTOP));
        assert_eq!(opt3_bit_for_icon(I::Poisonreact), None);
    }

    #[test]
    fn monsters_show_a_reduced_set() {
        assert!(player_opt3_reaction(OPT3_BERSERK).is_some());
        assert!(monster_opt3_reaction(OPT3_BERSERK).is_none());

        assert_eq!(
            monster_opt3_reaction(OPT3_ENERGYCOAT).unwrap().aura,
            &[EffectId::AsurabodyMonster]
        );
        assert_eq!(
            monster_opt3_reaction(OPT3_UNDEAD).unwrap().on_clear,
            &[EffectId::UndeadbodyDel]
        );
        assert!(monster_opt3_reaction(OPT3_BLADESTOP).unwrap().grip);
    }
}
