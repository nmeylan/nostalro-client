//! 4-slot RadialEmitter state machine for casting-ring / radial-shell
//! style effects.
//!
//! Mirrors the original game's four-slot radial effect — four slots, each
//! carrying the per-slot bookkeeping (`distance`, `rise_angle`, `process`
//! frame counter, alpha pair, oscillator arrays) that the per-frame
//! integrators advance, and that the matching render function turns into
//! a ring / cone / shell of billboards radiating outward from a center.
//!
//! Scope: state container + a trivial per-frame `tick()` that bumps `process`
//! on live slots. Each calling effect implements its own physics on top
//! (distance growth law, rise_angle decay curve, alpha envelope) — the
//! original game's per-frame integrators differ enough between effects that
//! encoding them all here would be premature.
//!
//! Cached billboard corners are NOT mirrored: the renderer recomputes
//! them per frame from
//! `distance` + `rise_angle_deg` + `height[]`.

/// Number of state slots per effect. Matches the original game's slot count.
pub const RADIAL_EMITTER_SLOTS: usize = 4;

/// Size of the per-slot oscillator / flag arrays. Matches the original
/// game's per-ring subdivision. Also the maximum number of billboards a
/// `RadialRing` draw can emit per slot.
pub const RADIAL_EMITTER_DIVISION: usize = 21;

#[derive(Clone, Copy, Debug)]
pub struct RadialEmitterSlot {
    pub alive: bool,
    pub process: u32,
    pub distance: f32,
    pub rise_angle_deg: f32,
    pub rot_start_deg: f32,
    pub alpha_b: f32,
    pub alpha_t: f32,
    pub full_display_angle_deg: f32,
    pub max_height: f32,
    pub height: [f32; RADIAL_EMITTER_DIVISION],
    pub flag1: [u8; RADIAL_EMITTER_DIVISION],
}

impl RadialEmitterSlot {
    pub const fn dormant() -> Self {
        Self {
            alive: false,
            process: 0,
            distance: 0.0,
            rise_angle_deg: 0.0,
            rot_start_deg: 0.0,
            alpha_b: 0.0,
            alpha_t: 0.0,
            full_display_angle_deg: 0.0,
            max_height: 0.0,
            height: [0.0; RADIAL_EMITTER_DIVISION],
            flag1: [0; RADIAL_EMITTER_DIVISION],
        }
    }

    pub fn spawn(distance: f32, rise_angle_deg: f32, max_height: f32) -> Self {
        let mut s = Self::dormant();
        s.alive = true;
        s.distance = distance;
        s.rise_angle_deg = rise_angle_deg;
        s.max_height = max_height;
        s.alpha_b = 1.0;
        s.alpha_t = 1.0;
        s.full_display_angle_deg = 360.0;
        s
    }

    pub fn tick(&mut self) {
        if self.alive {
            self.process = self.process.saturating_add(1);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RadialEmitter {
    pub slots: [RadialEmitterSlot; RADIAL_EMITTER_SLOTS],
}

impl RadialEmitter {
    pub const fn empty() -> Self {
        Self {
            slots: [RadialEmitterSlot::dormant(); RADIAL_EMITTER_SLOTS],
        }
    }

    pub fn from_slots(slots: [RadialEmitterSlot; RADIAL_EMITTER_SLOTS]) -> Self {
        Self { slots }
    }

    pub fn tick(&mut self) {
        for slot in self.slots.iter_mut() {
            slot.tick();
        }
    }

    pub fn active(&self) -> impl Iterator<Item = (usize, &RadialEmitterSlot)> {
        self.slots.iter().enumerate().filter(|(_, s)| s.alive)
    }
}

impl Default for RadialEmitter {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances_alive_and_leaves_dormant() {
        let mut emitter = RadialEmitter::empty();
        emitter.slots[0] = RadialEmitterSlot::spawn(1.0, 80.0, 5.0);
        emitter.slots[2] = RadialEmitterSlot::spawn(2.0, 70.0, 5.0);

        for _ in 0..3 {
            emitter.tick();
        }

        assert_eq!(emitter.slots[0].process, 3);
        assert!(emitter.slots[0].alive);
        assert_eq!(emitter.slots[2].process, 3);
        assert_eq!(emitter.slots[1].process, 0, "dormant slot must stay at 0");
        assert!(!emitter.slots[1].alive);
        assert!(!emitter.slots[3].alive);
    }

    #[test]
    fn active_yields_alive_slots_in_order_and_carries_state() {
        let mut emitter = RadialEmitter::empty();
        emitter.slots[0] = RadialEmitterSlot::spawn(1.0, 80.0, 5.0);
        emitter.slots[2] = RadialEmitterSlot::spawn(3.5, 45.0, 7.0);
        emitter.tick();

        let collected: Vec<(usize, f32, f32)> = emitter
            .active()
            .map(|(i, s)| (i, s.distance, s.rise_angle_deg))
            .collect();

        assert_eq!(
            collected,
            vec![(0, 1.0, 80.0), (2, 3.5, 45.0)],
            "active() yields slot 0 and slot 2 in index order with seeded values",
        );
    }
}
