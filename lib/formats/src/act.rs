use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::{Color, FormatError, read_string, version_at_least};

pub struct AttachPoint {
    pub ignored: u32,
    pub x: i32,
    pub y: i32,
    pub attribute: u32,
}

pub struct SprClip {
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
    pub clips: Vec<SprClip>,
    pub event_id: i32,
    pub attach_points: Vec<AttachPoint>,
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
        let mut reserved = [0u8; 10];
        r.read_exact(&mut reserved)?;

        let mut actions = Vec::with_capacity(action_count);
        for _ in 0..action_count {
            let motion_count = r.read_u32::<LE>()? as usize;
            let mut motions = Vec::with_capacity(motion_count);
            for _ in 0..motion_count {
                let mut range1 = [0i32; 4];
                let mut range2 = [0i32; 4];
                for v in &mut range1 { *v = r.read_i32::<LE>()?; }
                for v in &mut range2 { *v = r.read_i32::<LE>()?; }

                let clip_count = r.read_u32::<LE>()? as usize;
                let mut clips = Vec::with_capacity(clip_count);
                for _ in 0..clip_count {
                    let x = r.read_i32::<LE>()?;
                    let y = r.read_i32::<LE>()?;
                    let sprite_index = r.read_i32::<LE>()?;
                    let mirror = r.read_u32::<LE>()?;

                    let (color, zoom_x, zoom_y, angle, sprite_type) = if version_at_least(version, 2, 0) {
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

                    clips.push(SprClip {
                        x, y, sprite_index, mirror, color,
                        zoom_x, zoom_y, angle, sprite_type,
                        width, height,
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
                        points.push(AttachPoint {
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

                motions.push(Motion { range1, range2, clips, event_id, attach_points });
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

        Ok(ActFile { version, actions, events, delays })
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
}
