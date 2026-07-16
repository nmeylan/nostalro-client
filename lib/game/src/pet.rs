use ragnarok_formats::act::ActFile;

/// Owner-side view of the player's active pet. Populated from ZC_PROPERTY_PET
/// (0x1a2) and the ZC_CHANGESTATE_PET (0x1a4) state stream.
#[derive(Debug, Clone, Default)]
pub struct PetState {
    pub gid: Option<u32>,
    pub job: i16,
    pub name: String,
    pub renamed: bool,
    pub level: i16,
    pub hunger: i16,
    pub intimacy: i16,
    pub accessory: u16,
    pub egg_index: Option<u16>,
    pub capture_pending: bool,
}

/// Hunger bands (original client). Values index msgstringtable 667..=671.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HungerState {
    VeryHungry,
    Hungry,
    Neutral,
    Satisfied,
    Stuffed,
}

impl HungerState {
    pub fn from_value(hunger: i16) -> Self {
        match hunger {
            h if h < 10 => HungerState::VeryHungry,
            h if h < 25 => HungerState::Hungry,
            h if h < 75 => HungerState::Neutral,
            h if h < 90 => HungerState::Satisfied,
            _ => HungerState::Stuffed,
        }
    }

    /// 0..=4, used to index the emotion table.
    pub fn index(self) -> usize {
        match self {
            HungerState::VeryHungry => 0,
            HungerState::Hungry => 1,
            HungerState::Neutral => 2,
            HungerState::Satisfied => 3,
            HungerState::Stuffed => 4,
        }
    }

    pub fn msg_id(self) -> u16 {
        667 + self.index() as u16
    }

    pub fn label(self) -> &'static str {
        match self {
            HungerState::VeryHungry => "Very Hungry",
            HungerState::Hungry => "Hungry",
            HungerState::Neutral => "Neutral",
            HungerState::Satisfied => "Satisfied",
            HungerState::Stuffed => "Stuffed",
        }
    }
}

/// Intimacy bands (original client). msgstringtable 672/673/669/674/675.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntimacyState {
    Awkward,
    Shy,
    Neutral,
    Cordial,
    Loyal,
}

impl IntimacyState {
    pub fn from_value(intimacy: i16) -> Self {
        match intimacy {
            i if i < 100 => IntimacyState::Awkward,
            i if i < 250 => IntimacyState::Shy,
            i if i < 750 => IntimacyState::Neutral,
            i if i < 900 => IntimacyState::Cordial,
            _ => IntimacyState::Loyal,
        }
    }

    /// 0..=4, used to index the emotion table.
    pub fn index(self) -> usize {
        match self {
            IntimacyState::Awkward => 0,
            IntimacyState::Shy => 1,
            IntimacyState::Neutral => 2,
            IntimacyState::Cordial => 3,
            IntimacyState::Loyal => 4,
        }
    }

    pub fn msg_id(self) -> u16 {
        match self {
            IntimacyState::Awkward => 672,
            IntimacyState::Shy => 673,
            IntimacyState::Neutral => 669,
            IntimacyState::Cordial => 674,
            IntimacyState::Loyal => 675,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            IntimacyState::Awkward => "Awkward",
            IntimacyState::Shy => "Shy",
            IntimacyState::Neutral => "Neutral",
            IntimacyState::Cordial => "Cordial",
            IntimacyState::Loyal => "Loyal",
        }
    }
}

/// Capture roulette (slotmachine) state machine, driven by CZ/ZC_TRYCAPTURE.
/// The slotmachine sprite has one non-directional action per phase: 0 idle,
/// 1 spin, 2 jackpot, 3 miss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouletteState {
    Idle,
    Spinning,
    Success,
    Fail,
}

pub struct PetRoulette {
    pub target_gid: u32,
    pub state: RouletteState,
    /// CZ_TRYCAPTURE_MONSTER already sent for this target.
    pub sent: bool,
    /// Capture result once ZC_TRYCAPTURE arrives.
    pub result: Option<bool>,
    /// Milliseconds elapsed in the current phase.
    phase_ms: f32,
    /// slotmachine (action, motion) frame to draw this update.
    pub frame: (usize, usize),
    /// Render-clock time at which the window auto-closes.
    pub close_at: Option<f32>,
}

impl PetRoulette {
    pub fn new(target_gid: u32) -> Self {
        Self {
            target_gid,
            state: RouletteState::Idle,
            sent: false,
            result: None,
            phase_ms: 0.0,
            frame: (0, 0),
            close_at: None,
        }
    }

    /// slotmachine.act action row for the current state.
    pub fn action_index(&self) -> usize {
        match self.state {
            RouletteState::Idle => 0,
            RouletteState::Spinning => 1,
            RouletteState::Success => 2,
            RouletteState::Fail => 3,
        }
    }

    /// The result arrived: start the spin that settles onto the jackpot/miss row.
    pub fn resolve(&mut self, ok: bool) {
        self.result = Some(ok);
        self.state = RouletteState::Spinning;
        self.phase_ms = 0.0;
    }

    /// Advances the phase clock and picks the frame to draw. `idle` loops; `spin`
    /// runs once then settles onto the result row, which plays once and then
    /// schedules the close 500 ms later.
    pub fn advance(&mut self, act: &ActFile, dt_ms: f32, now: f32) {
        self.phase_ms += dt_ms;
        let action = self.action_index();
        let motion_count = act
            .actions
            .get(action)
            .map(|a| a.motions.len())
            .unwrap_or(0)
            .max(1);
        let delay_ms = act
            .delays
            .get(action)
            .copied()
            .filter(|d| *d > 0.0)
            .map(|d| d * 25.0)
            .unwrap_or(100.0);
        let raw = (self.phase_ms / delay_ms) as usize;
        match self.state {
            RouletteState::Idle => self.frame = (action, raw % motion_count),
            RouletteState::Spinning => {
                if raw >= motion_count {
                    self.state = if self.result.unwrap_or(false) {
                        RouletteState::Success
                    } else {
                        RouletteState::Fail
                    };
                    self.phase_ms = 0.0;
                    self.frame = (self.action_index(), 0);
                } else {
                    self.frame = (action, raw);
                }
            }
            RouletteState::Success | RouletteState::Fail => {
                if raw >= motion_count {
                    self.close_at.get_or_insert(now + 0.5);
                    self.frame = (action, motion_count - 1);
                } else {
                    self.frame = (action, raw);
                }
            }
        }
    }
}

/// CZ/ZC_PET_ACT `data` encoding boundaries.
pub const PET_TALK_MALE_OFFSET: i32 = 50000;
pub const PET_TALK_FEMALE_OFFSET: i32 = 90000;

/// XML hunger keys, indexed by `HungerState::index()`.
pub const HUNGER_KEYS: [&str; 5] = ["hungry", "bit_hungry", "noting", "full", "so_full"];
/// XML act keys, indexed by pet act (PM_*).
pub const ACT_KEYS: [&str; 11] = [
    "feeding", "hunting", "danger", "dead", "normal", "perfor_s", "levelup", "perfor_1",
    "perfor_2", "perfor_3", "connect",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetTalkCode {
    pub mob_id: i32,
    pub act: usize,
    pub hunger: usize,
    pub female: bool,
}

/// Encodes an owner-generated talk line into the CZ_PET_ACT `data` value.
pub fn encode_pet_talk(mob_id: i32, act: usize, hunger: usize, female: bool) -> i32 {
    let base = (mob_id - 1000) * 100 + act as i32 * 10 + hunger as i32;
    base + if female {
        PET_TALK_FEMALE_OFFSET
    } else {
        PET_TALK_MALE_OFFSET
    }
}

/// Decodes a ZC_PET_ACT `data` value into a talk line, or `None` for an emote.
pub fn decode_pet_talk(data: i32) -> Option<PetTalkCode> {
    if data < PET_TALK_MALE_OFFSET {
        return None;
    }
    let (female, base) = if data >= PET_TALK_FEMALE_OFFSET {
        (true, data - PET_TALK_FEMALE_OFFSET)
    } else {
        (false, data - PET_TALK_MALE_OFFSET)
    };
    Some(PetTalkCode {
        mob_id: base / 100 + 1000,
        act: ((base % 100) / 10) as usize,
        hunger: (base % 10) as usize,
        female,
    })
}

/// ZC_CHANGESTATE_PET (0x1a4) `atype` values.
pub const PET_STATE_INIT: i8 = 0;
pub const PET_STATE_INTIMACY: i8 = 1;
pub const PET_STATE_HUNGER: i8 = 2;
pub const PET_STATE_ACCESSORY: i8 = 3;
pub const PET_STATE_PERFORMANCE: i8 = 4;
pub const PET_STATE_MARKER: i8 = 5;

impl PetState {
    pub fn is_active(&self) -> bool {
        self.gid.is_some()
    }

    pub fn apply_property(&mut self, p: &crate::event::PetProperty) {
        self.name = p.name.clone();
        self.renamed = p.renamed;
        self.level = p.level;
        self.hunger = p.hunger;
        self.intimacy = p.intimacy;
        self.accessory = p.accessory;
        self.job = p.job;
    }

    /// Applies the pure-state part of ZC_CHANGESTATE_PET. Entity-side effects
    /// (marker / accessory ACT swap / performance) are handled by the caller.
    pub fn apply_state_changed(&mut self, ty: i8, gid: u32, data: i32) {
        match ty {
            PET_STATE_INIT => self.gid = Some(gid),
            PET_STATE_INTIMACY => self.intimacy = data as i16,
            PET_STATE_HUNGER => self.hunger = data as i16,
            PET_STATE_ACCESSORY => self.accessory = data as u16,
            _ => {}
        }
    }

    pub fn hunger_state(&self) -> HungerState {
        HungerState::from_value(self.hunger)
    }

    pub fn intimacy_state(&self) -> IntimacyState {
        IntimacyState::from_value(self.intimacy)
    }

    /// The pet vanished (returned to egg / left the map): drop the tracked GID
    /// but keep the last known stats for the info window.
    pub fn clear_entity(&mut self) {
        self.gid = None;
    }

    /// Illustration texture path keyed by mob class, Poring when unmapped.
    pub fn illust_path(&self) -> &'static str {
        crate::pet_tables::pet_illust(self.job).unwrap_or(crate::pet_tables::PET_ILLUST_FALLBACK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::PetProperty;

    #[test]
    fn rename_lock_reflects_modified_flag() {
        let mut pet = PetState::default();
        pet.apply_property(&PetProperty {
            name: "Poring".into(),
            renamed: true,
            level: 1,
            hunger: 50,
            intimacy: 500,
            accessory: 0,
            job: 1002,
        });
        assert!(pet.renamed);
    }

    #[test]
    fn talk_code_round_trips() {
        for female in [false, true] {
            let data = encode_pet_talk(1002, 4, 3, female);
            let code = decode_pet_talk(data).unwrap();
            assert_eq!(code.mob_id, 1002);
            assert_eq!(code.act, 4);
            assert_eq!(code.hunger, 3);
            assert_eq!(code.female, female);
        }
        assert_eq!(decode_pet_talk(30), None);
    }

    fn slotmachine_act() -> ActFile {
        use ragnarok_formats::act::{Action, Motion};
        let motion = || Motion {
            range1: [0; 4],
            range2: [0; 4],
            clips: Vec::new(),
            event_id: -1,
            attach_points: Vec::new(),
        };
        ActFile {
            version: (2, 5),
            actions: (0..4)
                .map(|_| Action {
                    motions: vec![motion(), motion()],
                })
                .collect(),
            events: Vec::new(),
            delays: vec![4.0; 4],
        }
    }

    #[test]
    fn roulette_spins_then_settles_on_result_then_closes() {
        let act = slotmachine_act();
        let delay_ms = 4.0 * 25.0;
        let mut r = PetRoulette::new(42);

        r.advance(&act, delay_ms, 0.0);
        assert_eq!(r.state, RouletteState::Idle, "idles until the result arrives");

        r.resolve(false);
        assert_eq!(r.state, RouletteState::Spinning);

        // Spin plays both motions, then settles onto the miss row.
        r.advance(&act, delay_ms, 0.1);
        assert_eq!(r.frame, (1, 1));
        r.advance(&act, delay_ms, 0.2);
        assert_eq!(r.state, RouletteState::Fail);

        // Miss row plays out, then schedules the close 500 ms later.
        r.advance(&act, delay_ms, 0.3);
        r.advance(&act, delay_ms * 2.0, 0.4);
        assert_eq!(r.close_at, Some(0.9));
    }

    #[test]
    fn feed_cycle_updates_hunger_and_intimacy() {
        let mut pet = PetState::default();
        pet.hunger = 20;
        pet.apply_state_changed(PET_STATE_HUNGER, 1, 80);
        pet.apply_state_changed(PET_STATE_INTIMACY, 1, 910);
        assert_eq!(pet.hunger, 80);
        assert_eq!(pet.intimacy, 910);
        assert_eq!(pet.intimacy_state(), IntimacyState::Loyal);
    }
}
