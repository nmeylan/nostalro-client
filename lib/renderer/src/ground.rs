use ragnarok_formats::gnd::{GndFile, GndSurface, Lightmap};
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
}

pub struct GroundRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    batches: Vec<DrawBatch>,
    lightmap_bind_group: wgpu::BindGroup,
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
        // Preload all ground textures
        for tex_name in &gnd.textures {
            let path = format!("data/texture/{tex_name}");
            texture_cache.get_or_load(&path, grf, device, queue);
        }

        // Build lightmap atlas
        let lightmap_bind_group =
            build_lightmap_atlas(&gnd.lightmaps, device, queue, &texture_cache.bind_group_layout);

        // Build mesh grouped by texture
        let atlas_dim = lightmap_atlas_dim(gnd.lightmaps.len());
        let (vertices, indices, batches) = build_mesh(gnd, atlas_dim);

        let vertex_buffer = create_buffer(device, "ground_vertices", &vertices, wgpu::BufferUsages::VERTEX);
        let index_buffer = create_buffer(device, "ground_indices", &indices, wgpu::BufferUsages::INDEX);

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
        }
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        global_uniforms: &'a GlobalUniforms,
        texture_cache: &'a TextureCache,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &global_uniforms.bind_group, &[]);
        pass.set_bind_group(2, &self.lightmap_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        for batch in &self.batches {
            if let Some(tex_bg) = texture_cache.get(&batch.texture_name) {
                pass.set_bind_group(1, tex_bg, &[]);
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
    // Group quads by texture for batched rendering
    let mut texture_quads: std::collections::HashMap<String, (Vec<GroundVertex>, Vec<u32>)> =
        std::collections::HashMap::new();

    for y in 0..gnd.height {
        for x in 0..gnd.width {
            let cell_idx = (y * gnd.width + x) as usize;
            let cell = &gnd.cells[cell_idx];

            if cell.top_surface >= 0 {
                let surface = &gnd.surfaces[cell.top_surface as usize];
                let tex_name = texture_name_for_surface(gnd, surface);
                let color = bgra_to_rgba_f32(surface.color_bgra);

                let wx = x as f32 * gnd.zoom;
                let wz = y as f32 * gnd.zoom;

                let h = cell.height;
                let positions = [
                    [wx, h[0], wz],
                    [wx + gnd.zoom, h[1], wz],
                    [wx, h[2], wz + gnd.zoom],
                    [wx + gnd.zoom, h[3], wz + gnd.zoom],
                ];

                let normal = compute_quad_normal(&positions);

                let lm_uvs = lightmap_uvs(surface.lightmap_id, atlas_dim);

                let verts = [
                    GroundVertex {
                        position: positions[0],
                        normal,
                        tex_coord: [surface.u[0], surface.v[0]],
                        lightmap_coord: lm_uvs[0],
                        color,
                    },
                    GroundVertex {
                        position: positions[1],
                        normal,
                        tex_coord: [surface.u[1], surface.v[1]],
                        lightmap_coord: lm_uvs[1],
                        color,
                    },
                    GroundVertex {
                        position: positions[2],
                        normal,
                        tex_coord: [surface.u[2], surface.v[2]],
                        lightmap_coord: lm_uvs[2],
                        color,
                    },
                    GroundVertex {
                        position: positions[3],
                        normal,
                        tex_coord: [surface.u[3], surface.v[3]],
                        lightmap_coord: lm_uvs[3],
                        color,
                    },
                ];

                let entry = texture_quads.entry(tex_name).or_insert_with(|| (Vec::new(), Vec::new()));
                let base = entry.0.len() as u32;
                entry.0.extend_from_slice(&verts);
                entry.1.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
            }

            // Front wall
            if cell.front_surface >= 0 && y + 1 < gnd.height {
                let next_cell = &gnd.cells[((y + 1) * gnd.width + x) as usize];
                let surface = &gnd.surfaces[cell.front_surface as usize];
                let tex_name = texture_name_for_surface(gnd, surface);
                let color = bgra_to_rgba_f32(surface.color_bgra);

                let wx = x as f32 * gnd.zoom;
                let wz = (y + 1) as f32 * gnd.zoom;

                let positions = [
                    [wx, cell.height[2], wz],
                    [wx + gnd.zoom, cell.height[3], wz],
                    [wx, next_cell.height[0], wz],
                    [wx + gnd.zoom, next_cell.height[1], wz],
                ];

                let normal = compute_quad_normal(&positions);
                let lm_uvs = lightmap_uvs(surface.lightmap_id, atlas_dim);

                let verts = [
                    GroundVertex { position: positions[0], normal, tex_coord: [surface.u[0], surface.v[0]], lightmap_coord: lm_uvs[0], color },
                    GroundVertex { position: positions[1], normal, tex_coord: [surface.u[1], surface.v[1]], lightmap_coord: lm_uvs[1], color },
                    GroundVertex { position: positions[2], normal, tex_coord: [surface.u[2], surface.v[2]], lightmap_coord: lm_uvs[2], color },
                    GroundVertex { position: positions[3], normal, tex_coord: [surface.u[3], surface.v[3]], lightmap_coord: lm_uvs[3], color },
                ];

                let entry = texture_quads.entry(tex_name).or_insert_with(|| (Vec::new(), Vec::new()));
                let base = entry.0.len() as u32;
                entry.0.extend_from_slice(&verts);
                entry.1.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
            }

            // Right wall
            if cell.right_surface >= 0 && x + 1 < gnd.width {
                let next_cell = &gnd.cells[(y * gnd.width + x + 1) as usize];
                let surface = &gnd.surfaces[cell.right_surface as usize];
                let tex_name = texture_name_for_surface(gnd, surface);
                let color = bgra_to_rgba_f32(surface.color_bgra);

                let wx = (x + 1) as f32 * gnd.zoom;
                let wz = y as f32 * gnd.zoom;

                let positions = [
                    [wx, cell.height[1], wz],
                    [wx, next_cell.height[0], wz],
                    [wx, cell.height[3], wz + gnd.zoom],
                    [wx, next_cell.height[2], wz + gnd.zoom],
                ];

                let normal = compute_quad_normal(&positions);
                let lm_uvs = lightmap_uvs(surface.lightmap_id, atlas_dim);

                let verts = [
                    GroundVertex { position: positions[0], normal, tex_coord: [surface.u[0], surface.v[0]], lightmap_coord: lm_uvs[0], color },
                    GroundVertex { position: positions[1], normal, tex_coord: [surface.u[1], surface.v[1]], lightmap_coord: lm_uvs[1], color },
                    GroundVertex { position: positions[2], normal, tex_coord: [surface.u[2], surface.v[2]], lightmap_coord: lm_uvs[2], color },
                    GroundVertex { position: positions[3], normal, tex_coord: [surface.u[3], surface.v[3]], lightmap_coord: lm_uvs[3], color },
                ];

                let entry = texture_quads.entry(tex_name).or_insert_with(|| (Vec::new(), Vec::new()));
                let base = entry.0.len() as u32;
                entry.0.extend_from_slice(&verts);
                entry.1.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
            }
        }
    }

    // Flatten into single vertex/index buffers with draw batches
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    let mut batches = Vec::new();

    for (tex_name, (verts, idxs)) in texture_quads {
        let vertex_offset = all_vertices.len() as u32;
        let start_index = all_indices.len() as u32;
        all_vertices.extend_from_slice(&verts);
        // Offset indices by the vertex_offset
        all_indices.extend(idxs.iter().map(|i| i + vertex_offset));
        batches.push(DrawBatch {
            texture_name: tex_name,
            start_index,
            index_count: idxs.len() as u32,
        });
    }

    (all_vertices, all_indices, batches)
}

fn texture_name_for_surface(gnd: &GndFile, surface: &GndSurface) -> String {
    if surface.texture_id >= 0 && (surface.texture_id as usize) < gnd.textures.len() {
        format!("data/texture/{}", &gnd.textures[surface.texture_id as usize])
    } else {
        String::new()
    }
}

fn bgra_to_rgba_f32(bgra: [u8; 4]) -> [f32; 4] {
    [
        bgra[2] as f32 / 255.0, // R from B position
        bgra[1] as f32 / 255.0, // G
        bgra[0] as f32 / 255.0, // B from R position
        bgra[3] as f32 / 255.0, // A
    ]
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

fn lightmap_atlas_dim(lightmap_count: usize) -> u32 {
    if lightmap_count == 0 {
        return 1;
    }
    let grid = (lightmap_count as f64).sqrt().ceil() as u32;
    grid.max(1)
}

/// Returns UV coordinates for 4 corners of a lightmap cell within the atlas
fn lightmap_uvs(lightmap_id: i16, atlas_grid: u32) -> [[f32; 2]; 4] {
    if lightmap_id < 0 || atlas_grid == 0 {
        return [[0.0; 2]; 4];
    }
    let id = lightmap_id as u32;
    let gx = id % atlas_grid;
    let gy = id / atlas_grid;
    let cell_size = 1.0 / atlas_grid as f32;
    // Half-pixel inset to avoid bleeding
    let atlas_pixel_size = atlas_grid * 8;
    let half_texel = 0.5 / atlas_pixel_size as f32;

    let u0 = gx as f32 * cell_size + half_texel;
    let v0 = gy as f32 * cell_size + half_texel;
    let u1 = (gx + 1) as f32 * cell_size - half_texel;
    let v1 = (gy + 1) as f32 * cell_size - half_texel;

    [[u0, v0], [u1, v0], [u0, v1], [u1, v1]]
}

fn build_lightmap_atlas(
    lightmaps: &[Lightmap],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    let grid = lightmap_atlas_dim(lightmaps.len());
    let atlas_size = (grid * 8).max(1);

    let mut pixels = vec![255u8; (atlas_size * atlas_size * 4) as usize];

    for (i, lm) in lightmaps.iter().enumerate() {
        let gx = (i as u32 % grid) * 8;
        let gy = (i as u32 / grid) * 8;

        for ly in 0..8u32 {
            for lx in 0..8u32 {
                let lm_idx = (ly * 8 + lx) as usize;
                let intensity = lm.intensity[lm_idx];
                let spec_r = lm.specular[lm_idx * 3];
                let spec_g = lm.specular[lm_idx * 3 + 1];
                let spec_b = lm.specular[lm_idx * 3 + 2];

                let px = gx + lx;
                let py = gy + ly;
                let offset = ((py * atlas_size + px) * 4) as usize;
                // Intensity is the shadow/brightness map
                // Specular RGB is a color tint (often white = 255,255,255)
                // Use max(specular, intensity) to avoid zeroing out
                pixels[offset] = spec_r.max(intensity);
                pixels[offset + 1] = spec_g.max(intensity);
                pixels[offset + 2] = spec_b.max(intensity);
                pixels[offset + 3] = 255;
            }
        }
    }

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

    // group 0: global uniforms, group 1: ground texture, group 2: lightmap texture
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
        // BGRA: B=255, G=128, R=0, A=200
        let result = bgra_to_rgba_f32([255, 128, 0, 200]);
        assert!((result[0] - 0.0).abs() < 0.01); // R from bgra[2]
        assert!((result[1] - 128.0 / 255.0).abs() < 0.01);
        assert!((result[2] - 1.0).abs() < 0.01); // B from bgra[0]
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
        // ID 15 should be in the last cell of a 4x4 grid
        let uvs = lightmap_uvs(15, 4);
        assert!(uvs[3][0] > 0.7);
        assert!(uvs[3][1] > 0.7);
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
