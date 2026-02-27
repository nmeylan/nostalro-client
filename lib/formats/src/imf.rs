use std::io::Cursor;

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::FormatError;

pub struct ImfMotion {
    pub priority: i32,
    pub center_x: i32,
    pub center_y: i32,
}

pub struct ImfAction {
    pub motions: Vec<ImfMotion>,
}

pub struct ImfLayer {
    pub actions: Vec<ImfAction>,
}

pub struct ImfFile {
    pub version: f32,
    pub layers: Vec<ImfLayer>,
}

impl ImfFile {
    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Cursor::new(data);

        let version = r.read_f32::<LE>()?;
        let _checksum = r.read_i32::<LE>()?;
        let max_layer = r.read_i32::<LE>()?;

        let mut layers = Vec::with_capacity((max_layer + 1) as usize);
        for _ in 0..=max_layer {
            let num_actions = r.read_i32::<LE>()?;
            let mut actions = Vec::with_capacity(num_actions as usize);
            for _ in 0..num_actions {
                let num_motions = r.read_i32::<LE>()?;
                let mut motions = Vec::with_capacity(num_motions as usize);
                for _ in 0..num_motions {
                    motions.push(ImfMotion {
                        priority: r.read_i32::<LE>()?,
                        center_x: r.read_i32::<LE>()?,
                        center_y: r.read_i32::<LE>()?,
                    });
                }
                actions.push(ImfAction { motions });
            }
            layers.push(ImfLayer { actions });
        }

        Ok(ImfFile { version, layers })
    }
}
