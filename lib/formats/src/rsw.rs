use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::{FormatError, Vec3, read_string, read_vec3, version_at_least};

pub struct WaterSettings {
    pub level: Option<f32>,
    pub water_type: Option<i32>,
    pub wave_height: Option<f32>,
    pub wave_speed: Option<f32>,
    pub wave_pitch: Option<f32>,
    pub anim_speed: Option<u32>,
}

pub struct LightSettings {
    pub longitude: Option<i32>,
    pub latitude: Option<i32>,
    pub diffuse: Option<Vec3>,
    pub ambient: Option<Vec3>,
    pub shadow_map_alpha: Option<f32>,
}

pub struct RswModel {
    pub name: Option<String>,
    pub anim_type: Option<i32>,
    pub anim_speed: Option<f32>,
    pub block_type: Option<i32>,
    pub model_name: String,
    pub node_name: String,
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

pub struct RswLight {
    pub name: String,
    pub position: Vec3,
    pub color: Vec3,
    pub range: f32,
}

pub struct RswSound {
    pub name: String,
    pub file_name: String,
    pub position: Vec3,
    pub volume: f32,
    pub width: u32,
    pub height: u32,
    pub range: f32,
    pub cycle: f32,
}

pub struct RswEffect {
    pub name: String,
    pub position: Vec3,
    pub effect_type: u32,
    pub emit_speed: f32,
    pub param: [f32; 4],
}

pub enum RswObject {
    Model(RswModel),
    Light(RswLight),
    Sound(RswSound),
    Effect(RswEffect),
}

pub struct RswFile {
    pub version: (u8, u8),
    pub ini_file: String,
    pub gnd_file: String,
    pub gat_file: String,
    pub source_file: Option<String>,
    pub water: WaterSettings,
    pub light: LightSettings,
    pub ground_top: Option<i32>,
    pub ground_bottom: Option<i32>,
    pub ground_left: Option<i32>,
    pub ground_right: Option<i32>,
    pub objects: Vec<RswObject>,
}

impl RswFile {
    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Cursor::new(data);

        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != b"GRSW" {
            return Err(FormatError::InvalidMagic);
        }

        let ver_major = r.read_u8()?;
        let ver_minor = r.read_u8()?;
        let version = (ver_major, ver_minor);

        // Build version (v >= 2.5)
        let build_version = if version_at_least(version, 2, 5) {
            Some(r.read_u32::<LE>()?)
        } else {
            None
        };

        // Unknown byte (v >= 2.2)
        if version_at_least(version, 2, 2) {
            let _unknown = r.read_u8()?;
        }

        let ini_file = read_string(&mut r, 40)?;
        let gnd_file = read_string(&mut r, 40)?;
        let gat_file = read_string(&mut r, 40)?;

        let source_file = if version_at_least(version, 1, 4) {
            Some(read_string(&mut r, 40)?)
        } else {
            None
        };

        // Water settings (v < 2.6)
        let water = if !version_at_least(version, 2, 6) {
            WaterSettings {
                level: if version_at_least(version, 1, 3) { Some(r.read_f32::<LE>()?) } else { None },
                water_type: if version_at_least(version, 1, 8) { Some(r.read_i32::<LE>()?) } else { None },
                wave_height: if version_at_least(version, 1, 8) { Some(r.read_f32::<LE>()?) } else { None },
                wave_speed: if version_at_least(version, 1, 8) { Some(r.read_f32::<LE>()?) } else { None },
                wave_pitch: if version_at_least(version, 1, 8) { Some(r.read_f32::<LE>()?) } else { None },
                anim_speed: if version_at_least(version, 1, 9) { Some(r.read_u32::<LE>()?) } else { None },
            }
        } else {
            WaterSettings {
                level: None, water_type: None, wave_height: None,
                wave_speed: None, wave_pitch: None, anim_speed: None,
            }
        };

        // Light settings
        let light = LightSettings {
            longitude: if version_at_least(version, 1, 5) { Some(r.read_i32::<LE>()?) } else { None },
            latitude: if version_at_least(version, 1, 5) { Some(r.read_i32::<LE>()?) } else { None },
            diffuse: if version_at_least(version, 1, 5) { Some(read_vec3(&mut r)?) } else { None },
            ambient: if version_at_least(version, 1, 5) { Some(read_vec3(&mut r)?) } else { None },
            shadow_map_alpha: if version_at_least(version, 1, 7) { Some(r.read_f32::<LE>()?) } else { None },
        };

        // Ground bounds (v >= 1.6)
        let ground_top = if version_at_least(version, 1, 6) { Some(r.read_i32::<LE>()?) } else { None };
        let ground_bottom = if version_at_least(version, 1, 6) { Some(r.read_i32::<LE>()?) } else { None };
        let ground_left = if version_at_least(version, 1, 6) { Some(r.read_i32::<LE>()?) } else { None };
        let ground_right = if version_at_least(version, 1, 6) { Some(r.read_i32::<LE>()?) } else { None };

        // Objects
        let object_count = r.read_u32::<LE>()? as usize;
        let mut objects = Vec::with_capacity(object_count);
        for _ in 0..object_count {
            let obj_type = r.read_i32::<LE>()?;
            match obj_type {
                1 => objects.push(RswObject::Model(RswModel::parse(&mut r, version, build_version)?)),
                2 => objects.push(RswObject::Light(RswLight::parse(&mut r)?)),
                3 => objects.push(RswObject::Sound(RswSound::parse(&mut r, version)?)),
                4 => objects.push(RswObject::Effect(RswEffect::parse(&mut r)?)),
                _ => {
                    tracing::warn!("unknown RSW object type {obj_type}, stopping object parse");
                    break;
                }
            }
        }

        Ok(RswFile {
            version,
            ini_file,
            gnd_file,
            gat_file,
            source_file,
            water,
            light,
            ground_top,
            ground_bottom,
            ground_left,
            ground_right,
            objects,
        })
    }
}

const DEFAULT_SOUND_CYCLE: f32 = 4.0;

impl RswModel {
    fn parse(r: &mut Cursor<&[u8]>, version: (u8, u8), build_version: Option<u32>) -> Result<Self, FormatError> {
    let (name, anim_type, anim_speed, block_type) = if version_at_least(version, 1, 3) {
        (
            Some(read_string(r, 40)?),
            Some(r.read_i32::<LE>()?),
            Some(r.read_f32::<LE>()?),
            Some(r.read_i32::<LE>()?),
        )
    } else {
        (None, None, None, None)
    };

    // v2.6 build >= 186: unknown byte
    if version_at_least(version, 2, 6) {
        if let Some(bv) = build_version {
            if bv >= 186 {
                let _unknown = r.read_u8()?;
            }
        }
    }

    let model_name = read_string(r, 80)?;
    let node_name = read_string(r, 80)?;

    let position = read_vec3(r)?;
    let rotation = read_vec3(r)?;
    let scale = read_vec3(r)?;

    Ok(RswModel {
        name, anim_type, anim_speed, block_type,
        model_name, node_name, position, rotation, scale,
    })
    }
}

impl RswLight {
    fn parse(r: &mut Cursor<&[u8]>) -> Result<Self, FormatError> {
    let name = read_string(r, 80)?;
    let position = read_vec3(r)?;
    let mut color = read_vec3(r)?;
    // Clamp color channels to [0.0, 1.0]
    for c in &mut color {
        *c = c.clamp(0.0, 1.0);
    }
    let range = r.read_f32::<LE>()?;
    Ok(RswLight { name, position, color, range })
    }
}

impl RswSound {
    fn parse(r: &mut Cursor<&[u8]>, version: (u8, u8)) -> Result<Self, FormatError> {
    let name = read_string(r, 80)?;
    let file_name = read_string(r, 80)?;
    let position = read_vec3(r)?;
    let volume = r.read_f32::<LE>()?;
    let width = r.read_u32::<LE>()?;
    let height = r.read_u32::<LE>()?;
    let range = r.read_f32::<LE>()?;
    let cycle = if version_at_least(version, 2, 0) {
        r.read_f32::<LE>()?
    } else {
        DEFAULT_SOUND_CYCLE
    };
    Ok(RswSound { name, file_name, position, volume, width, height, range, cycle })
    }
}

impl RswEffect {
    fn parse(r: &mut Cursor<&[u8]>) -> Result<Self, FormatError> {
    let name = read_string(r, 80)?;
    let position = read_vec3(r)?;
    let effect_type = r.read_u32::<LE>()?;
    let emit_speed = r.read_f32::<LE>()?;
    let param = [
        r.read_f32::<LE>()?,
        r.read_f32::<LE>()?,
        r.read_f32::<LE>()?,
        r.read_f32::<LE>()?,
    ];
    Ok(RswEffect { name, position, effect_type, emit_speed, param })
    }
}
