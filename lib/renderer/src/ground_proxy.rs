//! Debug/tooling ground proxy — a large flat checker quad at `y = 0`.
//!
//! The real map ground is drawn by [`crate::ground::GroundRenderer`] when a
//! `.gnd` is loaded. Tools like the effect viewer never load a map yet still
//! need *something* in the depth buffer for effect primitives to clip
//! against (so a sphere's lower hemisphere is hidden the same way it would
//! be in-game).
//!
//! Geometry is a single screen-spanning quad at the world XZ plane, with
//! depth writes enabled. The fragment shader produces a simple checker so
//! the impact point is visually readable. Native RO coords: `-Y = up`, so
//! `y = 0` is the floor.

use crate::camera::Camera;
use crate::device::DEPTH_FORMAT;
use crate::global_uniforms::GlobalUniforms;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ProxyVertex {
    position: [f32; 3],
}

impl ProxyVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![
        0 => Float32x3,
    ];

    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRIBS,
    };
}

/// Edge length of the proxy quad, in world units. Large enough that any
/// reasonable camera placement keeps it filling the floor.
const HALF_SIZE: f32 = 5000.0;

pub struct GroundProxyRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl GroundProxyRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ground_proxy"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/ground_proxy.wgsl").into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ground_proxy"),
            bind_group_layouts: &[camera_bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ground_proxy"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[ProxyVertex::LAYOUT],
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
        });

        let s = HALF_SIZE;
        let verts: [ProxyVertex; 6] = [
            ProxyVertex { position: [-s, 0.0, -s] },
            ProxyVertex { position: [ s, 0.0, -s] },
            ProxyVertex { position: [ s, 0.0,  s] },
            ProxyVertex { position: [-s, 0.0, -s] },
            ProxyVertex { position: [ s, 0.0,  s] },
            ProxyVertex { position: [-s, 0.0,  s] },
        ];
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ground_proxy_vertices"),
            size: std::mem::size_of_val(&verts) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            vertex_count: verts.len() as u32,
        }
    }

    /// Upload the static quad once; safe to call on construction.
    pub fn initialise(&self, queue: &wgpu::Queue) {
        let s = HALF_SIZE;
        let verts: [ProxyVertex; 6] = [
            ProxyVertex { position: [-s, 0.0, -s] },
            ProxyVertex { position: [ s, 0.0, -s] },
            ProxyVertex { position: [ s, 0.0,  s] },
            ProxyVertex { position: [-s, 0.0, -s] },
            ProxyVertex { position: [ s, 0.0,  s] },
            ProxyVertex { position: [-s, 0.0,  s] },
        ];
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        global_uniforms: &'a GlobalUniforms,
        _camera: &Camera,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &global_uniforms.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}
