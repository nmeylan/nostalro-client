use ragnarok_formats::gat::GatFile;
use ragnarok_formats::grf::GrfArchive;

use crate::device::DEPTH_FORMAT;
use crate::global_uniforms::GlobalUniforms;
use crate::texture::TextureCache;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridSelectorVertex {
    pub position: [f32; 3],
    pub tex_coord: [f32; 2],
    pub color: [f32; 4],
}

impl GridSelectorVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<GridSelectorVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2,
            2 => Float32x4,
        ],
    };
}

const HOVER_COLOR: [f32; 4] = [0.196, 0.941, 0.627, 0.6];
const GRID_WALKABLE_COLOR: [f32; 4] = [0.196, 0.941, 0.627, 0.4];
const GRID_WATER_COLOR: [f32; 4] = [0.2, 0.4, 0.9, 0.4];
const GRID_Y_OFFSET: f32 = -0.2;

pub struct GridSelectorRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    texture_name: String,
    hover_visible: bool,
    grid_vertex_buffer: Option<wgpu::Buffer>,
    grid_index_buffer: Option<wgpu::Buffer>,
    grid_index_count: u32,
    pub show_grid: bool,
}

impl GridSelectorRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        global_uniforms: &GlobalUniforms,
        texture_cache: &mut TextureCache,
        grf: &GrfArchive,
    ) -> Self {
        let texture_name = "data/texture/grid.tga".to_string();
        if texture_cache
            .get_or_load(&texture_name, grf, device, queue, false)
            .is_none()
        {
            let img = generate_grid_border_texture();
            let bind_group = crate::texture::create_texture_bind_group(
                device,
                queue,
                &img,
                &texture_cache.bind_group_layout,
                "grid_fallback",
            );
            texture_cache.insert(&texture_name, bind_group, img.width(), img.height());
        }

        let pipeline = create_pipeline(
            device,
            surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
        );

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid_selector_hover_vertices"),
            size: (4 * std::mem::size_of::<GridSelectorVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let indices: [u32; 6] = [0, 1, 2, 2, 1, 3];
        let index_buffer = {
            use wgpu::util::DeviceExt;
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("grid_selector_hover_indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            })
        };

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            texture_name,
            hover_visible: false,
            grid_vertex_buffer: None,
            grid_index_buffer: None,
            grid_index_count: 0,
            show_grid: false,
        }
    }

    pub fn update_hover(&self, queue: &wgpu::Queue, corners: [[f32; 3]; 4]) {
        let uvs = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let vertices: [GridSelectorVertex; 4] = std::array::from_fn(|i| GridSelectorVertex {
            position: corners[i],
            tex_coord: uvs[i],
            color: HOVER_COLOR,
        });
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    }

    pub fn set_hover_visible(&mut self, v: bool) {
        self.hover_visible = v;
    }

    pub fn build_grid_mesh(
        &mut self,
        device: &wgpu::Device,
        gat: &GatFile,
        gnd_w: i32,
        gnd_h: i32,
        zoom: f32,
    ) {
        let gat_w = gat.width;
        let gat_h = gat.height;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let cell_w = (gnd_w as f32 / gat_w as f32) * zoom;
        let cell_h = (gnd_h as f32 / gat_h as f32) * zoom;

        for cy in 0..gat_h {
            for cx in 0..gat_w {
                let cell = &gat.cells[(cy * gat_w + cx) as usize];
                let color = if cell.is_water() {
                    GRID_WATER_COLOR
                } else if cell.is_walkable() {
                    GRID_WALKABLE_COLOR
                } else {
                    continue;
                };

                let wx = cx as f32 * cell_w;
                let wz = cy as f32 * cell_h;

                let base = vertices.len() as u32;
                vertices.push(GridSelectorVertex {
                    position: [wx, cell.height_sw + GRID_Y_OFFSET, wz],
                    tex_coord: [0.0, 0.0],
                    color,
                });
                vertices.push(GridSelectorVertex {
                    position: [wx + cell_w, cell.height_se + GRID_Y_OFFSET, wz],
                    tex_coord: [1.0, 0.0],
                    color,
                });
                vertices.push(GridSelectorVertex {
                    position: [wx, cell.height_nw + GRID_Y_OFFSET, wz + cell_h],
                    tex_coord: [0.0, 1.0],
                    color,
                });
                vertices.push(GridSelectorVertex {
                    position: [wx + cell_w, cell.height_ne + GRID_Y_OFFSET, wz + cell_h],
                    tex_coord: [1.0, 1.0],
                    color,
                });

                indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base + 2,
                    base + 1,
                    base + 3,
                ]);
            }
        }

        if vertices.is_empty() {
            self.grid_vertex_buffer = None;
            self.grid_index_buffer = None;
            self.grid_index_count = 0;
            return;
        }

        use wgpu::util::DeviceExt;
        self.grid_vertex_buffer = Some(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("grid_overlay_vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));
        self.grid_index_buffer = Some(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("grid_overlay_indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            },
        ));
        self.grid_index_count = indices.len() as u32;
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        global_uniforms: &'a GlobalUniforms,
        texture_cache: &'a TextureCache,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &global_uniforms.bind_group, &[]);

        let Some(tex_bg) = texture_cache.get(&self.texture_name) else {
            return;
        };
        pass.set_bind_group(1, tex_bg, &[]);

        if self.show_grid
            && let (Some(vb), Some(ib)) = (&self.grid_vertex_buffer, &self.grid_index_buffer)
        {
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.grid_index_count, 0, 0..1);
        }

        if self.hover_visible {
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..6, 0, 0..1);
        }
    }
}

fn generate_grid_border_texture() -> image::RgbaImage {
    const SIZE: u32 = 32;
    const BORDER: u32 = 2;
    let mut img = image::RgbaImage::new(SIZE, SIZE);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let on_border =
                !(BORDER..SIZE - BORDER).contains(&x) || !(BORDER..SIZE - BORDER).contains(&y);
            let alpha = if on_border { 255 } else { 0 };
            img.put_pixel(x, y, image::Rgba([255, 255, 255, alpha]));
        }
    }
    img
}

fn create_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    global_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("grid_selector"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/grid_selector.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("grid_selector"),
        bind_group_layouts: &[global_bind_group_layout, texture_bind_group_layout],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("grid_selector"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[GridSelectorVertex::LAYOUT],
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
