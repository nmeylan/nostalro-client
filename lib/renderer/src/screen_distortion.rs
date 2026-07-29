//! Hallucination's screen ripple: the finished frame, UI and cursor included, is
//! resampled with a per-scanline horizontal sine offset.

/// Phase advance per frame.
const PHASE_STEP_DEG: f32 = 7.0;
/// Total phase from the bottom scanline to the top. The original advances 2° per
/// scanline, which spans this much over its 768-line target; keeping the span
/// fixed makes the wave count resolution-independent.
const SPAN_DEG: f32 = 1536.0;
/// Peak horizontal displacement as a fraction of screen width.
const AMPLITUDE: f32 = 0.01;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RippleUniform {
    base_phase: f32,
    span_phase: f32,
    amplitude: f32,
    _pad: f32,
}

struct SceneTarget {
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

pub struct ScreenDistortion {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    format: wgpu::TextureFormat,
    target: Option<SceneTarget>,
    phase_deg: f32,
    active: bool,
}

impl ScreenDistortion {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("screen_distortion"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/screen_distortion.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen_distortion"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("screen_distortion"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("screen_distortion"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        // Whole-pixel steps, like the original's row copies, and no filtering
        // across the seam the shift leaves at one edge.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("screen_distortion"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen_distortion"),
            size: std::mem::size_of::<RippleUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            uniform,
            format,
            target: None,
            phase_deg: 0.0,
            active: false,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        if !active {
            self.target = None;
            self.phase_deg = 0.0;
        }
        self.active = active;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The offscreen colour target the frame must be drawn into while the ripple
    /// is running.
    pub fn scene_view(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> wgpu::TextureView {
        let stale = self
            .target
            .as_ref()
            .is_none_or(|target| target.width != width || target.height != height);
        if stale {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("screen_distortion_scene"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("screen_distortion"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.uniform.as_entire_binding(),
                    },
                ],
            });
            self.target = Some(SceneTarget {
                view,
                bind_group,
                width,
                height,
            });
        }
        self.target
            .as_ref()
            .expect("target was just created")
            .view
            .clone()
    }

    /// Draw the offscreen frame to `output`, distorted, and advance the wave.
    pub fn resolve(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        output: &wgpu::TextureView,
    ) {
        let Some(target) = &self.target else {
            return;
        };
        self.phase_deg = (self.phase_deg + PHASE_STEP_DEG) % 360.0;
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&RippleUniform {
                base_phase: self.phase_deg.to_radians(),
                span_phase: SPAN_DEG.to_radians(),
                amplitude: AMPLITUDE,
                _pad: 0.0,
            }),
        );

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("screen_distortion"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &target.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
