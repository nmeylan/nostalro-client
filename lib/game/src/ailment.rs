//! Status-ailment appearance and incapacitation rules, decoded from the
//! `body_state` (opt1, a single enum) and `health_state` (opt2, a bitmask)
//! fields that ride every entity-update packet. Mirrors the original game's
//! `SetAttrState`: a fixed body recolor, an animation freeze, persistent
//! overlay sprites, and movement blocking.

// opt1 — single value (rathena `e_sc_opt1`)
pub const OPT1_STONE: i16 = 1; // fully petrified
pub const OPT1_FREEZE: i16 = 2;
pub const OPT1_STUN: i16 = 3;
pub const OPT1_SLEEP: i16 = 4;
pub const OPT1_STONEWAIT: i16 = 6; // petrifying — still mobile

// opt2 — bitmask (rathena `e_sc_opt2`)
pub const OPT2_POISON: i16 = 0x0001;
pub const OPT2_CURSE: i16 = 0x0002;
pub const OPT2_BLIND: i16 = 0x0010;
pub const OPT2_ANGELUS: i16 = 0x0020;
pub const OPT2_BLEEDING: i16 = 0x0040;
pub const OPT2_DEADLYPOISON: i16 = 0x0080;

/// A persistent head-anchored status overlay whose sprite ships in the classic
/// GRF. Silence and the freeze ice-block are absent from it, so they have no
/// overlay (freeze still reads from its body tint + frozen pose).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AilmentOverlay {
    Stun,
    Sleep,
    Curse,
    Angelus,
}

impl AilmentOverlay {
    pub const ALL: [AilmentOverlay; 4] = [
        AilmentOverlay::Stun,
        AilmentOverlay::Sleep,
        AilmentOverlay::Curse,
        AilmentOverlay::Angelus,
    ];

    /// GRF sprite/act base path (without extension) and the ACT action to play.
    /// The original game billboards these above the actor's head.
    pub fn sprite(self) -> (&'static str, usize) {
        match self {
            AilmentOverlay::Stun => ("data/sprite/이팩트/status-stun", 0),
            AilmentOverlay::Sleep => ("data/sprite/이팩트/status-sleep", 0),
            AilmentOverlay::Curse => ("data/sprite/이팩트/status-curse", 0),
            AilmentOverlay::Angelus => ("data/sprite/이팩트/msg", 1),
        }
    }
}

/// Per-frame appearance state derived from the ailment fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AilmentVisual {
    /// Fixed body recolor (RGB multiply); `None` keeps the natural color.
    pub tint: Option<[u8; 3]>,
    /// Pause the sprite animation, holding the current pose.
    pub motion_locked: bool,
    /// Fullscreen darken — applies only when the local player holds the status.
    pub local_fullscreen_blind: bool,
}

pub fn ailment_visual(body_state: i16, health_state: i16) -> AilmentVisual {
    AilmentVisual {
        tint: ailment_tint(body_state, health_state),
        motion_locked: matches!(body_state, OPT1_FREEZE | OPT1_STONE),
        local_fullscreen_blind: health_state & OPT2_BLIND != 0,
    }
}

/// The single fixed body ARGB. The original issues sequential color writes and
/// the last one wins, so health-state colors override body-state ones and the
/// order is Curse > Bleeding > Poison > Freeze > Stone > StoneWait.
fn ailment_tint(body_state: i16, health_state: i16) -> Option<[u8; 3]> {
    if health_state & OPT2_CURSE != 0 {
        return Some([200, 50, 50]);
    }
    if health_state & OPT2_BLEEDING != 0 {
        return Some([200, 32, 0]);
    }
    if health_state & (OPT2_POISON | OPT2_DEADLYPOISON) != 0 {
        return Some([0, 192, 64]);
    }
    match body_state {
        OPT1_FREEZE => Some([0, 128, 255]),
        OPT1_STONE => Some([64, 64, 64]),
        OPT1_STONEWAIT => Some([128, 128, 128]),
        _ => None,
    }
}

/// Walking is blocked by an opt1 ailment — but not during the petrify delay
/// (`STONEWAIT`), and never by an opt2 bit. The server is authoritative; this
/// gates the client's optimistic move prediction so it stops mis-predicting.
pub fn movement_blocked(body_state: i16) -> bool {
    matches!(
        body_state,
        OPT1_STONE | OPT1_FREEZE | OPT1_STUN | OPT1_SLEEP
    )
}

/// The persistent overlays that should be active for the state. Several can
/// apply at once (one body-state overlay plus the independent opt2 ones).
pub fn ailment_overlays(body_state: i16, health_state: i16) -> Vec<AilmentOverlay> {
    let mut out = Vec::new();
    match body_state {
        OPT1_STUN => out.push(AilmentOverlay::Stun),
        OPT1_SLEEP => out.push(AilmentOverlay::Sleep),
        _ => {}
    }
    if health_state & OPT2_CURSE != 0 {
        out.push(AilmentOverlay::Curse);
    }
    if health_state & OPT2_ANGELUS != 0 {
        out.push(AilmentOverlay::Angelus);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ailment_visual_tint_precedence_and_motion_lock() {
        // Freeze: blue + motion locked.
        let v = ailment_visual(OPT1_FREEZE, 0);
        assert_eq!(v.tint, Some([0, 128, 255]));
        assert!(v.motion_locked);

        // Poison bit: green, no lock.
        let v = ailment_visual(0, OPT2_POISON);
        assert_eq!(v.tint, Some([0, 192, 64]));
        assert!(!v.motion_locked);

        // Poison + Bleeding + Curse all set: Curse wins.
        let v = ailment_visual(0, OPT2_POISON | OPT2_BLEEDING | OPT2_CURSE);
        assert_eq!(v.tint, Some([200, 50, 50]));

        // Freeze body + Poison health: green wins (health over body), but the
        // freeze still locks motion.
        let v = ailment_visual(OPT1_FREEZE, OPT2_POISON);
        assert_eq!(v.tint, Some([0, 192, 64]));
        assert!(v.motion_locked);

        // StoneWait: gray, NOT motion locked (petrify delay still animates).
        let v = ailment_visual(OPT1_STONEWAIT, 0);
        assert_eq!(v.tint, Some([128, 128, 128]));
        assert!(!v.motion_locked);
    }

    #[test]
    fn movement_blocked_excludes_stonewait_and_opt2() {
        assert!(movement_blocked(OPT1_STONE));
        assert!(movement_blocked(OPT1_FREEZE));
        assert!(movement_blocked(OPT1_STUN));
        assert!(movement_blocked(OPT1_SLEEP));
        assert!(!movement_blocked(OPT1_STONEWAIT)); // can move while petrifying
        assert!(!movement_blocked(0)); // normal
    }

    #[test]
    fn overlays_combine_body_and_health_states() {
        assert_eq!(ailment_overlays(OPT1_STUN, 0), vec![AilmentOverlay::Stun]);
        // Stun body + Curse + Angelus health -> three overlays.
        assert_eq!(
            ailment_overlays(OPT1_STUN, OPT2_CURSE | OPT2_ANGELUS),
            vec![
                AilmentOverlay::Stun,
                AilmentOverlay::Curse,
                AilmentOverlay::Angelus
            ]
        );
        // Blind alone produces no head overlay (it is the fullscreen wash).
        assert!(ailment_overlays(0, OPT2_BLIND).is_empty());
    }
}
