use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::{Color, FormatError, read_string, version_at_least};

pub struct AnchorPoint {
    pub ignored: u32,
    pub x: i32,
    pub y: i32,
    pub attribute: u32,
}

pub struct SpriteFrame {
    pub x: i32,
    pub y: i32,
    pub sprite_index: i32,
    pub mirror: u32,
    pub color: Color,
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub angle: i32,
    pub sprite_type: u32,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub struct Motion {
    pub range1: [i32; 4],
    pub range2: [i32; 4],
    pub clips: Vec<SpriteFrame>,
    pub event_id: i32,
    pub attach_points: Vec<AnchorPoint>,
}

pub struct Action {
    pub motions: Vec<Motion>,
}

pub struct ActFile {
    pub version: (u8, u8),
    pub actions: Vec<Action>,
    pub events: Vec<String>,
    pub delays: Vec<f32>,
}

impl ActFile {
    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Cursor::new(data);

        let mut sig = [0u8; 2];
        r.read_exact(&mut sig)?;
        if &sig != b"AC" {
            return Err(FormatError::InvalidMagic);
        }

        let ver_minor = r.read_u8()?;
        let ver_major = r.read_u8()?;
        let version = (ver_major, ver_minor);

        let action_count = r.read_u16::<LE>()? as usize;
        r.seek(SeekFrom::Current(10))?;

        let mut actions = Vec::with_capacity(action_count);
        for _ in 0..action_count {
            let motion_count = r.read_u32::<LE>()? as usize;
            let mut motions = Vec::with_capacity(motion_count);
            for _ in 0..motion_count {
                let mut range1 = [0i32; 4];
                let mut range2 = [0i32; 4];
                for v in &mut range1 {
                    *v = r.read_i32::<LE>()?;
                }
                for v in &mut range2 {
                    *v = r.read_i32::<LE>()?;
                }

                let clip_count = r.read_u32::<LE>()? as usize;
                let mut clips = Vec::with_capacity(clip_count);
                for _ in 0..clip_count {
                    let x = r.read_i32::<LE>()?;
                    let y = r.read_i32::<LE>()?;
                    let sprite_index = r.read_i32::<LE>()?;
                    let mirror = r.read_u32::<LE>()?;

                    let (color, zoom_x, zoom_y, angle, sprite_type) =
                        if version_at_least(version, 2, 0) {
                            let color_u32 = r.read_u32::<LE>()?;
                            let color = color_u32.to_le_bytes();

                            let (zoom_x, zoom_y) = if version_at_least(version, 2, 4) {
                                (r.read_f32::<LE>()?, r.read_f32::<LE>()?)
                            } else {
                                let zoom = r.read_f32::<LE>()?;
                                (zoom, zoom)
                            };

                            let angle = r.read_i32::<LE>()?;
                            let sprite_type = r.read_u32::<LE>()?;
                            (color, zoom_x, zoom_y, angle, sprite_type)
                        } else {
                            ([255, 255, 255, 255], 1.0, 1.0, 0, 0)
                        };

                    let (width, height) = if version_at_least(version, 2, 5) {
                        (Some(r.read_u32::<LE>()?), Some(r.read_u32::<LE>()?))
                    } else {
                        (None, None)
                    };

                    clips.push(SpriteFrame {
                        x,
                        y,
                        sprite_index,
                        mirror,
                        color,
                        zoom_x,
                        zoom_y,
                        angle,
                        sprite_type,
                        width,
                        height,
                    });
                }

                let event_id = if version_at_least(version, 2, 0) {
                    r.read_i32::<LE>()?
                } else {
                    -1
                };

                let attach_points = if version_at_least(version, 2, 3) {
                    let count = r.read_u32::<LE>()? as usize;
                    let mut points = Vec::with_capacity(count);
                    for _ in 0..count {
                        points.push(AnchorPoint {
                            ignored: r.read_u32::<LE>()?,
                            x: r.read_i32::<LE>()?,
                            y: r.read_i32::<LE>()?,
                            attribute: r.read_u32::<LE>()?,
                        });
                    }
                    points
                } else {
                    Vec::new()
                };

                motions.push(Motion {
                    range1,
                    range2,
                    clips,
                    event_id,
                    attach_points,
                });
            }
            actions.push(Action { motions });
        }

        let events = if version_at_least(version, 2, 1) {
            let count = r.read_u32::<LE>()? as usize;
            let mut events = Vec::with_capacity(count);
            for _ in 0..count {
                events.push(read_string(&mut r, 40)?);
            }
            events
        } else {
            Vec::new()
        };

        let delays = if version_at_least(version, 2, 2) {
            let mut delays = Vec::with_capacity(action_count);
            for _ in 0..action_count {
                delays.push(r.read_f32::<LE>()?);
            }
            delays
        } else {
            Vec::new()
        };

        Ok(ActFile {
            version,
            actions,
            events,
            delays,
        })
    }
}

/// Frame index within `action_idx` at which the swing connects — the first
/// motion carrying the `"atk"` event. Falls back to the second-to-last frame
/// when the action has no atk event, mirroring the original game's lookup.
pub fn atk_keyframe_index(act: &ActFile, action_idx: usize) -> usize {
    let Some(action) = act.actions.get(action_idx) else {
        return 0;
    };
    let motion_count = action.motions.len();
    if motion_count == 0 {
        return 0;
    }
    for (i, motion) in action.motions.iter().enumerate() {
        if motion.event_id >= 0
            && act
                .events
                .get(motion.event_id as usize)
                .is_some_and(|name| name.eq_ignore_ascii_case("atk"))
        {
            return i;
        }
    }
    motion_count.saturating_sub(2)
}

pub fn attachment_offset(body_motion: &Motion, head_motion: &Motion) -> (i32, i32) {
    match (
        body_motion.attach_points.first(),
        head_motion.attach_points.first(),
    ) {
        (Some(body), Some(head)) => (body.x - head.x, body.y - head.y),
        _ => (0, 0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpriteActionType {
    Idle = 0,
    Walk = 1,
    Sit = 2,
    Pickup = 3,
    ReadyFight = 4,
    Attack1 = 5,
    Hurt = 6,
    Freeze = 7,
    Die = 8,
    Freeze2 = 9,
    Attack2 = 10,
    Attack3 = 11,
    Skill = 12,
}

impl SpriteActionType {
    pub fn is_animated(self) -> bool {
        !matches!(self, Self::Idle | Self::Sit | Self::ReadyFight)
    }

    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Idle),
            1 => Some(Self::Walk),
            2 => Some(Self::Sit),
            3 => Some(Self::Pickup),
            4 => Some(Self::ReadyFight),
            5 => Some(Self::Attack1),
            6 => Some(Self::Hurt),
            7 => Some(Self::Freeze),
            8 => Some(Self::Die),
            9 => Some(Self::Freeze2),
            10 => Some(Self::Attack2),
            11 => Some(Self::Attack3),
            12 => Some(Self::Skill),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Walk => "Walk",
            Self::Sit => "Sit",
            Self::Pickup => "Pickup",
            Self::ReadyFight => "Ready",
            Self::Attack1 => "Attack1",
            Self::Hurt => "Hurt",
            Self::Freeze => "Freeze",
            Self::Die => "Die",
            Self::Freeze2 => "Freeze2",
            Self::Attack2 => "Attack2",
            Self::Attack3 => "Attack3",
            Self::Skill => "Skill",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionType {
    Loop,
    OneShot,
    Static,
}

#[derive(Clone)]
pub struct SpriteAnimationState {
    action: usize,
    direction: usize,
    motion_index: usize,
    accumulated_ms: f32,
    motion_type: MotionType,
    finished: bool,
    motion_speed_override_ms: Option<f32>,
    motion_speed_factor: Option<f32>,
    remaining_repeats: u16,
    walk_dist: f32,
    prev_motion: usize,
    prev_action: usize,
}

impl SpriteAnimationState {
    pub fn new(direction: u8) -> Self {
        Self {
            action: 0,
            direction: direction as usize % 8,
            motion_index: 0,
            accumulated_ms: 0.0,
            motion_type: MotionType::Loop,
            finished: false,
            motion_speed_override_ms: None,
            motion_speed_factor: None,
            remaining_repeats: 0,
            walk_dist: 0.0,
            prev_motion: 0,
            prev_action: 0,
        }
    }

    /// Frame sound-event ids crossed since the last call, scanning every frame
    /// stepped through (forward, or wrapping when the animation looped). Frames
    /// with no event (`event_id == -1`) are skipped. On an action change the
    /// scan restarts from frame 0. `action_idx` is the direction-resolved slot.
    pub fn crossed_event_ids(&mut self, act: &ActFile, action_idx: usize) -> Vec<i32> {
        let mut out = Vec::new();
        if action_idx >= act.actions.len() {
            return out;
        }
        let motions = &act.actions[action_idx].motions;
        let motion_count = motions.len();
        if motion_count == 0 {
            return out;
        }
        let cur = self.motion_index.min(motion_count - 1);
        let old = if self.action != self.prev_action {
            0
        } else {
            self.prev_motion.min(motion_count - 1)
        };
        if cur >= old {
            for m in (old + 1)..=cur {
                if motions[m].event_id != -1 {
                    out.push(motions[m].event_id);
                }
            }
        } else {
            for m in (old + 1)..motion_count {
                if motions[m].event_id != -1 {
                    out.push(motions[m].event_id);
                }
            }
            for m in 0..=cur {
                if motions[m].event_id != -1 {
                    out.push(motions[m].event_id);
                }
            }
        }
        self.prev_motion = cur;
        self.prev_action = self.action;
        out
    }

    pub fn action(&self) -> usize {
        self.action
    }

    pub fn direction(&self) -> usize {
        self.direction
    }

    pub fn motion_index(&self) -> usize {
        self.motion_index
    }

    pub fn set_action(&mut self, action: usize, motion_type: MotionType) {
        let changed = self.action != action;
        let restart = changed || (motion_type == MotionType::OneShot && self.finished);
        if restart {
            self.action = action;
            self.motion_index = 0;
            self.accumulated_ms = 0.0;
            self.finished = false;
            self.motion_speed_override_ms = None;
            self.motion_speed_factor = None;
            self.remaining_repeats = 0;
        } else if self.motion_type != motion_type {
            self.finished = false;
            self.motion_speed_override_ms = None;
            self.motion_speed_factor = None;
            self.remaining_repeats = 0;
        }
        self.motion_type = motion_type;
    }

    pub fn play(&mut self, action: usize, total_ms: f32, start_frame: usize) {
        self.action = action;
        self.motion_index = start_frame;
        self.accumulated_ms = 0.0;
        self.motion_type = MotionType::OneShot;
        self.finished = false;
        self.motion_speed_override_ms = Some(total_ms);
        self.motion_speed_factor = None;
        self.remaining_repeats = 0;
    }

    /// Plays `action` looping at the ACT's native frame delay multiplied by
    /// `speed_factor` (attack-speed relative to average). The attack state is
    /// ended by the entity state machine, so the swing simply loops meanwhile.
    pub fn play_attack_loop(&mut self, action: usize, speed_factor: f32, start_frame: usize) {
        self.action = action;
        self.motion_index = start_frame;
        self.accumulated_ms = 0.0;
        self.motion_type = MotionType::Loop;
        self.finished = false;
        self.motion_speed_override_ms = None;
        self.motion_speed_factor = Some(if speed_factor > 0.0 { speed_factor } else { 1.0 });
        self.remaining_repeats = 0;
    }

    pub fn play_repeated(&mut self, action: usize, total_ms: f32, repeat_count: u16) {
        self.action = action;
        self.motion_index = 0;
        self.accumulated_ms = 0.0;
        self.motion_type = MotionType::OneShot;
        self.finished = false;
        self.motion_speed_override_ms = Some(total_ms / repeat_count.max(1) as f32);
        self.motion_speed_factor = None;
        self.remaining_repeats = repeat_count.saturating_sub(1);
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn set_action_clamped(&mut self, action: usize, motion_type: MotionType, act: &ActFile) {
        let max_action = act.actions.len() / 8;
        self.set_action(action % max_action.max(1), motion_type);
    }

    pub fn set_direction(&mut self, direction: u8) {
        self.direction = direction as usize % 16;
    }

    pub fn action_index(&self, act: &ActFile, camera_dir: u8) -> usize {
        let effective_dir = (camera_dir as usize + 12 - self.direction) % 8;
        (self.action * 8 + effective_dir) % act.actions.len()
    }

    pub fn flat_action_index(&self, act: &ActFile) -> usize {
        (self.action * 8 + self.direction) % act.actions.len()
    }

    pub fn set_motion_speed_override(&mut self, total_ms: Option<f32>) {
        self.motion_speed_override_ms = total_ms;
    }

    pub fn reset_motion(&mut self) {
        self.motion_index = 0;
        self.accumulated_ms = 0.0;
    }

    pub fn set_motion_index(&mut self, idx: usize) {
        self.motion_index = idx;
    }

    pub fn step_forward(&mut self, motion_count: usize) {
        if motion_count > 0 {
            self.motion_index = (self.motion_index + 1) % motion_count;
        }
    }

    pub fn step_backward(&mut self, motion_count: usize) {
        if motion_count > 0 {
            self.motion_index = if self.motion_index == 0 {
                motion_count - 1
            } else {
                self.motion_index - 1
            };
        }
    }

    pub fn update(&mut self, dt_secs: f32, act: &ActFile, camera_dir: u8) {
        let action_idx = self.action_index(act, camera_dir);
        self.advance(action_idx, act, dt_secs);
    }

    pub fn update_by_distance(&mut self, dist_cells: f32, act: &ActFile, camera_dir: u8) {
        if self.motion_type != MotionType::Loop {
            return;
        }
        let action_idx = self.action_index(act, camera_dir);
        let motion_count = act.actions[action_idx].motions.len();
        if motion_count == 0 {
            return;
        }
        let frame_dist = if action_idx < act.delays.len() && act.delays[action_idx] > 0.0 {
            act.delays[action_idx] / 6.0
        } else {
            0.15
        };
        let cycle = frame_dist * motion_count as f32;
        self.walk_dist = (self.walk_dist + dist_cells.max(0.0)) % cycle.max(1e-4);
        self.motion_index = (self.walk_dist / frame_dist) as usize % motion_count;
    }

    pub fn update_flat(&mut self, dt_secs: f32, act: &ActFile) {
        let action_idx = self.flat_action_index(act);
        self.advance(action_idx, act, dt_secs);
    }

    fn advance(&mut self, action_idx: usize, act: &ActFile, dt_secs: f32) {
        if self.finished || self.motion_type == MotionType::Static {
            return;
        }
        let motion_count = act.actions[action_idx].motions.len();
        if motion_count == 0 {
            return;
        }

        let native_delay = if action_idx < act.delays.len() && act.delays[action_idx] > 0.0 {
            act.delays[action_idx] * 25.0
        } else {
            150.0
        };
        let delay_ms = if let Some(total_ms) = self.motion_speed_override_ms {
            if motion_count > 0 {
                total_ms / motion_count as f32
            } else {
                150.0
            }
        } else {
            native_delay * self.motion_speed_factor.unwrap_or(1.0)
        };

        self.accumulated_ms += dt_secs * 1000.0;
        while self.accumulated_ms >= delay_ms {
            self.accumulated_ms -= delay_ms;
            if self.motion_type == MotionType::OneShot && self.motion_index == motion_count - 1 {
                if self.remaining_repeats > 0 {
                    self.remaining_repeats -= 1;
                    self.motion_index = 0;
                    continue;
                }
                self.finished = true;
                self.accumulated_ms = 0.0;
                return;
            }
            self.motion_index = (self.motion_index + 1) % motion_count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_v2_5_act() {
        let mut data = Vec::new();
        data.extend_from_slice(b"AC");
        data.push(5);
        data.push(2);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 10]);

        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 32]);
        data.extend_from_slice(&1u32.to_le_bytes());

        data.extend_from_slice(&10i32.to_le_bytes());
        data.extend_from_slice(&20i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&2.0f32.to_le_bytes());
        data.extend_from_slice(&90i32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&32u32.to_le_bytes());
        data.extend_from_slice(&64u32.to_le_bytes());

        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());

        data.extend_from_slice(&1u32.to_le_bytes());
        let mut event_name = [0u8; 40];
        event_name[..4].copy_from_slice(b"atk1");
        data.extend_from_slice(&event_name);

        data.extend_from_slice(&4.0f32.to_le_bytes());

        let act = ActFile::parse(&data).unwrap();
        assert_eq!(act.version, (2, 5));
        assert_eq!(act.actions.len(), 1);
        assert_eq!(act.actions[0].motions.len(), 1);

        let clip = &act.actions[0].motions[0].clips[0];
        assert_eq!(clip.x, 10);
        assert_eq!(clip.y, 20);
        assert_eq!(clip.zoom_x, 1.0);
        assert_eq!(clip.zoom_y, 2.0);
        assert_eq!(clip.width, Some(32));
        assert_eq!(clip.height, Some(64));

        assert_eq!(act.events.len(), 1);
        assert_eq!(act.events[0], "atk1");
        assert_eq!(act.delays, [4.0]);
    }

    fn make_motion_with_attach(x: i32, y: i32) -> Motion {
        Motion {
            range1: [0; 4],
            range2: [0; 4],
            clips: Vec::new(),
            event_id: -1,
            attach_points: vec![AnchorPoint {
                ignored: 0,
                x,
                y,
                attribute: 0,
            }],
        }
    }

    #[test]
    fn head_offset_from_attach_points() {
        let body = make_motion_with_attach(10, -20);
        let head = make_motion_with_attach(3, -5);
        assert_eq!(attachment_offset(&body, &head), (7, -15));
    }

    #[test]
    fn head_offset_missing_attach_points() {
        let with_attach = make_motion_with_attach(10, 20);
        let without_attach = Motion {
            range1: [0; 4],
            range2: [0; 4],
            clips: Vec::new(),
            event_id: -1,
            attach_points: Vec::new(),
        };
        assert_eq!(attachment_offset(&without_attach, &with_attach), (0, 0));
        assert_eq!(attachment_offset(&with_attach, &without_attach), (0, 0));
    }

    fn make_act(action_count: usize, motions_per_action: usize) -> ActFile {
        let actions: Vec<Action> = (0..action_count)
            .map(|_| Action {
                motions: (0..motions_per_action)
                    .map(|_| Motion {
                        range1: [0; 4],
                        range2: [0; 4],
                        clips: Vec::new(),
                        event_id: -1,
                        attach_points: Vec::new(),
                    })
                    .collect(),
            })
            .collect();
        ActFile {
            version: (2, 5),
            actions,
            events: Vec::new(),
            delays: vec![4.0; action_count],
        }
    }

    #[test]
    fn crossed_event_ids_forward_and_wrap() {
        let mut act = make_act(1, 4);
        for (i, m) in act.actions[0].motions.iter_mut().enumerate() {
            m.event_id = i as i32; // frames 0..4 carry events 0..4
        }
        let mut anim = SpriteAnimationState::new(0);
        anim.set_motion_index(0);
        // first call establishes baseline at frame 0
        assert_eq!(anim.crossed_event_ids(&act, 0), Vec::<i32>::new());
        // step to frame 3: crosses frames 1,2,3
        anim.set_motion_index(3);
        assert_eq!(anim.crossed_event_ids(&act, 0), vec![1, 2, 3]);
        // wrap to frame 1: crosses frame 0 (wrap) then 1
        anim.set_motion_index(1);
        assert_eq!(anim.crossed_event_ids(&act, 0), vec![0, 1]);
    }

    #[test]
    fn crossed_event_ids_skips_none_events() {
        let mut act = make_act(1, 3);
        act.actions[0].motions[1].event_id = -1;
        act.actions[0].motions[2].event_id = 7;
        let mut anim = SpriteAnimationState::new(0);
        anim.crossed_event_ids(&act, 0);
        anim.set_motion_index(2);
        assert_eq!(anim.crossed_event_ids(&act, 0), vec![7]);
    }

    #[test]
    fn atk_keyframe_prefers_atk_event_then_falls_back() {
        let mut act = make_act(1, 5);
        act.events = vec!["footstep.wav".into(), "atk".into()];
        act.actions[0].motions[3].event_id = 1; // frame 3 carries the "atk" event
        assert_eq!(atk_keyframe_index(&act, 0), 3);

        // No atk event -> second-to-last frame.
        let plain = make_act(1, 5);
        assert_eq!(atk_keyframe_index(&plain, 0), 3);
    }

    #[test]
    fn action_index_combines_action_direction_and_camera() {
        let act = make_act(16, 1);
        let anim = SpriteAnimationState::new(2);
        assert_eq!(anim.action_index(&act, 3), 5);
    }

    #[test]
    fn flat_action_index_direct_direction_mapping() {
        let act = make_act(16, 1);
        let mut anim = SpriteAnimationState::new(0);
        anim.set_direction(3);
        assert_eq!(anim.flat_action_index(&act), 3);
        anim.set_action(1, MotionType::Loop);
        assert_eq!(anim.flat_action_index(&act), 11);
    }

    #[test]
    fn update_advances_motion() {
        let act = make_act(8, 3);
        let mut anim = SpriteAnimationState::new(0);
        anim.update(0.5, &act, 0);
        assert_eq!(anim.motion_index, 2);
    }

    #[test]
    fn walk_frame_tracks_distance_and_holds_when_still() {
        // delay 4.0 -> frame_dist 4/6 cells; 4 motions -> cycle 2.667 cells
        let act = make_act(8, 4);
        let mut anim = SpriteAnimationState::new(0);
        anim.set_action(0, MotionType::Loop);

        anim.update_by_distance(0.7, &act, 0);
        assert_eq!(anim.motion_index(), 1);

        anim.update_by_distance(0.0, &act, 0);
        assert_eq!(anim.motion_index(), 1, "no ground travel must hold the frame");

        anim.update_by_distance(0.7, &act, 0);
        assert_eq!(anim.motion_index(), 2);
    }

    #[test]
    fn walk_frame_independent_of_delivery_chunking() {
        let act = make_act(8, 4);
        let mut smooth = SpriteAnimationState::new(0);
        let mut bursty = SpriteAnimationState::new(0);
        smooth.set_action(0, MotionType::Loop);
        bursty.set_action(0, MotionType::Loop);

        smooth.update_by_distance(1.8, &act, 0);
        for _ in 0..18 {
            bursty.update_by_distance(0.1, &act, 0);
        }
        assert_eq!(smooth.motion_index(), bursty.motion_index());
    }

    #[test]
    fn step_forward_wraps() {
        let mut anim = SpriteAnimationState::new(0);
        anim.step_forward(3);
        assert_eq!(anim.motion_index(), 1);
        anim.step_forward(3);
        assert_eq!(anim.motion_index(), 2);
        anim.step_forward(3);
        assert_eq!(anim.motion_index(), 0);
    }

    #[test]
    fn step_backward_wraps() {
        let mut anim = SpriteAnimationState::new(0);
        anim.step_backward(4);
        assert_eq!(anim.motion_index(), 3);
    }

    #[test]
    fn static_frame_holds_and_never_finishes() {
        let act = make_act(13, 6);
        let mut anim = SpriteAnimationState::new(0);
        anim.set_action(SpriteActionType::Skill as usize, MotionType::Static);
        anim.set_motion_index(4);

        for _ in 0..10 {
            anim.update(0.5, &act, 0);
        }
        assert_eq!(anim.motion_index(), 4);
        assert!(!anim.is_finished());
    }
}
