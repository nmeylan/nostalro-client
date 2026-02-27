use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::{FormatError, read_string};

pub struct StrKeyframe {
    pub frame_index: i32,
    pub frame_type: i32,
    pub offset: [f32; 2],
    pub uv: [f32; 8],
    pub xy: [f32; 8],
    pub texture_index: f32,
    pub anim_type: i32,
    pub delay: f32,
    pub angle: f32,
    pub color: [f32; 4],
    pub src_blend: i32,
    pub dst_blend: i32,
    pub mt_preset: i32,
}

pub struct StrLayer {
    pub textures: Vec<String>,
    pub keyframes: Vec<StrKeyframe>,
}

pub struct StrFile {
    pub version: (u8, u8),
    pub fps: u32,
    pub max_key: u32,
    pub layers: Vec<StrLayer>,
}

impl StrFile {
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

            let key_count = r.read_i32::<LE>()? as usize;
            let mut keyframes = Vec::with_capacity(key_count);
            for _ in 0..key_count {
                let frame_index = r.read_i32::<LE>()?;
                let frame_type = r.read_i32::<LE>()?;
                let offset = [r.read_f32::<LE>()?, r.read_f32::<LE>()?];

                let mut uv = [0f32; 8];
                for v in &mut uv { *v = r.read_f32::<LE>()?; }
                let mut xy = [0f32; 8];
                for v in &mut xy { *v = r.read_f32::<LE>()?; }

                let texture_index = r.read_f32::<LE>()?;
                let anim_type = r.read_i32::<LE>()?;
                let delay = r.read_f32::<LE>()?;
                let angle = r.read_f32::<LE>()?;

                let mut color = [0f32; 4];
                for v in &mut color { *v = r.read_f32::<LE>()?; }

                let src_blend = r.read_i32::<LE>()?;
                let dst_blend = r.read_i32::<LE>()?;
                let mt_preset = r.read_i32::<LE>()?;

                keyframes.push(StrKeyframe {
                    frame_index, frame_type, offset, uv, xy,
                    texture_index, anim_type, delay, angle, color,
                    src_blend, dst_blend, mt_preset,
                });
            }

            layers.push(StrLayer { textures, keyframes });
        }

        Ok(StrFile {
            version: (ver_major, ver_minor),
            fps,
            max_key,
            layers,
        })
    }
}
