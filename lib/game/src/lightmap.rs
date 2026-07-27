use ragnarok_formats::gat::GatFile;
use ragnarok_formats::gnd::GndFile;

/// How many lightmap texels feed one GAT cell's tint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sampling {
    /// The cell's own quadrant texel.
    SingleTap,
    /// The cell's quadrant texel averaged with its four GAT neighbours.
    FiveTap,
}

const TAPS: [(i32, i32); 5] = [(0, 0), (0, -1), (0, 1), (1, 0), (-1, 0)];

/// The four quadrant texels of an 8x8 lightmap, one per GAT sub-cell, as (y, x).
const QUADRANT_TEXELS: [(usize, usize); 4] = [(1, 1), (1, 5), (5, 1), (5, 5)];

/// A GAT cell is treated as standing on the terrain unless it rises this far
/// above the ground cell's highest corner.
const ON_GROUND_TOLERANCE: f32 = 2.5;

/// Per-GAT-cell multiplier applied to a sprite standing on that cell, baked from
/// the ground lightmap once per map.
pub struct ActorLightmap {
    width: i32,
    height: i32,
    intensity: Vec<[f32; 3]>,
}

impl ActorLightmap {
    pub fn build(gnd: &GndFile, gat: &GatFile) -> Option<Self> {
        Self::build_with(gnd, gat, Sampling::FiveTap)
    }

    pub fn build_with(gnd: &GndFile, gat: &GatFile, sampling: Sampling) -> Option<Self> {
        if !gnd.has_lightmap_data() {
            return None;
        }

        let taps: &[(i32, i32)] = match sampling {
            Sampling::SingleTap => &TAPS[..1],
            Sampling::FiveTap => &TAPS,
        };
        let divisor = taps.len() as f32;

        let mut intensity = Vec::with_capacity((gat.width * gat.height) as usize);
        for cy in 0..gat.height {
            for cx in 0..gat.width {
                intensity.push(cell_intensity(gnd, gat, cx, cy, taps, divisor));
            }
        }

        Some(Self {
            width: gat.width,
            height: gat.height,
            intensity,
        })
    }

    pub fn intensity_at(&self, cx: i32, cy: i32) -> [f32; 3] {
        if cx < 0 || cy < 0 || cx >= self.width || cy >= self.height {
            return [1.0; 3];
        }
        self.intensity[(cy * self.width + cx) as usize]
    }
}

fn cell_intensity(
    gnd: &GndFile,
    gat: &GatFile,
    cx: i32,
    cy: i32,
    taps: &[(i32, i32)],
    divisor: f32,
) -> [f32; 3] {
    let cell = &gat.cells[(cy * gat.width + cx) as usize];
    let gat_height = (cell.height_sw + cell.height_se + cell.height_nw + cell.height_ne) / 4.0;

    let Some((top, _bottom)) = ground_height_min_max(gnd, cx / 2, cy / 2) else {
        return [1.0; 3];
    };
    if gat_height < top - ON_GROUND_TOLERANCE {
        return [1.0; 3];
    }

    let mut shadow_total = 0u32;
    let mut color_total = [0u32; 3];
    for (dx, dy) in taps {
        let (shadow, color) = texel(gnd, cx + dx, cy + dy);
        shadow_total += shadow as u32;
        for c in 0..3 {
            color_total[c] += color[c] as u32;
        }
    }

    let shadow = shadow_total as f32 / divisor / 255.0;
    let mut out = [0.0f32; 3];
    for c in 0..3 {
        out[c] = color_total[c] as f32 / divisor / 128.0 + shadow;
    }

    let max = out[0].max(out[1]).max(out[2]);
    if max > 1.0 {
        for c in &mut out {
            *c /= max;
        }
    }
    out
}

/// Lowest and highest corner of a ground cell's top surface. `None` when the
/// cell has no top surface. Ragnarok's Y axis points down, so the minimum is the
/// highest point of the terrain.
fn ground_height_min_max(gnd: &GndFile, x: i32, y: i32) -> Option<(f32, f32)> {
    if x < 0 || y < 0 || x >= gnd.width || y >= gnd.height {
        return None;
    }
    let cell = &gnd.cells[(y * gnd.width + x) as usize];
    if cell.surface_up < 0 {
        return None;
    }
    let h = [
        cell.height_sw,
        cell.height_se,
        cell.height_nw,
        cell.height_ne,
    ];
    let min = h.iter().copied().fold(f32::INFINITY, f32::min);
    let max = h.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    Some((min, max))
}

/// Shadow intensity and colour of the lightmap texel covering GAT cell
/// (`cx`, `cy`). White where there is nothing to sample.
fn texel(gnd: &GndFile, cx: i32, cy: i32) -> (u8, [u8; 3]) {
    const WHITE: (u8, [u8; 3]) = (255, [255; 3]);

    if cx < 0 || cy < 0 || cx / 2 >= gnd.width || cy / 2 >= gnd.height {
        return WHITE;
    }
    let cell = &gnd.cells[((cy / 2) * gnd.width + cx / 2) as usize];
    if cell.surface_up < 0 {
        return WHITE;
    }
    let Some(surface) = gnd.surfaces.get(cell.surface_up as usize) else {
        return WHITE;
    };
    if surface.lightmap_id < 0 {
        return WHITE;
    }
    let Some(lightmap) = gnd.lightmaps.get(surface.lightmap_id as usize) else {
        return WHITE;
    };

    let (ty, tx) = QUADRANT_TEXELS[((cy % 2) * 2 + (cx % 2)) as usize];
    let offset = ty * 8 + tx;
    (
        lightmap.shadow[offset],
        [
            lightmap.color[offset * 3],
            lightmap.color[offset * 3 + 1],
            lightmap.color[offset * 3 + 2],
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::gat::GatCell;
    use ragnarok_formats::gnd::{GndCell, GndSurface, Lightmap};

    const DARK_SHADOW: u8 = 60;

    /// A 4x4 ground split down the middle: the west half in deep shade, the east
    /// half in full light with a red tinge. GAT is the usual 2x resolution, every
    /// cell at ground level.
    fn split_map() -> (GndFile, GatFile) {
        let lightmaps = vec![
            Lightmap {
                shadow: [DARK_SHADOW; 64],
                color: [0; 192],
            },
            Lightmap {
                shadow: [255; 64],
                color: {
                    let mut c = [0u8; 192];
                    for texel in c.chunks_mut(3) {
                        texel[0] = 64;
                    }
                    c
                },
            },
        ];
        let cells: Vec<GndCell> = (0..16)
            .map(|i| GndCell {
                height_sw: 0.0,
                height_se: 0.0,
                height_nw: 0.0,
                height_ne: 0.0,
                surface_up: i,
                surface_south: -1,
                surface_east: -1,
            })
            .collect();
        let surfaces = (0..16)
            .map(|i| GndSurface {
                tex_u: [0.0; 4],
                tex_v: [0.0; 4],
                texture_id: 0,
                lightmap_id: if i % 4 < 2 { 0 } else { 1 },
                color_bgra: [255; 4],
            })
            .collect();
        let gnd = GndFile {
            version: (1, 7),
            width: 4,
            height: 4,
            zoom: 10.0,
            textures: vec!["t.bmp".into()],
            lightmaps,
            surfaces,
            cells,
        };

        let gat = GatFile {
            version: (1, 2),
            width: 8,
            height: 8,
            cells: (0..64)
                .map(|_| GatCell {
                    height_sw: 0.0,
                    height_se: 0.0,
                    height_nw: 0.0,
                    height_ne: 0.0,
                    cell_flags: 0,
                })
                .collect(),
        };
        (gnd, gat)
    }

    #[test]
    fn shaded_cell_is_darker_than_lit_cell_and_keeps_its_hue() {
        let (gnd, gat) = split_map();
        let lm = ActorLightmap::build(&gnd, &gat).unwrap();

        let shaded = lm.intensity_at(2, 2);
        let lit = lm.intensity_at(5, 5);

        assert!(shaded[0] < lit[0], "{shaded:?} vs {lit:?}");
        assert!(shaded.iter().all(|c| *c > 0.0 && *c < 1.0), "{shaded:?}");
        assert!(lit[0] > lit[1], "red tinge lost: {lit:?}");

        assert_eq!(lm.intensity_at(-1, 0), [1.0; 3]);
        assert_eq!(lm.intensity_at(8, 0), [1.0; 3]);
    }

    #[test]
    fn single_tap_reads_only_the_cells_own_quadrant() {
        let (gnd, gat) = split_map();
        let lm = ActorLightmap::build_with(&gnd, &gat, Sampling::SingleTap).unwrap();

        assert_eq!(lm.intensity_at(2, 2), [DARK_SHADOW as f32 / 255.0; 3]);
    }

    #[test]
    fn a_cell_raised_above_the_terrain_is_not_tinted() {
        let (gnd, mut gat) = split_map();
        for cell in &mut gat.cells {
            cell.height_sw = -20.0;
            cell.height_se = -20.0;
            cell.height_nw = -20.0;
            cell.height_ne = -20.0;
        }
        let lm = ActorLightmap::build(&gnd, &gat).unwrap();

        assert_eq!(lm.intensity_at(2, 2), [1.0; 3]);
    }
}
