use ragnarok_formats::act::ActFile;

pub struct SpriteAnimationState {
    action: usize,
    direction: usize,
    motion_index: usize,
    accumulated_ms: f32,
}

impl SpriteAnimationState {
    pub fn new(direction: u8) -> Self {
        Self {
            action: 0,
            direction: direction as usize % 8,
            motion_index: 0,
            accumulated_ms: 0.0,
        }
    }

    /// Sets the base action (0=idle, 1=walk, etc). Only resets motion if action changed.
    pub fn set_action(&mut self, action: usize) {
        if self.action != action {
            self.action = action;
            self.motion_index = 0;
            self.accumulated_ms = 0.0;
        }
    }

    pub fn set_direction(&mut self, direction: u8) {
        self.direction = direction as usize % 8;
    }

    /// Computes the flat action index into the ACT file, applying camera direction offset.
    /// RO ACT files store actions in groups of 8 (one per direction).
    /// Our camera direction_index goes CW (N=0,E=2,S=4,W=6) while RO ACT
    /// directions go CCW, so we negate camera_dir. The +4 accounts for our
    /// default camera being north vs the ACT reference frame.
    pub fn action_index(&self, act: &ActFile, camera_dir: u8) -> usize {
        let effective_dir = (12 - self.direction - camera_dir as usize) % 8;
        (self.action * 8 + effective_dir) % act.actions.len()
    }

    pub fn motion_index(&self) -> usize {
        self.motion_index
    }

    pub fn update(&mut self, dt_secs: f32, act: &ActFile, camera_dir: u8) {
        let action_idx = self.action_index(act, camera_dir);
        let motion_count = act.actions[action_idx].motions.len();
        if motion_count == 0 {
            return;
        }

        let delay_ms = if action_idx < act.delays.len() {
            let d = act.delays[action_idx] * 25.0;
            if d > 0.0 { d } else { 150.0 }
        } else {
            150.0
        };

        self.accumulated_ms += dt_secs * 1000.0;
        while self.accumulated_ms >= delay_ms {
            self.accumulated_ms -= delay_ms;
            self.motion_index = (self.motion_index + 1) % motion_count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::act::{ActFile, Action, Motion};

    fn make_act(action_count: usize, motions_per_action: usize) -> ActFile {
        let actions: Vec<Action> = (0..action_count).map(|_| {
            Action {
                motions: (0..motions_per_action).map(|_| Motion {
                    range1: [0; 4], range2: [0; 4],
                    clips: Vec::new(), event_id: -1,
                    attach_points: Vec::new(),
                }).collect(),
            }
        }).collect();
        ActFile {
            version: (2, 5),
            actions,
            events: Vec::new(),
            delays: vec![4.0; action_count],
        }
    }

    #[test]
    fn action_index_combines_action_direction_and_camera() {
        let act = make_act(16, 1);
        let anim = SpriteAnimationState::new(2);
        // action=0, entity_dir=2, camera_dir=3 => (12-2-3)%8=7
        assert_eq!(anim.action_index(&act, 3), 7);
    }

    #[test]
    fn action_index_walk_with_camera_offset() {
        let act = make_act(16, 1);
        let mut anim = SpriteAnimationState::new(0);
        anim.set_action(1); // walk
        // action=1, entity_dir=0, camera_dir=0 => (12-0-0)%8=4, 1*8+4=12
        assert_eq!(anim.action_index(&act, 0), 12);
    }

    #[test]
    fn entity_south_at_north_camera_shows_front() {
        // Entity faces south (0), camera north (dir=0)
        // Entity faces toward camera → front sprite (ACT dir 4)
        let act = make_act(8, 1);
        let anim = SpriteAnimationState::new(0);
        assert_eq!(anim.action_index(&act, 0), 4);
    }

    #[test]
    fn entity_north_at_north_camera_shows_back() {
        // Entity faces north (4), camera north (dir=0)
        // Entity faces away → back sprite (ACT dir 0)
        let act = make_act(8, 1);
        let anim = SpriteAnimationState::new(4);
        assert_eq!(anim.action_index(&act, 0), 0);
    }

    #[test]
    fn entity_east_at_north_camera_shows_left() {
        // Entity faces east (6), camera north (dir=0)
        // Entity faces right on screen → ACT dir 6
        let act = make_act(8, 1);
        let anim = SpriteAnimationState::new(6);
        assert_eq!(anim.action_index(&act, 0), 6);
    }

    #[test]
    fn set_action_resets_motion_only_on_change() {
        let mut anim = SpriteAnimationState::new(0);
        anim.motion_index = 3;
        anim.accumulated_ms = 50.0;
        anim.set_action(0); // same action, no reset
        assert_eq!(anim.motion_index, 3);

        anim.set_action(1); // different action, resets
        assert_eq!(anim.motion_index, 0);
        assert_eq!(anim.accumulated_ms, 0.0);
    }

    #[test]
    fn update_advances_motion() {
        let act = make_act(8, 3);
        let mut anim = SpriteAnimationState::new(0);
        // delay = 4.0 * 25 = 100ms, advance by 250ms => 2 frames
        anim.update(0.25, &act, 0);
        assert_eq!(anim.motion_index, 2);
    }
}
