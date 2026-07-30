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

const BODY_LAYER: usize = 0;
const HEAD_LAYER: usize = 1;

/// Per (action, motion) draw order of a player composite's body and head, with
/// `true` meaning the body covers the head.
#[derive(Debug, Clone, Default)]
pub struct ImfLayerOrder {
    actions: Vec<Vec<bool>>,
}

impl ImfLayerOrder {
    pub fn from_file(file: &ImfFile) -> Option<Self> {
        let body = file.layers.get(BODY_LAYER)?;
        let head = file.layers.get(HEAD_LAYER)?;
        let actions = body
            .actions
            .iter()
            .enumerate()
            .map(|(action_idx, action)| {
                action
                    .motions
                    .iter()
                    .enumerate()
                    .map(|(motion_idx, motion)| {
                        motion.priority == 0
                            && head
                                .actions
                                .get(action_idx)
                                .and_then(|a| a.motions.get(motion_idx))
                                .is_some()
                    })
                    .collect()
            })
            .collect();
        Some(Self { actions })
    }

    pub fn body_over_head(&self, action_idx: usize, motion_idx: usize) -> bool {
        self.actions
            .get(action_idx)
            .and_then(|motions| motions.get(motion_idx))
            .copied()
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic(layers: &[&[i32]]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&1.01f32.to_le_bytes());
        out.extend_from_slice(&0i32.to_le_bytes());
        out.extend_from_slice(&(layers.len() as i32 - 1).to_le_bytes());
        for priorities in layers {
            out.extend_from_slice(&1i32.to_le_bytes());
            out.extend_from_slice(&(priorities.len() as i32).to_le_bytes());
            for p in *priorities {
                out.extend_from_slice(&p.to_le_bytes());
                out.extend_from_slice(&0i32.to_le_bytes());
                out.extend_from_slice(&0i32.to_le_bytes());
            }
        }
        out
    }

    #[test]
    fn layer_order_follows_body_priority() {
        let file = ImfFile::parse(&synthetic(&[&[1, 0], &[0, 1]])).unwrap();
        let order = ImfLayerOrder::from_file(&file).unwrap();

        assert!(!order.body_over_head(0, 0));
        assert!(order.body_over_head(0, 1));
        assert!(!order.body_over_head(0, 2));
        assert!(!order.body_over_head(1, 0));
    }
}
