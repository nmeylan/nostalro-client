use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::{FormatError, read_string, version_at_least};

pub struct Lightmap {
    pub intensity: [u8; 64],
    pub specular: [u8; 192],
}

pub struct GndSurface {
    pub u: [f32; 4],
    pub v: [f32; 4],
    pub texture_id: i16,
    pub lightmap_id: i16,
    pub color_bgra: [u8; 4],
}

pub struct GndCell {
    pub height: [f32; 4],
    pub top_surface: i32,
    pub front_surface: i32,
    pub right_surface: i32,
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

        // Textures
        let texture_count = r.read_i32::<LE>()? as usize;
        let texture_name_len = r.read_i32::<LE>()? as usize;
        let mut textures = Vec::with_capacity(texture_count);
        for _ in 0..texture_count {
            textures.push(read_string(&mut r, texture_name_len)?);
        }

        // Lightmaps
        let lightmap_count = r.read_i32::<LE>()? as usize;
        let lmap_width = r.read_i32::<LE>()?;
        let lmap_height = r.read_i32::<LE>()?;
        let _lmap_cells_per_grid = r.read_i32::<LE>()?;

        let mut lightmaps = Vec::with_capacity(lightmap_count);
        let lmap_pixel_count = (lmap_width * lmap_height) as usize;
        for _ in 0..lightmap_count {
            if version_at_least(version, 1, 7) {
                // 64 bytes intensity + 192 bytes specular RGB
                let mut intensity = [0u8; 64];
                let mut specular = [0u8; 192];
                let skip_count = lmap_pixel_count * 4 - 64 - 192;
                r.read_exact(&mut intensity)?;
                r.read_exact(&mut specular)?;
                if skip_count > 0 {
                    let mut skip = vec![0u8; skip_count];
                    r.read_exact(&mut skip)?;
                }
                lightmaps.push(Lightmap { intensity, specular });
            } else {
                // v < 1.7: skip lightmap data
                let skip_count = lmap_pixel_count * 4;
                let mut skip = vec![0u8; skip_count];
                r.read_exact(&mut skip)?;
                lightmaps.push(Lightmap {
                    intensity: [0; 64],
                    specular: [0; 192],
                });
            }
        }

        // Surfaces
        let surface_count = r.read_i32::<LE>()? as usize;
        let mut surfaces = Vec::with_capacity(surface_count);
        for _ in 0..surface_count {
            let mut u = [0f32; 4];
            let mut v = [0f32; 4];
            for val in &mut u { *val = r.read_f32::<LE>()?; }
            for val in &mut v { *val = r.read_f32::<LE>()?; }
            let texture_id = r.read_i16::<LE>()?;
            let lightmap_id = r.read_i16::<LE>()?;
            let mut color_bgra = [0u8; 4];
            r.read_exact(&mut color_bgra)?;
            surfaces.push(GndSurface { u, v, texture_id, lightmap_id, color_bgra });
        }

        // Cells
        let cell_count = (width as usize) * (height as usize);
        let mut cells = Vec::with_capacity(cell_count);
        for _ in 0..cell_count {
            let h = [
                r.read_f32::<LE>()?,
                r.read_f32::<LE>()?,
                r.read_f32::<LE>()?,
                r.read_f32::<LE>()?,
            ];

            let (top, front, right) = if version_at_least(version, 1, 7) {
                (r.read_i32::<LE>()?, r.read_i32::<LE>()?, r.read_i32::<LE>()?)
            } else {
                (r.read_i16::<LE>()? as i32, r.read_i16::<LE>()? as i32, r.read_i16::<LE>()? as i32)
            };

            cells.push(GndCell {
                height: h,
                top_surface: top,
                front_surface: front,
                right_surface: right,
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
        data.extend_from_slice(&10.0f32.to_le_bytes()); // zoom
        // 0 textures
        data.extend_from_slice(&0i32.to_le_bytes()); // texture_count
        data.extend_from_slice(&40i32.to_le_bytes()); // texture_name_len
        // 0 lightmaps
        data.extend_from_slice(&0i32.to_le_bytes()); // lightmap_count
        data.extend_from_slice(&8i32.to_le_bytes()); // lmap_width
        data.extend_from_slice(&8i32.to_le_bytes()); // lmap_height
        data.extend_from_slice(&1i32.to_le_bytes()); // cells_per_grid
        // 0 surfaces
        data.extend_from_slice(&0i32.to_le_bytes());
        data
    }

    #[test]
    fn parse_v1_6_uses_i16_surface_indices() {
        let mut data = build_gnd_header((1, 6), 1, 1);
        // 1 cell with i16 surface indices
        for _ in 0..4 { data.extend_from_slice(&1.0f32.to_le_bytes()); }
        data.extend_from_slice(&0i16.to_le_bytes()); // top
        data.extend_from_slice(&(-1i16).to_le_bytes()); // front
        data.extend_from_slice(&(-1i16).to_le_bytes()); // right

        let gnd = GndFile::parse(&data).unwrap();
        assert_eq!(gnd.cells[0].top_surface, 0);
        assert_eq!(gnd.cells[0].front_surface, -1);
    }

    #[test]
    fn parse_v1_7_uses_i32_surface_indices() {
        let mut data = build_gnd_header((1, 7), 1, 1);
        for _ in 0..4 { data.extend_from_slice(&2.0f32.to_le_bytes()); }
        data.extend_from_slice(&5i32.to_le_bytes()); // top
        data.extend_from_slice(&(-1i32).to_le_bytes()); // front
        data.extend_from_slice(&3i32.to_le_bytes()); // right

        let gnd = GndFile::parse(&data).unwrap();
        assert_eq!(gnd.cells[0].top_surface, 5);
        assert_eq!(gnd.cells[0].right_surface, 3);
        assert_eq!(gnd.cells[0].height[0], 2.0);
    }
}
