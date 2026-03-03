use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::WaterSettings;

use crate::device::DEPTH_FORMAT;
use crate::global_uniforms::GlobalUniforms;
use crate::texture::TextureCache;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaterVertex {
    pub position: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl WaterVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<WaterVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2,
        ],
    };
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct WaterUniforms {
    wave_height: f32,
    wave_pitch: f32,
    wave_offset: f32,
    opacity: f32,
}

const WATER_FRAMES: usize = 32;
const WATER_TEXTURE_REPEAT: f32 = 5.0;

pub struct WaterRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_names: Vec<String>,
    wave_height: f32,
    wave_pitch: f32,
    wave_speed: f32,
    anim_speed: f32,
}

impl WaterRenderer {
    pub fn from_water_settings(
        water: &WaterSettings,
        gnd: &GndFile,
        grf: &GrfArchive,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        global_uniforms: &GlobalUniforms,
        texture_cache: &mut TextureCache,
        surface_format: wgpu::TextureFormat,
    ) -> Option<Self> {
        let raw_level = water.level?;
        let zoom = gnd.zoom;
        let scale_factor = zoom / 10.0;
        let water_y = raw_level * scale_factor;

        let water_type = water.water_type.unwrap_or(0);
        let wave_height = water.wave_height.unwrap_or(1.0) * scale_factor;
        let wave_speed = water.wave_speed.unwrap_or(2.0);
        let wave_pitch = water.wave_pitch.unwrap_or(50.0);
        let anim_speed = water.anim_speed.unwrap_or(3) as f32;

        // Load water textures
        let texture_names: Vec<String> = (0..WATER_FRAMES).map(|i| {
            format!("data/texture/\u{C6CC}\u{D130}/water{}{:02}.jpg", water_type, i)
        }).collect();
        for name in &texture_names {
            texture_cache.get_or_load(name, grf, device, queue);
        }

        let (vertices, indices) = build_water_mesh(gnd, water_y);
        if vertices.is_empty() {
            return None;
        }

        let vertex_buffer = create_buffer(device, "water_vertices", &vertices, wgpu::BufferUsages::VERTEX);
        let index_buffer = create_buffer(device, "water_indices", &indices, wgpu::BufferUsages::INDEX);

        let uniforms = WaterUniforms {
            wave_height,
            wave_pitch,
            wave_offset: 0.0,
            opacity: 0.6,
        };

        let uniform_buffer = {
            use wgpu::util::DeviceExt;
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("water_uniforms"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        };

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water_uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water_uniforms"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline = create_pipeline(
            device,
            surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
            &uniform_bind_group_layout,
        );

        Some(Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            uniform_buffer,
            uniform_bind_group,
            texture_names,
            wave_height,
            wave_pitch,
            wave_speed,
            anim_speed,
        })
    }

    pub fn update(&self, queue: &wgpu::Queue, elapsed: f32) {
        let uniforms = WaterUniforms {
            wave_height: self.wave_height,
            wave_pitch: self.wave_pitch,
            wave_offset: elapsed * self.wave_speed * 100.0,
            opacity: 0.6,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        global_uniforms: &'a GlobalUniforms,
        texture_cache: &'a TextureCache,
        elapsed: f32,
    ) {
        let frame_idx = if self.anim_speed > 0.0 {
            ((elapsed / (self.anim_speed / 60.0)) as usize) % self.texture_names.len()
        } else {
            0
        };

        let tex_name = &self.texture_names[frame_idx];
        let tex_bg = match texture_cache.get(tex_name) {
            Some(bg) => bg,
            None => return,
        };

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &global_uniforms.bind_group, &[]);
        pass.set_bind_group(1, tex_bg, &[]);
        pass.set_bind_group(2, &self.uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

pub fn build_water_mesh(gnd: &GndFile, water_y: f32) -> (Vec<WaterVertex>, Vec<u32>) {
    let zoom = gnd.zoom;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for y in 0..gnd.height {
        for x in 0..gnd.width {
            let cell_idx = (y * gnd.width + x) as usize;
            let cell = &gnd.cells[cell_idx];

            let avg_height = (cell.height[0] + cell.height[1] + cell.height[2] + cell.height[3]) / 4.0;
            // In native RO coords, more negative = higher; skip cells far above water
            if avg_height < water_y - 5.0 * zoom {
                continue;
            }

            let wx = x as f32 * zoom;
            let wz = y as f32 * zoom;

            let u0 = (x as f32 % WATER_TEXTURE_REPEAT) / WATER_TEXTURE_REPEAT;
            let u1 = ((x + 1) as f32 % WATER_TEXTURE_REPEAT) / WATER_TEXTURE_REPEAT;
            let v0 = (y as f32 % WATER_TEXTURE_REPEAT) / WATER_TEXTURE_REPEAT;
            let v1 = ((y + 1) as f32 % WATER_TEXTURE_REPEAT) / WATER_TEXTURE_REPEAT;

            let base = vertices.len() as u32;
            vertices.push(WaterVertex { position: [wx,        water_y, wz],        tex_coord: [u0, v0] });
            vertices.push(WaterVertex { position: [wx + zoom, water_y, wz],        tex_coord: [u1, v0] });
            vertices.push(WaterVertex { position: [wx,        water_y, wz + zoom], tex_coord: [u0, v1] });
            vertices.push(WaterVertex { position: [wx + zoom, water_y, wz + zoom], tex_coord: [u1, v1] });

            indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
        }
    }

    (vertices, indices)
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
    uniform_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("water"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/water.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("water"),
        bind_group_layouts: &[
            global_bind_group_layout,
            texture_bind_group_layout,
            uniform_bind_group_layout,
        ],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("water"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[WaterVertex::LAYOUT],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
            depth_write_enabled: false,
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
    use ragnarok_formats::gnd::{GndCell, GndFile};

    fn make_gnd(width: i32, height: i32, cell_height: f32) -> GndFile {
        let cell_count = (width * height) as usize;
        GndFile {
            version: (1, 7),
            width,
            height,
            zoom: 10.0,
            textures: vec![],
            lightmaps: vec![],
            surfaces: vec![],
            cells: (0..cell_count).map(|_| GndCell {
                height: [cell_height; 4],
                top_surface: -1,
                front_surface: -1,
                right_surface: -1,
            }).collect(),
        }
    }

    #[test]
    fn water_mesh_generates_quads_for_cells_below_water() {
        // In native RO coords, more negative = higher; cell at -5 is above water at -10
        let gnd = make_gnd(4, 4, -5.0);
        let water_y = -10.0;
        let (vertices, indices) = build_water_mesh(&gnd, water_y);
        assert_eq!(vertices.len(), 16 * 4);
        assert_eq!(indices.len(), 16 * 6);
    }

    #[test]
    fn water_mesh_uv_tiling_repeats_every_5_cells() {
        let gnd = make_gnd(6, 1, -5.0);
        let (vertices, _) = build_water_mesh(&gnd, -10.0);
        // Cell 0: u0 = 0/5 = 0.0
        assert!((vertices[0].tex_coord[0] - 0.0).abs() < 0.01);
        // Cell 5 should wrap: 5 % 5 = 0
        let cell5_base = 5 * 4;
        assert!((vertices[cell5_base].tex_coord[0] - 0.0).abs() < 0.01);
    }
}
