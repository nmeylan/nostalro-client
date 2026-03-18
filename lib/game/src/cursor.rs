use ragnarok_formats::act::ActFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorType {
    Default = 0,
    Talk = 1,
    Click = 2,
    Lock = 3,
    Rotate = 4,
    Attack = 5,
    Warp = 7,
    NoWalk = 13,
}

pub struct CursorAnimationState {
    cursor_type: CursorType,
    motion_index: usize,
    accumulated_ms: f32,
}

impl CursorAnimationState {
    pub fn new() -> Self {
        Self {
            cursor_type: CursorType::Default,
            motion_index: 0,
            accumulated_ms: 0.0,
        }
    }

    pub fn set_cursor_type(&mut self, ty: CursorType) {
        if self.cursor_type != ty {
            self.cursor_type = ty;
            self.motion_index = 0;
            self.accumulated_ms = 0.0;
        }
    }

    pub fn cursor_type(&self) -> CursorType {
        self.cursor_type
    }

    pub fn action_index(&self) -> usize {
        self.cursor_type as usize
    }

    pub fn motion_index(&self) -> usize {
        self.motion_index
    }

    pub fn update(&mut self, dt_secs: f32, act: &ActFile) {
        let action_idx = self.action_index();
        if action_idx >= act.actions.len() {
            return;
        }
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

    fn make_cursor_act(action_count: usize, motions_per_action: usize) -> ActFile {
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
    fn action_index_maps_to_cursor_type_value() {
        let mut anim = CursorAnimationState::new();
        assert_eq!(anim.action_index(), 0);

        anim.set_cursor_type(CursorType::NoWalk);
        assert_eq!(anim.action_index(), 13);

        anim.set_cursor_type(CursorType::Attack);
        assert_eq!(anim.action_index(), 5);
    }

    #[test]
    fn set_cursor_type_resets_on_change() {
        let act = make_cursor_act(14, 4);
        let mut anim = CursorAnimationState::new();
        anim.update(0.5, &act);
        assert!(anim.motion_index > 0);

        anim.set_cursor_type(CursorType::Talk);
        assert_eq!(anim.motion_index(), 0);
        assert_eq!(anim.accumulated_ms, 0.0);

        // Same type again does not reset
        anim.update(0.25, &act);
        let idx = anim.motion_index();
        anim.set_cursor_type(CursorType::Talk);
        assert_eq!(anim.motion_index(), idx);
    }

    #[test]
    fn update_advances_motion_frames() {
        let act = make_cursor_act(1, 3);
        let mut anim = CursorAnimationState::new();
        // delay = 4.0 * 25 = 100ms, advance by 250ms => 2 frames
        anim.update(0.25, &act);
        assert_eq!(anim.motion_index(), 2);
    }
}
