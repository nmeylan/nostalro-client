use ragnarok_formats::gnd::{GndCell, GndFile, GndSurface, Lightmap};
use ragnarok_formats::grf::GrfArchive;

use crate::device::DEPTH_FORMAT;
use crate::global_uniforms::GlobalUniforms;
use crate::texture::{self, TextureCache};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GroundVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub lightmap_coord: [f32; 2],
    pub color: [f32; 4],
}

impl GroundVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<GroundVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
            3 => Float32x2,
            4 => Float32x4,
        ],
    };
}

struct DrawBatch {
    texture_name: String,
    start_index: u32,
    index_count: u32,
    /// Top surfaces read the map-wide cell texture; walls stay on the slice
    /// atlas, which a world-XZ lookup cannot address.
    top: bool,
}

pub struct GroundRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    batches: Vec<DrawBatch>,
    lightmap_bind_group: wgpu::BindGroup,
    cell_lightmap_bind_group: wgpu::BindGroup,
    lightmap_off_bind_group: wgpu::BindGroup,
    lightmap_enabled: bool,
}

impl GroundRenderer {
    pub fn from_gnd(
        gnd: &GndFile,
        grf: &GrfArchive,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        global_uniforms: &GlobalUniforms,
        texture_cache: &mut TextureCache,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        for tex_name in &gnd.textures {
            let path = ragnarok_resources::texture::named(tex_name);
            texture_cache.get_or_load_ground(&path, grf, device, queue);
        }

        let lightmap_bind_group = build_lightmap_atlas(
            &gnd.lightmaps,
            device,
            queue,
            &texture_cache.bind_group_layout,
        );
        let cell_lightmap_bind_group =
            build_cell_lightmap(gnd, device, queue, &texture_cache.bind_group_layout);

        // Disabled lightmap: black color map (adds nothing) with full alpha (no shadow).
        let off_img = image::RgbaImage::from_raw(1, 1, vec![0, 0, 0, 255]).unwrap();
        let lightmap_off_bind_group = texture::create_texture_bind_group(
            device,
            queue,
            &off_img,
            &texture_cache.bind_group_layout,
            "lightmap_off",
        );

        let atlas_dim = lightmap_atlas_dim(gnd.lightmaps.len());
        let (vertices, indices, batches) = build_mesh(gnd, atlas_dim);

        let vertex_buffer = create_buffer(
            device,
            "ground_vertices",
            &vertices,
            wgpu::BufferUsages::VERTEX,
        );
        let index_buffer = create_buffer(
            device,
            "ground_indices",
            &indices,
            wgpu::BufferUsages::INDEX,
        );

        let pipeline = create_pipeline(
            device,
            surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
        );

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            batches,
            lightmap_bind_group,
            cell_lightmap_bind_group,
            lightmap_off_bind_group,
            lightmap_enabled: true,
        }
    }

    pub fn set_lightmap_enabled(&mut self, enabled: bool) {
        self.lightmap_enabled = enabled;
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        global_uniforms: &'a GlobalUniforms,
        texture_cache: &'a TextureCache,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &global_uniforms.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        for batch in &self.batches {
            if let Some(tex_bg) = texture_cache.get_ground(&batch.texture_name) {
                let lightmap = match (self.lightmap_enabled, batch.top) {
                    (false, _) => &self.lightmap_off_bind_group,
                    (true, true) => &self.cell_lightmap_bind_group,
                    (true, false) => &self.lightmap_bind_group,
                };
                pass.set_bind_group(1, tex_bg, &[]);
                pass.set_bind_group(2, lightmap, &[]);
                pass.draw_indexed(
                    batch.start_index..batch.start_index + batch.index_count,
                    0,
                    0..1,
                );
            }
        }
    }
}

fn build_mesh(gnd: &GndFile, atlas_dim: u32) -> (Vec<GroundVertex>, Vec<u32>, Vec<DrawBatch>) {
    let mut texture_quads: std::collections::HashMap<
        (String, bool),
        (Vec<GroundVertex>, Vec<u32>),
    > = std::collections::HashMap::new();

    let face_normals = build_face_normals(gnd);

    for y in 0..gnd.height {
        for x in 0..gnd.width {
            let cell_idx = (y * gnd.width + x) as usize;
            let cell = &gnd.cells[cell_idx];

            if cell.surface_up >= 0 {
                let surface = &gnd.surfaces[cell.surface_up as usize];
                let tex_name = texture_name_for_surface(gnd, surface);
                let colors = [
                    corner_color(gnd, x, y),
                    corner_color(gnd, x + 1, y),
                    corner_color(gnd, x, y + 1),
                    corner_color(gnd, x + 1, y + 1),
                ];

                let wx = x as f32 * gnd.zoom;
                let wz = y as f32 * gnd.zoom;

                let positions = [
                    [wx, cell.height_sw, wz],
                    [wx + gnd.zoom, cell.height_se, wz],
                    [wx, cell.height_nw, wz + gnd.zoom],
                    [wx + gnd.zoom, cell.height_ne, wz + gnd.zoom],
                ];

                let normals = smooth_vertex_normals(gnd, &face_normals, x, y);

                let lm_uvs = cell_lightmap_uvs(x, y, gnd.width, gnd.height);

                let verts = [
                    GroundVertex {
                        position: positions[0],
                        normal: normals[0],
                        tex_coord: [surface.tex_u[0], surface.tex_v[0]],
                        lightmap_coord: lm_uvs[0],
                        color: colors[0],
                    },
                    GroundVertex {
                        position: positions[1],
                        normal: normals[1],
                        tex_coord: [surface.tex_u[1], surface.tex_v[1]],
                        lightmap_coord: lm_uvs[1],
                        color: colors[1],
                    },
                    GroundVertex {
                        position: positions[2],
                        normal: normals[2],
                        tex_coord: [surface.tex_u[2], surface.tex_v[2]],
                        lightmap_coord: lm_uvs[2],
                        color: colors[2],
                    },
                    GroundVertex {
                        position: positions[3],
                        normal: normals[3],
                        tex_coord: [surface.tex_u[3], surface.tex_v[3]],
                        lightmap_coord: lm_uvs[3],
                        color: colors[3],
                    },
                ];

                let entry = texture_quads
                    .entry((tex_name, true))
                    .or_insert_with(|| (Vec::new(), Vec::new()));
                let base = entry.0.len() as u32;
                entry.0.extend_from_slice(&verts);
                entry.1.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base + 2,
                    base + 1,
                    base + 3,
                ]);
            }

            if cell.surface_south >= 0 && y + 1 < gnd.height {
                let next_cell = &gnd.cells[((y + 1) * gnd.width + x) as usize];
                let surface = &gnd.surfaces[cell.surface_south as usize];
                let tex_name = texture_name_for_surface(gnd, surface);
                let color_west = corner_color(gnd, x, y + 1);
                let color_east = corner_color(gnd, x + 1, y + 1);

                let wx = x as f32 * gnd.zoom;
                let wz = (y + 1) as f32 * gnd.zoom;

                let positions = [
                    [wx, cell.height_nw, wz],
                    [wx + gnd.zoom, cell.height_ne, wz],
                    [wx, next_cell.height_sw, wz],
                    [wx + gnd.zoom, next_cell.height_se, wz],
                ];

                let normal = compute_quad_normal(&positions);
                let lm_uvs = lightmap_uvs(surface.lightmap_id, atlas_dim);

                let verts = [
                    GroundVertex {
                        position: positions[0],
                        normal,
                        tex_coord: [surface.tex_u[0], surface.tex_v[0]],
                        lightmap_coord: lm_uvs[0],
                        color: color_west,
                    },
                    GroundVertex {
                        position: positions[1],
                        normal,
                        tex_coord: [surface.tex_u[1], surface.tex_v[1]],
                        lightmap_coord: lm_uvs[1],
                        color: color_east,
                    },
                    GroundVertex {
                        position: positions[2],
                        normal,
                        tex_coord: [surface.tex_u[2], surface.tex_v[2]],
                        lightmap_coord: lm_uvs[2],
                        color: color_west,
                    },
                    GroundVertex {
                        position: positions[3],
                        normal,
                        tex_coord: [surface.tex_u[3], surface.tex_v[3]],
                        lightmap_coord: lm_uvs[3],
                        color: color_east,
                    },
                ];

                let entry = texture_quads
                    .entry((tex_name, false))
                    .or_insert_with(|| (Vec::new(), Vec::new()));
                let base = entry.0.len() as u32;
                entry.0.extend_from_slice(&verts);
                entry.1.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base + 2,
                    base + 1,
                    base + 3,
                ]);
            }

            if cell.surface_east >= 0 && x + 1 < gnd.width {
                let next_cell = &gnd.cells[(y * gnd.width + x + 1) as usize];
                let surface = &gnd.surfaces[cell.surface_east as usize];
                let tex_name = texture_name_for_surface(gnd, surface);
                let color_north = corner_color(gnd, x + 1, y);
                let color_south = corner_color(gnd, x + 1, y + 1);

                let wx = (x + 1) as f32 * gnd.zoom;
                let wz = y as f32 * gnd.zoom;

                let positions = [
                    [wx, cell.height_se, wz],
                    [wx, next_cell.height_sw, wz],
                    [wx, cell.height_ne, wz + gnd.zoom],
                    [wx, next_cell.height_nw, wz + gnd.zoom],
                ];

                let normal = compute_quad_normal(&positions);
                let lm_uvs = lightmap_uvs(surface.lightmap_id, atlas_dim);

                let verts = [
                    GroundVertex {
                        position: positions[0],
                        normal,
                        tex_coord: [surface.tex_u[0], surface.tex_v[0]],
                        lightmap_coord: lm_uvs[0],
                        color: color_north,
                    },
                    GroundVertex {
                        position: positions[1],
                        normal,
                        tex_coord: [surface.tex_u[1], surface.tex_v[1]],
                        lightmap_coord: lm_uvs[1],
                        color: color_north,
                    },
                    GroundVertex {
                        position: positions[2],
                        normal,
                        tex_coord: [surface.tex_u[2], surface.tex_v[2]],
                        lightmap_coord: lm_uvs[2],
                        color: color_south,
                    },
                    GroundVertex {
                        position: positions[3],
                        normal,
                        tex_coord: [surface.tex_u[3], surface.tex_v[3]],
                        lightmap_coord: lm_uvs[3],
                        color: color_south,
                    },
                ];

                let entry = texture_quads
                    .entry((tex_name, false))
                    .or_insert_with(|| (Vec::new(), Vec::new()));
                let base = entry.0.len() as u32;
                entry.0.extend_from_slice(&verts);
                entry.1.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base + 2,
                    base + 1,
                    base + 3,
                ]);
            }
        }
    }

    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    let mut batches = Vec::new();

    for ((tex_name, top), (verts, idxs)) in texture_quads {
        let vertex_offset = all_vertices.len() as u32;
        let start_index = all_indices.len() as u32;
        all_vertices.extend_from_slice(&verts);
        all_indices.extend(idxs.iter().map(|i| i + vertex_offset));
        batches.push(DrawBatch {
            texture_name: tex_name,
            start_index,
            index_count: idxs.len() as u32,
            top,
        });
    }

    (all_vertices, all_indices, batches)
}

fn texture_name_for_surface(gnd: &GndFile, surface: &GndSurface) -> String {
    if surface.texture_id >= 0 && (surface.texture_id as usize) < gnd.textures.len() {
        ragnarok_resources::texture::named(&gnd.textures[surface.texture_id as usize])
    } else {
        String::new()
    }
}

/// A ground vertex reads the tint of the cell whose north-west corner it sits
/// on, so the cells meeting at a corner all agree there and the tint field stays
/// continuous. Cells off the map or without a top surface contribute black.
/// I hope this is now correct
fn corner_color(gnd: &GndFile, x: i32, y: i32) -> [f32; 4] {
    if x < 0 || y < 0 || x >= gnd.width || y >= gnd.height {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let cell = &gnd.cells[(y * gnd.width + x) as usize];
    if cell.surface_up < 0 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let [r, g, b, _] = bgra_to_rgba_f32(gnd.surfaces[cell.surface_up as usize].color_bgra);
    [r, g, b, 1.0]
}

fn bgra_to_rgba_f32(bgra: [u8; 4]) -> [f32; 4] {
    [
        bgra[2] as f32 / 255.0, // R from B position
        bgra[1] as f32 / 255.0, // G
        bgra[0] as f32 / 255.0, // B from R position
        bgra[3] as f32 / 255.0, // A
    ]
}

/// Normals of the two triangles making up a cell's top surface. Zero for cells
/// that have no top surface, so they add nothing when a neighbour reads them.
#[derive(Clone, Copy, Default)]
struct FaceNormals {
    face0: glam::Vec3,
    face1: glam::Vec3,
}

fn build_face_normals(gnd: &GndFile) -> Vec<FaceNormals> {
    gnd.cells
        .iter()
        .map(|cell| {
            if cell.surface_up < 0 {
                return FaceNormals::default();
            }
            let v0 = glam::Vec3::new(0.0, cell.height_sw, 0.0);
            let v1 = glam::Vec3::new(gnd.zoom, cell.height_se, 0.0);
            let v2 = glam::Vec3::new(0.0, cell.height_nw, gnd.zoom);
            let v3 = glam::Vec3::new(gnd.zoom, cell.height_ne, gnd.zoom);
            FaceNormals {
                face0: (v2 - v0).cross(v2 - v1).normalize_or_zero(),
                face1: (v3 - v2).cross(v3 - v1).normalize_or_zero(),
            }
        })
        .collect()
}

/// Averages each corner of a cell's top quad with the neighbouring faces that
/// share that corner's exact height, leaving a hard crease where heights differ.
fn smooth_vertex_normals(gnd: &GndFile, faces: &[FaceNormals], x: i32, y: i32) -> [[f32; 3]; 4] {
    let cell = &gnd.cells[(y * gnd.width + x) as usize];
    let own = faces[(y * gnd.width + x) as usize];

    let mut acc = [
        own.face0,
        own.face0 + own.face1,
        own.face0 + own.face1,
        own.face1,
    ];

    let neighbour = |dx: i32, dz: i32| -> Option<(&GndCell, FaceNormals)> {
        let nx = x + dx;
        let nz = y + dz;
        if nx < 0 || nz < 0 || nx >= gnd.width || nz >= gnd.height {
            return None;
        }
        let idx = (nz * gnd.width + nx) as usize;
        Some((&gnd.cells[idx], faces[idx]))
    };

    type Height = fn(&GndCell) -> f32;
    let terms: [(usize, i32, i32, Height, f32, bool, bool); 12] = [
        (0, 0, -1, |c| c.height_nw, cell.height_sw, true, true),
        (0, -1, 0, |c| c.height_se, cell.height_sw, true, true),
        (0, -1, -1, |c| c.height_ne, cell.height_sw, false, true),
        (1, 1, 0, |c| c.height_sw, cell.height_se, true, false),
        (1, 0, -1, |c| c.height_ne, cell.height_se, false, true),
        (1, 1, -1, |c| c.height_nw, cell.height_se, true, true),
        (2, -1, 0, |c| c.height_ne, cell.height_nw, false, true),
        (2, 0, 1, |c| c.height_sw, cell.height_nw, true, false),
        (2, -1, 1, |c| c.height_se, cell.height_nw, true, true),
        (3, 1, 0, |c| c.height_nw, cell.height_ne, true, true),
        (3, 0, 1, |c| c.height_se, cell.height_ne, true, true),
        (3, 1, 1, |c| c.height_sw, cell.height_ne, true, false),
    ];

    for &(corner, dx, dz, height_of, own_height, use_face0, use_face1) in &terms {
        let Some((n_cell, n_faces)) = neighbour(dx, dz) else {
            continue;
        };
        // Exact equality, no epsilon: the exactness is what keeps creases hard.
        if height_of(n_cell) != own_height {
            continue;
        }
        if use_face0 {
            acc[corner] += n_faces.face0;
        }
        if use_face1 {
            acc[corner] += n_faces.face1;
        }
    }

    acc.map(|n| n.normalize_or_zero().to_array())
}

fn compute_quad_normal(positions: &[[f32; 3]; 4]) -> [f32; 3] {
    let v0 = glam::Vec3::from(positions[0]);
    let v1 = glam::Vec3::from(positions[1]);
    let v2 = glam::Vec3::from(positions[2]);
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let n = edge1.cross(edge2).normalize_or_zero();
    [n.x, n.y, n.z]
}

const LIGHTMAP_CELL: u32 = 8;
const LIGHTMAP_BORDER: u32 = 1;
const LIGHTMAP_STRIDE: u32 = LIGHTMAP_CELL + 2 * LIGHTMAP_BORDER;

fn lightmap_atlas_dim(lightmap_count: usize) -> u32 {
    if lightmap_count == 0 {
        return 1;
    }
    let grid = (lightmap_count as f64).sqrt().ceil() as u32;
    grid.max(1)
}

fn lightmap_atlas_pixel_size(grid: u32) -> u32 {
    (grid * LIGHTMAP_STRIDE).max(1)
}

fn lightmap_uvs(lightmap_id: i16, atlas_grid: u32) -> [[f32; 2]; 4] {
    if lightmap_id < 0 || atlas_grid == 0 {
        return [[0.0; 2]; 4];
    }
    let id = lightmap_id as u32;
    let gx = id % atlas_grid;
    let gy = id / atlas_grid;
    let atlas_px = lightmap_atlas_pixel_size(atlas_grid) as f32;

    let ox = (gx * LIGHTMAP_STRIDE + LIGHTMAP_BORDER) as f32;
    let oy = (gy * LIGHTMAP_STRIDE + LIGHTMAP_BORDER) as f32;

    let u0 = (ox + 0.5) / atlas_px;
    let v0 = (oy + 0.5) / atlas_px;
    let u1 = (ox + LIGHTMAP_CELL as f32 - 0.5) / atlas_px;
    let v1 = (oy + LIGHTMAP_CELL as f32 - 0.5) / atlas_px;

    [[u0, v0], [u1, v0], [u0, v1], [u1, v1]]
}

fn pack_lightmap_atlas(lightmaps: &[Lightmap], grid: u32, atlas_size: u32) -> Vec<u8> {
    let mut pixels = vec![255u8; (atlas_size * atlas_size * 4) as usize];

    for (i, lm) in lightmaps.iter().enumerate() {
        let cx = (i as u32 % grid) * LIGHTMAP_STRIDE;
        let cy = (i as u32 / grid) * LIGHTMAP_STRIDE;

        for by in 0..LIGHTMAP_STRIDE {
            for bx in 0..LIGHTMAP_STRIDE {
                let sx = (bx as i32 - LIGHTMAP_BORDER as i32).clamp(0, LIGHTMAP_CELL as i32 - 1)
                    as usize;
                let sy = (by as i32 - LIGHTMAP_BORDER as i32).clamp(0, LIGHTMAP_CELL as i32 - 1)
                    as usize;
                let lm_idx = sy * LIGHTMAP_CELL as usize + sx;

                let px = cx + bx;
                let py = cy + by;
                let offset = ((py * atlas_size + px) * 4) as usize;
                pixels[offset] = lm.color[lm_idx * 3];
                pixels[offset + 1] = lm.color[lm_idx * 3 + 1];
                pixels[offset + 2] = lm.color[lm_idx * 3 + 2];
                pixels[offset + 3] = lm.shadow[lm_idx];
            }
        }
    }

    pixels
}

pub const LIGHTMAP_CELL_STRIDE: u32 = LIGHTMAP_CELL - 1;

pub fn cell_lightmap_size(gnd: &GndFile) -> (u32, u32) {
    (
        gnd.width.max(1) as u32 * LIGHTMAP_CELL_STRIDE + 1,
        gnd.height.max(1) as u32 * LIGHTMAP_CELL_STRIDE + 1,
    )
}

/// One 8x8 lightmap per ground cell, laid out in map order and overlapping its
/// neighbours by the shared edge texel. Cells without a top surface are left
/// neutral (`[0, 0, 0, 255]`): colour adds nothing and shadow is unshadowed, so
/// a neighbouring cell's edge interpolates toward "no light change" rather than
/// toward a dark rim.
pub fn pack_cell_lightmap(gnd: &GndFile) -> Vec<u8> {
    let (tex_w, tex_h) = cell_lightmap_size(gnd);
    let (tex_w, tex_h) = (tex_w as usize, tex_h as usize);
    let mut pixels = vec![0u8; tex_w * tex_h * 4];
    for p in pixels.chunks_exact_mut(4) {
        p[3] = 255;
    }

    for y in 0..gnd.height {
        for x in 0..gnd.width {
            let cell = &gnd.cells[(y * gnd.width + x) as usize];
            if cell.surface_up < 0 {
                continue;
            }
            let lm_id = gnd.surfaces[cell.surface_up as usize].lightmap_id;
            if lm_id < 0 {
                continue;
            }
            let Some(lm) = gnd.lightmaps.get(lm_id as usize) else {
                continue;
            };

            let bx = x as usize * LIGHTMAP_CELL_STRIDE as usize;
            let by = y as usize * LIGHTMAP_CELL_STRIDE as usize;
            for ty in 0..LIGHTMAP_CELL as usize {
                for tx in 0..LIGHTMAP_CELL as usize {
                    let src = ty * LIGHTMAP_CELL as usize + tx;
                    let dst = ((by + ty) * tex_w + bx + tx) * 4;
                    pixels[dst] = lm.color[src * 3];
                    pixels[dst + 1] = lm.color[src * 3 + 1];
                    pixels[dst + 2] = lm.color[src * 3 + 2];
                    pixels[dst + 3] = lm.shadow[src];
                }
            }
        }
    }

    pixels
}

/// Spans texel centre 0 to texel centre 7 of the cell's own 8x8. Because the
/// last texel is shared with the next cell, one cell's trailing edge and its
/// neighbour's leading edge resolve to the same texel centre, so the sample
/// point stays a continuous function of world position.
fn cell_lightmap_uvs(x: i32, y: i32, gnd_w: i32, gnd_h: i32) -> [[f32; 2]; 4] {
    let tex_w = (gnd_w.max(1) * LIGHTMAP_CELL_STRIDE as i32 + 1) as f32;
    let tex_h = (gnd_h.max(1) * LIGHTMAP_CELL_STRIDE as i32 + 1) as f32;
    let ox = (x * LIGHTMAP_CELL_STRIDE as i32) as f32;
    let oy = (y * LIGHTMAP_CELL_STRIDE as i32) as f32;
    let u0 = (ox + 0.5) / tex_w;
    let u1 = (ox + LIGHTMAP_CELL as f32 - 0.5) / tex_w;
    let v0 = (oy + 0.5) / tex_h;
    let v1 = (oy + LIGHTMAP_CELL as f32 - 0.5) / tex_h;
    [[u0, v0], [u1, v0], [u0, v1], [u1, v1]]
}

fn build_cell_lightmap(
    gnd: &GndFile,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    let (tex_w, tex_h) = cell_lightmap_size(gnd);
    let img = image::RgbaImage::from_raw(tex_w, tex_h, pack_cell_lightmap(gnd)).unwrap();
    texture::create_texture_bind_group_clamped(
        device,
        queue,
        &img,
        bind_group_layout,
        "lightmap_cells",
    )
}

fn build_lightmap_atlas(
    lightmaps: &[Lightmap],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    let grid = lightmap_atlas_dim(lightmaps.len());
    let atlas_size = lightmap_atlas_pixel_size(grid);

    let pixels = pack_lightmap_atlas(lightmaps, grid, atlas_size);

    let img = image::RgbaImage::from_raw(atlas_size, atlas_size, pixels).unwrap();
    texture::create_texture_bind_group(device, queue, &img, bind_group_layout, "lightmap_atlas")
}

fn create_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: &str,
    data: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(data),
        usage,
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    global_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("terrain"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/terrain.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("terrain"),
        bind_group_layouts: &[
            global_bind_group_layout,
            texture_bind_group_layout,
            texture_bind_group_layout,
        ],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("terrain"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[GroundVertex::LAYOUT],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgra_to_rgba_converts_correctly() {
        let result = bgra_to_rgba_f32([255, 128, 0, 200]);
        assert!((result[0] - 0.0).abs() < 0.01);
        assert!((result[1] - 128.0 / 255.0).abs() < 0.01);
        assert!((result[2] - 1.0).abs() < 0.01);
        assert!((result[3] - 200.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn lightmap_atlas_dim_computes_correct_grid() {
        assert_eq!(lightmap_atlas_dim(0), 1);
        assert_eq!(lightmap_atlas_dim(1), 1);
        assert_eq!(lightmap_atlas_dim(4), 2);
        assert_eq!(lightmap_atlas_dim(5), 3);
        assert_eq!(lightmap_atlas_dim(9), 3);
        assert_eq!(lightmap_atlas_dim(100), 10);
    }

    #[test]
    fn lightmap_uvs_stay_within_bounds() {
        let uvs = lightmap_uvs(0, 4);
        for uv in &uvs {
            assert!(uv[0] >= 0.0 && uv[0] <= 1.0);
            assert!(uv[1] >= 0.0 && uv[1] <= 1.0);
        }
        let uvs = lightmap_uvs(15, 4);
        assert!(uvs[3][0] > 0.7);
        assert!(uvs[3][1] > 0.7);
    }

    #[test]
    fn pack_lightmap_atlas_splits_channels_and_borders() {
        let mut color = [0u8; 192];
        let mut shadow = [0u8; 64];
        color[0] = 200; // r
        color[1] = 100; // g
        color[2] = 50; // b
        shadow[0] = 255;
        let lightmaps = vec![Lightmap { shadow, color }];

        let atlas_size = lightmap_atlas_pixel_size(1);
        let pixels = pack_lightmap_atlas(&lightmaps, 1, atlas_size);
        let at = |x: u32, y: u32| {
            let o = ((y * atlas_size + x) * 4) as usize;
            [pixels[o], pixels[o + 1], pixels[o + 2], pixels[o + 3]]
        };

        // color map -> RGB, shadow -> alpha
        assert_eq!(at(1, 1), [200, 100, 50, 255]);
        // 1px border replicates the nearest interior texel
        assert_eq!(at(0, 0), [200, 100, 50, 255]);
        assert_eq!(at(0, 1), [200, 100, 50, 255]);
    }

    #[test]
    fn cell_lightmap_blocks_are_map_ordered_and_meet_without_a_seam() {
        let lm = |base: u8| Lightmap {
            shadow: std::array::from_fn(|i| base + i as u8),
            color: [0u8; 192],
        };
        let surface = |lightmap_id: i16| GndSurface {
            tex_u: [0.0, 1.0, 0.0, 1.0],
            tex_v: [0.0, 0.0, 1.0, 1.0],
            texture_id: 0,
            lightmap_id,
            color_bgra: [255, 255, 255, 255],
        };
        let gnd = GndFile {
            version: (1, 7),
            width: 2,
            height: 1,
            zoom: 1.0,
            textures: vec!["ground.bmp".to_string()],
            lightmaps: vec![lm(0), lm(100)],
            surfaces: vec![surface(0), surface(1)],
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

        // Two cells overlap by their shared edge column: 7 + 7 + 1 texels wide.
        let (tex_w, tex_h) = cell_lightmap_size(&gnd);
        assert_eq!((tex_w, tex_h), (15, 8));

        let pixels = pack_cell_lightmap(&gnd);
        let alpha = |x: usize, y: usize| pixels[(y * tex_w as usize + x) * 4 + 3];
        for ty in 0..8 {
            for tx in 0..7 {
                assert_eq!(
                    alpha(tx, ty),
                    (ty * 8 + tx) as u8,
                    "cell 0 texel ({tx},{ty})"
                );
                assert_eq!(
                    alpha(8 + tx, ty),
                    100 + (ty * 8 + tx + 1) as u8,
                    "cell 1 texel ({},{ty})",
                    tx + 1
                );
            }
        }

        let (vertices, _, _) = build_mesh(&gnd, 1);
        let uv = |i: usize| vertices[i].lightmap_coord;
        // Cell 0's trailing edge and cell 1's leading edge land on the same
        // texel centre, so the sampled value matches across the boundary.
        assert_eq!(uv(0)[0], 0.5 / 15.0);
        assert_eq!(uv(1)[0], 7.5 / 15.0);
        assert_eq!(uv(4)[0], 7.5 / 15.0);
        assert_eq!(uv(5)[0], 14.5 / 15.0);
        assert_eq!(uv(0)[1], 0.5 / 8.0);
        assert_eq!(uv(3)[1], 7.5 / 8.0);
    }

    #[test]
    fn cell_tint_reaches_only_the_corner_it_owns() {
        let surface = |color_bgra: [u8; 4]| GndSurface {
            tex_u: [0.0, 1.0, 0.0, 1.0],
            tex_v: [0.0, 0.0, 1.0, 1.0],
            texture_id: 0,
            lightmap_id: -1,
            color_bgra,
        };
        let gnd = GndFile {
            version: (1, 7),
            width: 3,
            height: 3,
            zoom: 1.0,
            textures: vec!["ground.bmp".to_string()],
            lightmaps: Vec::new(),
            surfaces: vec![surface([255, 255, 255, 255]), surface([0, 0, 0, 255])],
            cells: (0..9)
                .map(|i| GndCell {
                    height_sw: 0.0,
                    height_se: 0.0,
                    height_nw: 0.0,
                    height_ne: 0.0,
                    surface_up: if i == 4 { 1 } else { 0 },
                    surface_south: -1,
                    surface_east: -1,
                })
                .collect(),
        };

        let (vertices, _, _) = build_mesh(&gnd, 1);
        let color = |cell: usize, k: usize| vertices[cell * 4 + k].color;
        let black = [0.0, 0.0, 0.0, 1.0];
        let white = [1.0, 1.0, 1.0, 1.0];

        // The tinted cell darkens its own north-west corner and nothing else,
        // so its quad fades back to its neighbours' tint across the cell.
        assert_eq!(color(4, 0), black);
        assert_eq!(color(4, 1), white);
        assert_eq!(color(4, 2), white);
        assert_eq!(color(4, 3), white);

        // The neighbour sharing that corner darkens there too, and only there.
        assert_eq!(color(0, 3), black);
        assert_eq!(color(0, 0), white);
        assert_eq!(color(0, 1), white);
        assert_eq!(color(0, 2), white);

        // A cell that does not touch the corner is untouched.
        assert_eq!(color(8, 0), white);

        // Off-map corners contribute black.
        assert_eq!(color(8, 3), black);
    }

    /// Two cells side by side in x, heights given as [sw, se, nw, ne].
    fn gnd_two_cells(cell_a: [f32; 4], cell_b: [f32; 4]) -> GndFile {
        GndFile {
            version: (1, 7),
            width: 2,
            height: 1,
            zoom: 1.0,
            textures: vec!["ground.bmp".to_string()],
            lightmaps: Vec::new(),
            surfaces: vec![GndSurface {
                tex_u: [0.0, 1.0, 0.0, 1.0],
                tex_v: [0.0, 0.0, 1.0, 1.0],
                texture_id: 0,
                lightmap_id: -1,
                color_bgra: [255, 255, 255, 255],
            }],
            cells: [cell_a, cell_b]
                .iter()
                .map(|h| GndCell {
                    height_sw: h[0],
                    height_se: h[1],
                    height_nw: h[2],
                    height_ne: h[3],
                    surface_up: 0,
                    surface_south: -1,
                    surface_east: -1,
                })
                .collect(),
        }
    }

    #[test]
    fn equal_shared_corner_height_averages_the_normal_in_both_quads() {
        // Flat cell next to one sloping down along x, meeting at height 0.
        let gnd = gnd_two_cells([0.0, 0.0, 0.0, 0.0], [0.0, -1.0, 0.0, -1.0]);
        let (vertices, _, _) = build_mesh(&gnd, 1);

        let flat_sw = vertices[0].normal;
        let flat_se = vertices[1].normal;
        let slope_sw = vertices[4].normal;

        assert!(flat_sw[0].abs() < 1e-5 && (flat_sw[1] + 1.0).abs() < 1e-5);
        for i in 0..3 {
            assert!(
                (flat_se[i] - slope_sw[i]).abs() < 1e-5,
                "shared corner differs on axis {i}: {flat_se:?} vs {slope_sw:?}"
            );
        }
        assert!(flat_se[0] < -0.1, "shared corner did not tilt: {flat_se:?}");
    }

    #[test]
    fn height_step_leaves_each_quad_its_own_face_normal() {
        // Same slope, but dropped so no corner height matches the flat cell.
        let gnd = gnd_two_cells([0.0, 0.0, 0.0, 0.0], [-0.5, -1.5, -0.5, -1.5]);
        let (vertices, _, _) = build_mesh(&gnd, 1);

        let flat_se = vertices[1].normal;
        let slope_sw = vertices[4].normal;

        assert!(flat_se[0].abs() < 1e-5 && (flat_se[1] + 1.0).abs() < 1e-5);
        let diagonal = -std::f32::consts::FRAC_1_SQRT_2;
        assert!((slope_sw[0] - diagonal).abs() < 1e-4);
        assert!((slope_sw[1] - diagonal).abs() < 1e-4);
    }

    #[test]
    fn compute_quad_normal_points_up_for_flat_quad() {
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
        ];
        let normal = compute_quad_normal(&positions);
        // Flat horizontal quad: normal points -Y (upward in native RO coords)
        assert!(normal[1] < -0.99, "normal.y = {}", normal[1]);
        assert!(normal[0].abs() < 0.01);
        assert!(normal[2].abs() < 0.01);
    }
}
