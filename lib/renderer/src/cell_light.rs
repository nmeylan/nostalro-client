use ragnarok_formats::gnd::GndFile;

/// Index of the centre texel of an 8x8 lightmap.
const CENTRE_TEXEL: usize = 4 * 8 + 4;

/// One texel per ground cell holding the coloured light baked into that cell,
/// which map models add to their own lighting.
pub struct CellLightMap {
    pub width: u32,
    pub height: u32,
    /// RGBA8, row-major from cell (0, 0).
    pub pixels: Vec<u8>,
}

impl CellLightMap {
    pub fn from_gnd(gnd: &GndFile) -> Option<Self> {
        if !gnd.has_lightmap_data() {
            return None;
        }

        let mut pixels = Vec::with_capacity((gnd.width * gnd.height * 4) as usize);
        for y in 0..gnd.height {
            for x in 0..gnd.width {
                let rgb = cell_light(gnd, x, y);
                pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }

        Some(Self {
            width: gnd.width as u32,
            height: gnd.height as u32,
            pixels,
        })
    }

    pub fn at(&self, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2]]
    }
}

/// Cells with no top surface contribute nothing, so a model standing off the
/// ground grid keeps its own lighting.
fn cell_light(gnd: &GndFile, x: i32, y: i32) -> [u8; 3] {
    let cell = &gnd.cells[(y * gnd.width + x) as usize];
    if cell.surface_up < 0 {
        return [0; 3];
    }
    let Some(surface) = gnd.surfaces.get(cell.surface_up as usize) else {
        return [0; 3];
    };
    if surface.lightmap_id < 0 {
        return [0; 3];
    }
    let Some(lightmap) = gnd.lightmaps.get(surface.lightmap_id as usize) else {
        return [0; 3];
    };

    let mut rgb = [0u8; 3];
    for (c, out) in rgb.iter_mut().enumerate() {
        *out = lightmap.color[CENTRE_TEXEL * 3 + c].saturating_mul(2);
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::gnd::{GndCell, GndSurface, Lightmap};

    fn gnd_with_centre_colours(colours: &[[u8; 3]]) -> GndFile {
        let lightmaps = colours
            .iter()
            .map(|rgb| {
                let mut color = [0u8; 192];
                for (c, v) in rgb.iter().enumerate() {
                    color[CENTRE_TEXEL * 3 + c] = *v;
                }
                Lightmap {
                    shadow: [255; 64],
                    color,
                }
            })
            .collect();
        let surfaces = (0..colours.len())
            .map(|i| GndSurface {
                tex_u: [0.0; 4],
                tex_v: [0.0; 4],
                texture_id: 0,
                lightmap_id: i as i16,
                color_bgra: [255; 4],
            })
            .collect();
        let cells = (0..colours.len())
            .map(|i| GndCell {
                height_sw: 0.0,
                height_se: 0.0,
                height_nw: 0.0,
                height_ne: 0.0,
                surface_up: i as i32,
                surface_south: -1,
                surface_east: -1,
            })
            .collect();
        GndFile {
            version: (1, 7),
            width: colours.len() as i32,
            height: 1,
            zoom: 10.0,
            textures: vec!["t.bmp".into()],
            lightmaps,
            surfaces,
            cells,
        }
    }

    #[test]
    fn cell_colour_is_doubled_and_clamped() {
        let gnd = gnd_with_centre_colours(&[[40, 10, 0], [200, 0, 0], [0, 0, 0]]);
        let map = CellLightMap::from_gnd(&gnd).unwrap();

        assert_eq!(map.at(0, 0), [80, 20, 0]);
        assert_eq!(map.at(1, 0), [255, 0, 0]);
        assert_eq!(map.at(2, 0), [0, 0, 0]);
    }

    #[test]
    fn a_cell_without_a_top_surface_adds_nothing() {
        let mut gnd = gnd_with_centre_colours(&[[40, 10, 0]]);
        gnd.cells[0].surface_up = -1;
        let map = CellLightMap::from_gnd(&gnd).unwrap();

        assert_eq!(map.at(0, 0), [0, 0, 0]);
    }

    #[test]
    fn an_old_gnd_has_no_cell_light() {
        let mut gnd = gnd_with_centre_colours(&[[40, 10, 0]]);
        gnd.version = (1, 6);
        assert!(CellLightMap::from_gnd(&gnd).is_none());
    }
}
