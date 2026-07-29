use ragnarok_formats::gat::GatFile;
use ragnarok_formats::gnd::GndFile;

/// Texel step between neighbouring cells. A cell's 8x8 lightmap spans it
/// edge-to-edge, so its last row and column are the next cell's first.
const CELL_STRIDE: usize = 7;

/// A GAT cell is treated as standing on the terrain unless it rises this far
/// above the ground cell's highest corner.
const ON_GROUND_TOLERANCE: f32 = 2.5;

/// Ground-lightmap tint for a sprite, read from the same texels the terrain
/// samples so an actor crossing a pool brightens continuously.
pub struct ActorLightmap {
    gat_width: i32,
    gat_height: i32,
    /// Whether each GAT cell sits on the terrain; raised cells take no tint.
    on_ground: Vec<bool>,
    tex_width: usize,
    tex_height: usize,
    shadow: Vec<u8>,
    color: Vec<[u8; 3]>,
}

impl ActorLightmap {
    pub fn build(gnd: &GndFile, gat: &GatFile) -> Option<Self> {
        if !gnd.has_lightmap_data() {
            return None;
        }

        let tex_width = gnd.width.max(1) as usize * CELL_STRIDE + 1;
        let tex_height = gnd.height.max(1) as usize * CELL_STRIDE + 1;
        let mut shadow = vec![255u8; tex_width * tex_height];
        let mut color = vec![[0u8; 3]; tex_width * tex_height];

        for y in 0..gnd.height {
            for x in 0..gnd.width {
                let cell = &gnd.cells[(y * gnd.width + x) as usize];
                if cell.surface_up < 0 {
                    continue;
                }
                let Some(surface) = gnd.surfaces.get(cell.surface_up as usize) else {
                    continue;
                };
                if surface.lightmap_id < 0 {
                    continue;
                }
                let Some(lm) = gnd.lightmaps.get(surface.lightmap_id as usize) else {
                    continue;
                };
                let (bx, by) = (x as usize * CELL_STRIDE, y as usize * CELL_STRIDE);
                for ty in 0..8 {
                    for tx in 0..8 {
                        let src = ty * 8 + tx;
                        let dst = (by + ty) * tex_width + bx + tx;
                        shadow[dst] = lm.shadow[src];
                        color[dst] = [
                            lm.color[src * 3],
                            lm.color[src * 3 + 1],
                            lm.color[src * 3 + 2],
                        ];
                    }
                }
            }
        }

        let mut on_ground = Vec::with_capacity((gat.width * gat.height) as usize);
        for cy in 0..gat.height {
            for cx in 0..gat.width {
                on_ground.push(gat_cell_on_ground(gnd, gat, cx, cy));
            }
        }

        Some(Self {
            gat_width: gat.width,
            gat_height: gat.height,
            on_ground,
            tex_width,
            tex_height,
            shadow,
            color,
        })
    }

    /// Tint at a fractional GAT position. Sampling follows world position, not
    /// the cell index, so the value changes smoothly as an actor walks.
    pub fn intensity_at_pos(&self, cx: f32, cy: f32) -> [f32; 3] {
        let (ix, iy) = (cx.floor() as i32, cy.floor() as i32);
        if ix < 0 || iy < 0 || ix >= self.gat_width || iy >= self.gat_height {
            return [1.0; 3];
        }
        if !self.on_ground[(iy * self.gat_width + ix) as usize] {
            return [1.0; 3];
        }

        // Two GAT cells span one ground cell, which is CELL_STRIDE texels.
        let tx = cx / 2.0 * CELL_STRIDE as f32;
        let ty = cy / 2.0 * CELL_STRIDE as f32;
        let (shadow, color) = self.sample_bilinear(tx, ty);

        let mut out = [0.0f32; 3];
        for c in 0..3 {
            out[c] = color[c] / 128.0 + shadow / 255.0;
        }
        let max = out[0].max(out[1]).max(out[2]);
        if max > 1.0 {
            for c in &mut out {
                *c /= max;
            }
        }
        out
    }

    pub fn intensity_at(&self, cx: i32, cy: i32) -> [f32; 3] {
        self.intensity_at_pos(cx as f32 + 0.5, cy as f32 + 0.5)
    }

    fn sample_bilinear(&self, tx: f32, ty: f32) -> (f32, [f32; 3]) {
        let max_x = self.tex_width - 1;
        let max_y = self.tex_height - 1;
        let x0 = (tx.floor().max(0.0) as usize).min(max_x);
        let y0 = (ty.floor().max(0.0) as usize).min(max_y);
        let x1 = (x0 + 1).min(max_x);
        let y1 = (y0 + 1).min(max_y);
        let fx = (tx - x0 as f32).clamp(0.0, 1.0);
        let fy = (ty - y0 as f32).clamp(0.0, 1.0);

        let w = [
            (1.0 - fx) * (1.0 - fy),
            fx * (1.0 - fy),
            (1.0 - fx) * fy,
            fx * fy,
        ];
        let idx = [
            y0 * self.tex_width + x0,
            y0 * self.tex_width + x1,
            y1 * self.tex_width + x0,
            y1 * self.tex_width + x1,
        ];

        let mut shadow = 0.0;
        let mut color = [0.0f32; 3];
        for (weight, i) in w.iter().zip(idx) {
            shadow += self.shadow[i] as f32 * weight;
            for c in 0..3 {
                color[c] += self.color[i][c] as f32 * weight;
            }
        }
        (shadow, color)
    }
}

/// Ragnarok's Y axis points down, so a cell's highest point is its minimum.
fn gat_cell_on_ground(gnd: &GndFile, gat: &GatFile, cx: i32, cy: i32) -> bool {
    let cell = &gat.cells[(cy * gat.width + cx) as usize];
    let gat_height = (cell.height_sw + cell.height_se + cell.height_nw + cell.height_ne) / 4.0;
    match ground_height_min_max(gnd, cx / 2, cy / 2) {
        Some((top, _)) => gat_height >= top - ON_GROUND_TOLERANCE,
        None => false,
    }
}

/// Lowest and highest corner of a ground cell's top surface. `None` when the
/// cell has no top surface.
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

    /// Two ground cells whose shadow ramps linearly across x, continuing across
    /// the shared edge: texel i of the pair holds 16 * i.
    fn gradient_map() -> (GndFile, GatFile) {
        let ramp = |base: u8| Lightmap {
            shadow: std::array::from_fn(|i| base + 16 * (i % 8) as u8),
            color: [0; 192],
        };
        let gnd = GndFile {
            version: (1, 7),
            width: 2,
            height: 1,
            zoom: 10.0,
            textures: vec!["t.bmp".into()],
            lightmaps: vec![ramp(0), ramp(112)],
            surfaces: (0..2)
                .map(|i| GndSurface {
                    tex_u: [0.0; 4],
                    tex_v: [0.0; 4],
                    texture_id: 0,
                    lightmap_id: i,
                    color_bgra: [255; 4],
                })
                .collect(),
            cells: (0..2)
                .map(|i| GndCell {
                    height_sw: 0.0,
                    height_se: 0.0,
                    height_nw: 0.0,
                    height_ne: 0.0,
                    surface_up: i,
                    surface_south: -1,
                    surface_east: -1,
                })
                .collect(),
        };
        let gat = GatFile {
            version: (1, 2),
            width: 4,
            height: 2,
            cells: (0..8)
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
    fn walking_through_a_gradient_brightens_smoothly_not_in_cell_steps() {
        let (gnd, gat) = gradient_map();
        let lm = ActorLightmap::build(&gnd, &gat).unwrap();

        let walk: Vec<f32> = (1..=39)
            .map(|i| lm.intensity_at_pos(i as f32 / 10.0, 0.5)[0])
            .collect();

        assert!(
            walk.windows(2).all(|w| w[1] > w[0]),
            "not monotonic: {walk:?}"
        );
        let total = walk[walk.len() - 1] - walk[0];
        let biggest = walk
            .windows(2)
            .map(|w| w[1] - w[0])
            .fold(0.0f32, f32::max);
        assert!(
            biggest < total / 8.0,
            "step {biggest} too abrupt over {total}: {walk:?}"
        );

        // The old per-cell sampling returned one value for a whole GAT cell.
        assert_ne!(lm.intensity_at_pos(1.1, 0.5), lm.intensity_at_pos(1.9, 0.5));
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
