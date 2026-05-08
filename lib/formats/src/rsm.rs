use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::{
    FormatError, Mat3, Vec3, read_length_string, read_string, read_vec3, version_at_least,
};

pub struct TexVertex {
    pub color: u32,
    pub u: f32,
    pub v: f32,
}

pub struct RsmFace {
    pub vertex_ids: [u16; 3],
    pub tex_vertex_ids: [u16; 3],
    pub texture_index: u16,
    pub padding: u16,
    pub two_sided: i32,
    pub smooth_group: i32,
    pub extra_smooth_groups: Vec<i32>,
}

pub struct RotKeyframe {
    pub frame: i32,
    pub quaternion: [f32; 4],
}

pub struct PosKeyframe {
    pub frame: i32,
    pub position: Vec3,
    pub _reserved: f32,
}

pub struct ScaleKeyframe {
    pub frame: i32,
    pub scale: Vec3,
    pub _reserved: f32,
}

pub struct TextureFrameData {
    pub frame: i32,
    pub value: f32,
}

pub struct TextureKeyframeData {
    pub operation_type: u32,
    pub frames: Vec<TextureFrameData>,
}

pub struct TexturesKeyframeData {
    pub texture_index: u32,
    pub keyframes: Vec<TextureKeyframeData>,
}

pub struct RsmNode {
    pub name: String,
    pub parent_name: String,
    pub texture_ids: Vec<u32>,
    pub texture_names: Vec<String>,
    pub local_transform: Mat3,
    pub translation1: Option<Vec3>,
    pub translation2: Vec3,
    pub rotation_angle: Option<f32>,
    pub rotation_axis: Option<Vec3>,
    pub scale: Option<Vec3>,
    pub vertices: Vec<Vec3>,
    pub tex_vertices: Vec<TexVertex>,
    pub faces: Vec<RsmFace>,
    pub scale_keyframes: Vec<ScaleKeyframe>,
    pub rot_keyframes: Vec<RotKeyframe>,
    pub translation_keyframes: Vec<PosKeyframe>,
    pub textures_keyframes: Vec<TexturesKeyframeData>,
}

pub struct RsmFile {
    pub version: (u8, u8),
    pub anim_length: u32,
    pub shade_type: u32,
    pub alpha: Option<u8>,
    pub fps: Option<f32>,
    pub textures: Vec<String>,
    pub root_node_names: Vec<String>,
    pub nodes: Vec<RsmNode>,
}

impl RsmFile {
    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Cursor::new(data);

        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != b"GRSM" {
            return Err(FormatError::InvalidMagic);
        }

        let ver_major = r.read_u8()?;
        let ver_minor = r.read_u8()?;
        let version = (ver_major, ver_minor);

        let anim_length = r.read_u32::<LE>()?;
        let shade_type = r.read_u32::<LE>()?;

        let alpha = if version_at_least(version, 1, 4) {
            Some(r.read_u8()?)
        } else {
            None
        };

        if !version_at_least(version, 2, 2) {
            let mut reserved = [0u8; 16];
            r.read_exact(&mut reserved)?;
        }

        let fps = if version_at_least(version, 2, 2) {
            Some(r.read_f32::<LE>()?)
        } else {
            None
        };

        // Textures (global, v < 2.3 only)
        let textures = if !version_at_least(version, 2, 3) {
            let count = r.read_u32::<LE>()? as usize;
            let mut names = Vec::with_capacity(count);
            for _ in 0..count {
                names.push(read_versioned_string(&mut r, version)?);
            }
            names
        } else {
            Vec::new()
        };

        // Root node name(s)
        let root_node_names = if version_at_least(version, 2, 2) {
            let count = r.read_u32::<LE>()? as usize;
            let mut names = Vec::with_capacity(count);
            for _ in 0..count {
                names.push(read_versioned_string(&mut r, version)?);
            }
            names
        } else {
            vec![read_versioned_string(&mut r, version)?]
        };

        // Nodes
        let node_count = r.read_u32::<LE>()? as usize;
        let mut nodes = Vec::with_capacity(node_count);
        for _ in 0..node_count {
            nodes.push(parse_node(&mut r, version)?);
        }

        Ok(RsmFile {
            version,
            anim_length,
            shade_type,
            alpha,
            fps,
            textures,
            root_node_names,
            nodes,
        })
    }
}

fn read_versioned_string(r: &mut Cursor<&[u8]>, version: (u8, u8)) -> Result<String, FormatError> {
    if version_at_least(version, 2, 2) {
        read_length_string(r)
    } else {
        read_string(r, 40)
    }
}

fn parse_node(r: &mut Cursor<&[u8]>, version: (u8, u8)) -> Result<RsmNode, FormatError> {
    let name = read_versioned_string(r, version)?;
    let parent_name = read_versioned_string(r, version)?;

    // Texture references
    let mut texture_ids = Vec::new();
    let mut texture_names = Vec::new();

    if !version_at_least(version, 2, 3) {
        let count = r.read_u32::<LE>()? as usize;
        for _ in 0..count {
            texture_ids.push(r.read_u32::<LE>()?);
        }
    } else {
        let count = r.read_u32::<LE>()? as usize;
        for _ in 0..count {
            texture_names.push(read_versioned_string(r, version)?);
        }
    }

    // Offset matrix (3x3)
    let mut local_transform = [[0.0f32; 3]; 3];
    for row in &mut local_transform {
        for val in row.iter_mut() {
            *val = r.read_f32::<LE>()?;
        }
    }

    let translation1 = if !version_at_least(version, 2, 2) {
        Some(read_vec3(r)?)
    } else {
        None
    };

    let translation2 = read_vec3(r)?;

    let rotation_angle = if !version_at_least(version, 2, 2) {
        Some(r.read_f32::<LE>()?)
    } else {
        None
    };

    let rotation_axis = if !version_at_least(version, 2, 2) {
        Some(read_vec3(r)?)
    } else {
        None
    };

    let scale = if !version_at_least(version, 2, 2) {
        Some(read_vec3(r)?)
    } else {
        None
    };

    // Vertices
    let vertex_count = r.read_u32::<LE>()? as usize;
    let mut vertices = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        vertices.push(read_vec3(r)?);
    }

    // Texture coordinates
    let tex_vertex_count = r.read_u32::<LE>()? as usize;
    let mut tex_vertices = Vec::with_capacity(tex_vertex_count);
    for _ in 0..tex_vertex_count {
        let color = if version_at_least(version, 1, 2) {
            r.read_u32::<LE>()?
        } else {
            0xFFFFFFFF
        };
        let u = r.read_f32::<LE>()?;
        let v = r.read_f32::<LE>()?;
        tex_vertices.push(TexVertex { color, u, v });
    }

    // Faces
    let face_count = r.read_u32::<LE>()? as usize;
    let mut faces = Vec::with_capacity(face_count);
    for _ in 0..face_count {
        let face_length = if version_at_least(version, 2, 2) {
            Some(r.read_u32::<LE>()?)
        } else {
            None
        };

        let vertex_ids = [
            r.read_u16::<LE>()?,
            r.read_u16::<LE>()?,
            r.read_u16::<LE>()?,
        ];
        let tex_vertex_ids = [
            r.read_u16::<LE>()?,
            r.read_u16::<LE>()?,
            r.read_u16::<LE>()?,
        ];
        let texture_index = r.read_u16::<LE>()?;
        let padding = r.read_u16::<LE>()?;
        let two_sided = r.read_i32::<LE>()?;
        let smooth_group = r.read_i32::<LE>()?;

        // v2.2+: extra smooth group data
        let extra_smooth_groups = if let Some(length) = face_length {
            const RSM_FACE_BASE_SIZE: u32 = 24; // 6*u16 + u16 + u16 + i32 + i32
            let extra_count = (length.saturating_sub(RSM_FACE_BASE_SIZE) / 4) as usize;
            let mut extra = Vec::with_capacity(extra_count);
            for _ in 0..extra_count {
                extra.push(r.read_i32::<LE>()?);
            }
            extra
        } else {
            Vec::new()
        };

        faces.push(RsmFace {
            vertex_ids,
            tex_vertex_ids,
            texture_index,
            padding,
            two_sided,
            smooth_group,
            extra_smooth_groups,
        });
    }

    // Scale keyframes (v >= 1.6)
    let scale_keyframes = if version_at_least(version, 1, 6) {
        let count = r.read_u32::<LE>()? as usize;
        let mut kf = Vec::with_capacity(count);
        for _ in 0..count {
            kf.push(ScaleKeyframe {
                frame: r.read_i32::<LE>()?,
                scale: read_vec3(r)?,
                _reserved: r.read_f32::<LE>()?,
            });
        }
        kf
    } else {
        Vec::new()
    };

    // Rotation keyframes (always present)
    let rot_count = r.read_u32::<LE>()? as usize;
    let mut rot_keyframes = Vec::with_capacity(rot_count);
    for _ in 0..rot_count {
        rot_keyframes.push(RotKeyframe {
            frame: r.read_i32::<LE>()?,
            quaternion: [
                r.read_f32::<LE>()?,
                r.read_f32::<LE>()?,
                r.read_f32::<LE>()?,
                r.read_f32::<LE>()?,
            ],
        });
    }

    // Translation keyframes (v >= 2.2)
    let translation_keyframes = if version_at_least(version, 2, 2) {
        let count = r.read_u32::<LE>()? as usize;
        let mut kf = Vec::with_capacity(count);
        for _ in 0..count {
            kf.push(PosKeyframe {
                frame: r.read_i32::<LE>()?,
                position: read_vec3(r)?,
                _reserved: r.read_f32::<LE>()?,
            });
        }
        kf
    } else {
        Vec::new()
    };

    // Texture keyframes (v >= 2.3)
    let textures_keyframes = if version_at_least(version, 2, 3) {
        let count = r.read_u32::<LE>()? as usize;
        let mut tkfs = Vec::with_capacity(count);
        for _ in 0..count {
            let texture_index = r.read_u32::<LE>()?;
            let kf_count = r.read_u32::<LE>()? as usize;
            let mut keyframes = Vec::with_capacity(kf_count);
            for _ in 0..kf_count {
                let operation_type = r.read_u32::<LE>()?;
                let frame_count = r.read_u32::<LE>()? as usize;
                let mut frames = Vec::with_capacity(frame_count);
                for _ in 0..frame_count {
                    frames.push(TextureFrameData {
                        frame: r.read_i32::<LE>()?,
                        value: r.read_f32::<LE>()?,
                    });
                }
                keyframes.push(TextureKeyframeData {
                    operation_type,
                    frames,
                });
            }
            tkfs.push(TexturesKeyframeData {
                texture_index,
                keyframes,
            });
        }
        tkfs
    } else {
        Vec::new()
    };

    Ok(RsmNode {
        name,
        parent_name,
        texture_ids,
        texture_names,
        local_transform,
        translation1,
        translation2,
        rotation_angle,
        rotation_axis,
        scale,
        vertices,
        tex_vertices,
        faces,
        scale_keyframes,
        rot_keyframes,
        translation_keyframes,
        textures_keyframes,
    })
}
