// opt1 — single value (rathena `e_sc_opt1`)
pub const OPT1_STONE: i16 = 1;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AilmentOverlay {
    Stun,
    Sleep,
    Curse,
    Freeze,
}

impl AilmentOverlay {
    pub const ALL: [AilmentOverlay; 4] = [
        AilmentOverlay::Stun,
        AilmentOverlay::Sleep,
        AilmentOverlay::Curse,
        AilmentOverlay::Freeze,
    ];

    pub fn sprite(self) -> (&'static str, usize) {
        match self {
            AilmentOverlay::Stun => ("data/sprite/이팩트/status-stun", 0),
            AilmentOverlay::Sleep => ("data/sprite/이팩트/status-sleep", 0),
            AilmentOverlay::Curse => ("data/sprite/이팩트/status-curse", 0),
            AilmentOverlay::Freeze => ("data/sprite/이팩트/얼음땡", 0),
        }
    }

    /// True for overlays that encase the body at its ground origin; false for
    /// the head-top billboards (stun/sleep/curse).
    pub fn on_body(self) -> bool {
        matches!(self, AilmentOverlay::Freeze)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AilmentVisual {
    pub tint: Option<[u8; 3]>,
    pub motion_locked: bool,
    /// Applies only when the local player holds the status.
    pub local_fullscreen_blind: bool,
}

pub fn ailment_visual(body_state: i16, health_state: i16, rooted: bool) -> AilmentVisual {
    AilmentVisual {
        tint: ailment_tint(body_state, health_state, rooted),
        motion_locked: rooted || matches!(body_state, OPT1_FREEZE | OPT1_STONE),
        local_fullscreen_blind: health_state & OPT2_BLIND != 0,
    }
}

/// Health-state colors override body-state ones; order: Curse > Bleeding > Poison > Freeze > Stone > StoneWait > Root.
fn ailment_tint(body_state: i16, health_state: i16, rooted: bool) -> Option<[u8; 3]> {
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
        _ if rooted => Some([64, 64, 64]),
        _ => None,
    }
}

/// STONEWAIT does not block movement; opt2 bits never do. Root (Blade Stop)
/// immobilizes both bound actors.
pub fn movement_blocked(body_state: i16, rooted: bool) -> bool {
    rooted
        || matches!(
            body_state,
            OPT1_STONE | OPT1_FREEZE | OPT1_STUN | OPT1_SLEEP
        )
}

pub fn ailment_overlays(body_state: i16, health_state: i16) -> Vec<AilmentOverlay> {
    let mut out = Vec::new();
    match body_state {
        OPT1_STUN => out.push(AilmentOverlay::Stun),
        OPT1_SLEEP => out.push(AilmentOverlay::Sleep),
        OPT1_FREEZE => out.push(AilmentOverlay::Freeze),
        _ => {}
    }
    if health_state & OPT2_CURSE != 0 {
        out.push(AilmentOverlay::Curse);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ailment_visual_tint_precedence_and_motion_lock() {
        let v = ailment_visual(OPT1_FREEZE, 0, false);
        assert_eq!(v.tint, Some([0, 128, 255]));
        assert!(v.motion_locked);

        let v = ailment_visual(0, OPT2_POISON, false);
        assert_eq!(v.tint, Some([0, 192, 64]));
        assert!(!v.motion_locked);

        let v = ailment_visual(0, OPT2_POISON | OPT2_BLEEDING | OPT2_CURSE, false);
        assert_eq!(v.tint, Some([200, 50, 50]));

        let v = ailment_visual(OPT1_FREEZE, OPT2_POISON, false);
        assert_eq!(v.tint, Some([0, 192, 64]));
        assert!(v.motion_locked);

        let v = ailment_visual(OPT1_STONEWAIT, 0, false);
        assert_eq!(v.tint, Some([128, 128, 128]));
        assert!(!v.motion_locked);
    }

    #[test]
    fn root_darkens_and_locks_but_yields_to_a_stronger_status() {
        let v = ailment_visual(0, 0, true);
        assert_eq!(v.tint, Some([64, 64, 64]));
        assert!(v.motion_locked);

        // A real status tint still wins over Root's dark grey.
        let v = ailment_visual(0, OPT2_POISON, true);
        assert_eq!(v.tint, Some([0, 192, 64]));
        assert!(v.motion_locked, "still immobile while rooted");
    }

    #[test]
    fn movement_blocked_excludes_stonewait_and_opt2() {
        assert!(movement_blocked(OPT1_STONE, false));
        assert!(movement_blocked(OPT1_FREEZE, false));
        assert!(movement_blocked(OPT1_STUN, false));
        assert!(movement_blocked(OPT1_SLEEP, false));
        assert!(!movement_blocked(OPT1_STONEWAIT, false));
        assert!(!movement_blocked(0, false));
        assert!(movement_blocked(0, true), "root blocks movement");
    }

    #[test]
    fn overlays_combine_body_and_health_states() {
        assert_eq!(ailment_overlays(OPT1_STUN, 0), vec![AilmentOverlay::Stun]);
        assert_eq!(
            ailment_overlays(OPT1_STUN, OPT2_CURSE),
            vec![AilmentOverlay::Stun, AilmentOverlay::Curse]
        );
        assert_eq!(
            ailment_overlays(OPT1_FREEZE, 0),
            vec![AilmentOverlay::Freeze]
        );
        assert!(AilmentOverlay::Freeze.on_body());
        assert!(!AilmentOverlay::Stun.on_body());
        assert!(ailment_overlays(0, OPT2_ANGELUS).is_empty());
        assert!(ailment_overlays(0, OPT2_BLIND).is_empty());
    }
}
