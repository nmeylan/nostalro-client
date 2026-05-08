use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::{FormatError, read_string};

pub struct EffectFrame {
    pub frame_index: i32,
    pub frame_type: i32,
    pub offset: [f32; 2],
    pub tex_coords: [f32; 8],
    pub positions: [f32; 8],
    pub texture_index: f32,
    pub animation_mode: i32,
    pub delay: f32,
    pub angle: f32,
    pub color: [f32; 4],
    pub blend_src: i32,
    pub blend_dst: i32,
    pub multi_texture: i32,
}

pub struct EffectLayer {
    pub textures: Vec<String>,
    pub frames: Vec<EffectFrame>,
}

pub struct StrEffectFile {
    pub version: (u8, u8),
    pub fps: u32,
    pub max_key: u32,
    pub layers: Vec<EffectLayer>,
}

impl StrEffectFile {
    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Cursor::new(data);

        let mut sig = [0u8; 4];
        r.read_exact(&mut sig)?;
        if &sig != b"STRM" {
            return Err(FormatError::InvalidMagic);
        }

        let ver_major = r.read_u8()?;
        let ver_minor = r.read_u8()?;

        // 2 skip bytes
        let mut skip = [0u8; 2];
        r.read_exact(&mut skip)?;

        let fps = r.read_u32::<LE>()?;
        let max_key = r.read_u32::<LE>()?;
        let layer_count = r.read_u32::<LE>()? as usize;

        // 16 reserved bytes
        let mut reserved = [0u8; 16];
        r.read_exact(&mut reserved)?;

        let mut layers = Vec::with_capacity(layer_count);
        for _ in 0..layer_count {
            let tex_count = r.read_i32::<LE>()? as usize;
            let mut textures = Vec::with_capacity(tex_count);
            for _ in 0..tex_count {
                textures.push(read_string(&mut r, 128)?);
            }

            let frame_count = r.read_i32::<LE>()? as usize;
            let mut frames = Vec::with_capacity(frame_count);
            for _ in 0..frame_count {
                let frame_index = r.read_i32::<LE>()?;
                let frame_type = r.read_i32::<LE>()?;
                let offset = [r.read_f32::<LE>()?, r.read_f32::<LE>()?];

                let mut tex_coords = [0f32; 8];
                for v in &mut tex_coords {
                    *v = r.read_f32::<LE>()?;
                }
                let mut positions = [0f32; 8];
                for v in &mut positions {
                    *v = r.read_f32::<LE>()?;
                }

                let texture_index = r.read_f32::<LE>()?;
                let animation_mode = r.read_i32::<LE>()?;
                let delay = r.read_f32::<LE>()?;
                let angle = r.read_f32::<LE>()?;

                let mut color = [0f32; 4];
                for v in &mut color {
                    *v = r.read_f32::<LE>()?;
                }

                let blend_src = r.read_i32::<LE>()?;
                let blend_dst = r.read_i32::<LE>()?;
                let multi_texture = r.read_i32::<LE>()?;

                frames.push(EffectFrame {
                    frame_index,
                    frame_type,
                    offset,
                    tex_coords,
                    positions,
                    texture_index,
                    animation_mode,
                    delay,
                    angle,
                    color,
                    blend_src,
                    blend_dst,
                    multi_texture,
                });
            }

            layers.push(EffectLayer { textures, frames });
        }

        Ok(StrEffectFile {
            version: (ver_major, ver_minor),
            fps,
            max_key,
            layers,
        })
    }
}
