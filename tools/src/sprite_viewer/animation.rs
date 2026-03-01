use ragnarok_formats::act::ActFile;

pub struct AnimationState {
    pub action: usize,
    pub direction: usize,
    pub motion_index: usize,
    pub paused: bool,
    accumulated_ms: f32,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            action: 0,
            direction: 0,
            motion_index: 0,
            paused: false,
            accumulated_ms: 0.0,
        }
    }

    pub fn action_index(&self, act: &ActFile) -> usize {
        (self.action * 8 + self.direction) % act.actions.len()
    }

    pub fn current_delay_ms(&self, act: &ActFile) -> f32 {
        let idx = self.action_index(act);
        if idx < act.delays.len() {
            let d = act.delays[idx];
            if d > 0.0 { d } else { 150.0 }
        } else {
            150.0
        }
    }

    pub fn update(&mut self, dt_secs: f32, act: &ActFile) {
        if self.paused {
            return;
        }
        let action_idx = self.action_index(act);
        let motion_count = act.actions[action_idx].motions.len();
        if motion_count == 0 {
            return;
        }

        self.accumulated_ms += dt_secs * 1000.0;
        let interval = self.current_delay_ms(act);
        while self.accumulated_ms >= interval {
            self.accumulated_ms -= interval;
            self.motion_index = (self.motion_index + 1) % motion_count;
        }
    }

    pub fn set_action(&mut self, action: usize, act: &ActFile) {
        let max_action = act.actions.len() / 8;
        self.action = action % max_action.max(1);
        self.motion_index = 0;
        self.accumulated_ms = 0.0;
    }

    pub fn set_direction(&mut self, direction: usize) {
        self.direction = direction % 8;
        self.motion_index = 0;
        self.accumulated_ms = 0.0;
    }

    pub fn step_forward(&mut self, act: &ActFile) {
        let action_idx = self.action_index(act);
        let motion_count = act.actions[action_idx].motions.len();
        if motion_count > 0 {
            self.motion_index = (self.motion_index + 1) % motion_count;
        }
    }

    pub fn step_backward(&mut self, act: &ActFile) {
        let action_idx = self.action_index(act);
        let motion_count = act.actions[action_idx].motions.len();
        if motion_count > 0 {
            self.motion_index = if self.motion_index == 0 {
                motion_count - 1
            } else {
                self.motion_index - 1
            };
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
        // 8 directions per action, so total = action_count * 8
        // But we build just action_count actions total
        ActFile {
            version: (2, 5),
            actions,
            events: Vec::new(),
            delays: vec![100.0; action_count],
        }
    }

    #[test]
    fn animation_cycles_through_motions() {
        let act = make_act(8, 3);
        let mut anim = AnimationState::new();
        // 100ms per frame, advance by 250ms => should move to frame 2
        anim.update(0.25, &act);
        assert_eq!(anim.motion_index, 2);
    }

    #[test]
    fn animation_wraps_around() {
        let act = make_act(8, 3);
        let mut anim = AnimationState::new();
        anim.update(0.35, &act);
        assert_eq!(anim.motion_index, 0); // 350ms / 100ms = 3 frames, 3 % 3 = 0
    }

    #[test]
    fn step_backward_wraps() {
        let mut anim = AnimationState::new();
        anim.step_backward(&make_act(8, 4));
        assert_eq!(anim.motion_index, 3);
    }

    #[test]
    fn direction_change_resets_motion() {
        let mut anim = AnimationState::new();
        anim.motion_index = 2;
        anim.set_direction(3);
        assert_eq!(anim.direction, 3);
        assert_eq!(anim.motion_index, 0);
    }
}
