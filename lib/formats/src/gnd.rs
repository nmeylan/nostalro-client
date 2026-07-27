use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::{FormatError, read_string, version_at_least};

pub struct Lightmap {
    pub shadow: [u8; 64],
    pub color: [u8; 192],
}

pub struct GndSurface {
    pub tex_u: [f32; 4],
    pub tex_v: [f32; 4],
    pub texture_id: i16,
    pub lightmap_id: i16,
    pub color_bgra: [u8; 4],
}

pub struct GndCell {
    pub height_sw: f32,
    pub height_se: f32,
    pub height_nw: f32,
    pub height_ne: f32,
    pub surface_up: i32,
    pub surface_south: i32,
    pub surface_east: i32,
}

pub struct GndFile {
    pub version: (u8, u8),
    pub width: i32,
    pub height: i32,
    pub zoom: f32,
    pub textures: Vec<String>,
    pub lightmaps: Vec<Lightmap>,
    pub surfaces: Vec<GndSurface>,
    pub cells: Vec<GndCell>,
}

impl GndFile {
    /// Whether the lightmaps carry the 8x8 shadow + RGB layout this parser reads.
    /// Older files keep their lightmaps in a layout we skip, leaving zeroes behind.
    pub fn has_lightmap_data(&self) -> bool {
        version_at_least(self.version, 1, 7) && !self.lightmaps.is_empty()
    }

    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Cursor::new(data);

        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != b"GRGN" {
            return Err(FormatError::InvalidMagic);
        }

        let ver_major = r.read_u8()?;
        let ver_minor = r.read_u8()?;
        let version = (ver_major, ver_minor);

        let width = r.read_i32::<LE>()?;
        let height = r.read_i32::<LE>()?;
        let zoom = r.read_f32::<LE>()?;

        let texture_count = r.read_i32::<LE>()? as usize;
        let texture_name_len = r.read_i32::<LE>()? as usize;
        let mut textures = Vec::with_capacity(texture_count);
        for _ in 0..texture_count {
            textures.push(read_string(&mut r, texture_name_len)?);
        }

        let lightmap_count = r.read_i32::<LE>()? as usize;
        let lmap_width = r.read_i32::<LE>()?;
        let lmap_height = r.read_i32::<LE>()?;
        let _lmap_cells_per_grid = r.read_i32::<LE>()?;

        let mut lightmaps = Vec::with_capacity(lightmap_count);
        let lmap_pixel_count = (lmap_width * lmap_height) as usize;
        for _ in 0..lightmap_count {
            if version_at_least(version, 1, 7) {
                let mut shadow = [0u8; 64];
                let mut color = [0u8; 192];
                let skip_count = lmap_pixel_count * 4 - 64 - 192;
                r.read_exact(&mut shadow)?;
                r.read_exact(&mut color)?;
                if skip_count > 0 {
                    let mut skip = vec![0u8; skip_count];
                    r.read_exact(&mut skip)?;
                }
                lightmaps.push(Lightmap { shadow, color });
            } else {
                let skip_count = lmap_pixel_count * 4;
                let mut skip = vec![0u8; skip_count];
                r.read_exact(&mut skip)?;
                lightmaps.push(Lightmap {
                    shadow: [0; 64],
                    color: [0; 192],
                });
            }
        }

        let surface_count = r.read_i32::<LE>()? as usize;
        let mut surfaces = Vec::with_capacity(surface_count);
        for _ in 0..surface_count {
            let mut u = [0f32; 4];
            let mut v = [0f32; 4];
            for val in &mut u {
                *val = r.read_f32::<LE>()?;
            }
            for val in &mut v {
                *val = r.read_f32::<LE>()?;
            }
            let texture_id = r.read_i16::<LE>()?;
            let lightmap_id = r.read_i16::<LE>()?;
            let mut color_bgra = [0u8; 4];
            r.read_exact(&mut color_bgra)?;
            surfaces.push(GndSurface {
                tex_u: u,
                tex_v: v,
                texture_id,
                lightmap_id,
                color_bgra,
            });
        }

        let cell_count = (width as usize) * (height as usize);
        let mut cells = Vec::with_capacity(cell_count);
        for _ in 0..cell_count {
            let height_sw = r.read_f32::<LE>()?;
            let height_se = r.read_f32::<LE>()?;
            let height_nw = r.read_f32::<LE>()?;
            let height_ne = r.read_f32::<LE>()?;

            let (up, south, east) = if version_at_least(version, 1, 7) {
                (
                    r.read_i32::<LE>()?,
                    r.read_i32::<LE>()?,
                    r.read_i32::<LE>()?,
                )
            } else {
                (
                    r.read_i16::<LE>()? as i32,
                    r.read_i16::<LE>()? as i32,
                    r.read_i16::<LE>()? as i32,
                )
            };

            cells.push(GndCell {
                height_sw,
                height_se,
                height_nw,
                height_ne,
                surface_up: up,
                surface_south: south,
                surface_east: east,
            });
        }

        Ok(GndFile {
            version,
            width,
            height,
            zoom,
            textures,
            lightmaps,
            surfaces,
            cells,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_gnd_header(version: (u8, u8), width: i32, height: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"GRGN");
        data.push(version.0);
        data.push(version.1);
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        data.extend_from_slice(&10.0f32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&40i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&8i32.to_le_bytes());
        data.extend_from_slice(&8i32.to_le_bytes());
        data.extend_from_slice(&1i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data
    }

    #[test]
    fn parse_v1_6_uses_i16_surface_indices() {
        let mut data = build_gnd_header((1, 6), 1, 1);
        for _ in 0..4 {
            data.extend_from_slice(&1.0f32.to_le_bytes());
        }
        data.extend_from_slice(&0i16.to_le_bytes());
        data.extend_from_slice(&(-1i16).to_le_bytes());
        data.extend_from_slice(&(-1i16).to_le_bytes());

        let gnd = GndFile::parse(&data).unwrap();
        assert_eq!(gnd.cells[0].surface_up, 0);
        assert_eq!(gnd.cells[0].surface_south, -1);
    }

    #[test]
    fn parse_v1_7_uses_i32_surface_indices() {
        let mut data = build_gnd_header((1, 7), 1, 1);
        for _ in 0..4 {
            data.extend_from_slice(&2.0f32.to_le_bytes());
        }
        data.extend_from_slice(&5i32.to_le_bytes());
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&3i32.to_le_bytes());

        let gnd = GndFile::parse(&data).unwrap();
        assert_eq!(gnd.cells[0].surface_up, 5);
        assert_eq!(gnd.cells[0].surface_east, 3);
        assert_eq!(gnd.cells[0].height_sw, 2.0);
    }
}
