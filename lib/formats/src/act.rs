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

pub fn attachment_offset(body_motion: &Motion, head_motion: &Motion) -> (i32, i32) {
    match (
        body_motion.attach_points.first(),
        head_motion.attach_points.first(),
    ) {
        (Some(body), Some(head)) => (body.x - head.x, body.y - head.y),
        _ => (0, 0),
    }
}

/// Each action has 8 direction variants in the ACT file.
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

pub struct SpriteAnimationState {
    action: usize,
    direction: usize,
    motion_index: usize,
    accumulated_ms: f32,
    motion_type: MotionType,
    finished: bool,
    /// When set, overrides ACT file delay so the full animation plays in this many ms.
    motion_speed_override_ms: Option<f32>,
    /// Number of remaining full-cycle repeats before the animation finishes (0 = play once).
    remaining_repeats: u16,
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
            remaining_repeats: 0,
        }
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

    /// Sets the base action with the given motion type.
    /// Resets motion only when the action changes (or when OneShot has finished).
    pub fn set_action(&mut self, action: usize, motion_type: MotionType) {
        let changed = self.action != action;
        let restart = changed || (motion_type == MotionType::OneShot && self.finished);
        if restart {
            self.action = action;
            self.motion_index = 0;
            self.accumulated_ms = 0.0;
            self.finished = false;
            self.motion_speed_override_ms = None;
            self.remaining_repeats = 0;
        } else if self.motion_type != motion_type {
            self.finished = false;
            self.motion_speed_override_ms = None;
            self.remaining_repeats = 0;
        }
        self.motion_type = motion_type;
    }

    /// Force-start a one-shot animation with a fixed total duration.
    /// Always resets unconditionally. `start_frame` controls which frame to begin on.
    pub fn play(&mut self, action: usize, total_ms: f32, start_frame: usize) {
        self.action = action;
        self.motion_index = start_frame;
        self.accumulated_ms = 0.0;
        self.motion_type = MotionType::OneShot;
        self.finished = false;
        self.motion_speed_override_ms = Some(total_ms);
        self.remaining_repeats = 0;
    }

    /// Force-start an animation that repeats `repeat_count` times within `total_ms`.
    /// Each cycle plays all frames, then restarts. After all repeats finish, the animation
    /// marks itself as finished.
    pub fn play_repeated(&mut self, action: usize, total_ms: f32, repeat_count: u16) {
        self.action = action;
        self.motion_index = 0;
        self.accumulated_ms = 0.0;
        self.motion_type = MotionType::OneShot;
        self.finished = false;
        self.motion_speed_override_ms = Some(total_ms / repeat_count.max(1) as f32);
        self.remaining_repeats = repeat_count.saturating_sub(1);
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Sets the base action, clamping to the number of action types in the ACT file.
    pub fn set_action_clamped(&mut self, action: usize, motion_type: MotionType, act: &ActFile) {
        let max_action = act.actions.len() / 8;
        self.set_action(action % max_action.max(1), motion_type);
    }

    pub fn set_direction(&mut self, direction: u8) {
        self.direction = direction as usize % 16;
    }

    /// Computes the flat action index into the ACT file, applying camera direction offset.
    pub fn action_index(&self, act: &ActFile, camera_dir: u8) -> usize {
        let effective_dir = (camera_dir as usize + 12 - self.direction) % 8;
        (self.action * 8 + effective_dir) % act.actions.len()
    }

    /// Computes the flat action index without camera offset. Direction maps directly to ACT slot.
    pub fn flat_action_index(&self, act: &ActFile) -> usize {
        (self.action * 8 + self.direction) % act.actions.len()
    }

    pub fn reset_motion(&mut self) {
        self.motion_index = 0;
        self.accumulated_ms = 0.0;
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

        let delay_ms = if let Some(total_ms) = self.motion_speed_override_ms {
            // Distribute total animation time evenly across all frames
            if motion_count > 0 {
                total_ms / motion_count as f32
            } else {
                150.0
            }
        } else if action_idx < act.delays.len() {
            let d = act.delays[action_idx] * 25.0;
            if d > 0.0 { d } else { 150.0 }
        } else {
            150.0
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
        data.push(5); // minor = 5
        data.push(2); // major = 2 → version 2.5
        data.extend_from_slice(&1u16.to_le_bytes()); // 1 action
        data.extend_from_slice(&[0u8; 10]); // reserved

        // Action with 1 motion
        data.extend_from_slice(&1u32.to_le_bytes()); // motion_count
        // Motion
        data.extend_from_slice(&[0u8; 32]); // range1 + range2
        data.extend_from_slice(&1u32.to_le_bytes()); // 1 clip

        // Clip
        data.extend_from_slice(&10i32.to_le_bytes()); // x
        data.extend_from_slice(&20i32.to_le_bytes()); // y
        data.extend_from_slice(&0i32.to_le_bytes()); // sprite_index
        data.extend_from_slice(&0u32.to_le_bytes()); // mirror
        data.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // color
        data.extend_from_slice(&1.0f32.to_le_bytes()); // zoom_x (v2.4+)
        data.extend_from_slice(&2.0f32.to_le_bytes()); // zoom_y
        data.extend_from_slice(&90i32.to_le_bytes()); // angle
        data.extend_from_slice(&1u32.to_le_bytes()); // sprite_type
        data.extend_from_slice(&32u32.to_le_bytes()); // width (v2.5+)
        data.extend_from_slice(&64u32.to_le_bytes()); // height

        data.extend_from_slice(&(-1i32).to_le_bytes()); // event_id
        data.extend_from_slice(&0u32.to_le_bytes()); // attach_point_count

        // Events (v2.1+)
        data.extend_from_slice(&1u32.to_le_bytes()); // 1 event
        let mut event_name = [0u8; 40];
        event_name[..4].copy_from_slice(b"atk1");
        data.extend_from_slice(&event_name);

        // Delays (v2.2+)
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
}
