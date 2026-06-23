use ragnarok_formats::act::{ActFile, Motion, SpriteFrame, attachment_offset};
use ragnarok_formats::spr::{RgbaImageData, SpriteData};

use crate::device::DEPTH_FORMAT;
use crate::texture::create_texture_bind_group_from_rgba;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 3],
    pub tex_coord: [f32; 2],
    pub color: [f32; 4],
}

impl SpriteVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x4,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRIBS,
    };
}

pub struct SpriteTextures {
    pub bind_groups: Vec<wgpu::BindGroup>,
    pub sizes: Vec<(u32, u32)>,
    pub indexed_count: usize,
}

pub fn upload_sprite_textures(
    images: &[RgbaImageData],
    indexed_count: usize,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> SpriteTextures {
    let mut bind_groups = Vec::with_capacity(images.len());
    let mut sizes = Vec::with_capacity(images.len());

    for (i, img) in images.iter().enumerate() {
        let label = if i < indexed_count {
            format!("spr_idx_{i}")
        } else {
            format!("spr_rgba_{}", i - indexed_count)
        };
        let bg = create_texture_bind_group_from_rgba(
            device,
            queue,
            &img.data,
            img.width,
            img.height,
            layout,
            &label,
            wgpu::FilterMode::Nearest,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::AddressMode::ClampToEdge,
        );
        sizes.push((img.width, img.height));
        bind_groups.push(bg);
    }

    SpriteTextures {
        bind_groups,
        sizes,
        indexed_count,
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteUniforms {
    pub screen_size: [f32; 2],
    pub zoom: f32,
    pub _pad: f32,
    pub pan: [f32; 2],
    pub _pad2: [f32; 2],
}

const INITIAL_VERTEX_CAPACITY: usize = 1024;
const INITIAL_INDEX_CAPACITY: usize = 2048;

/// Depth configuration for a sprite pipeline.
#[derive(Clone, Copy)]
enum SpriteDepth {
    /// No depth-stencil attachment — for passes that bind none (UI/login).
    None,
    /// Depth attachment with `LessEqual` test; writes when `write` is set.
    Test { write: bool },
    /// Depth attachment bound but test/write disabled (`Always`, no write):
    /// a no-depth-check sprite, so it draws over the
    /// floor instead of being occluded by ground it sits at or below.
    Overlay,
}

pub struct SpriteRenderer {
    pub pipeline: wgpu::RenderPipeline,
    pub pipeline_no_depth: wgpu::RenderPipeline,
    pub pipeline_additive: wgpu::RenderPipeline,
    pub pipeline_additive_no_depth: wgpu::RenderPipeline,
    /// Sprite drawn with the depth attachment bound but the test/write
    /// disabled (no depth check): renders over the
    /// floor instead of being occluded by ground it sits at or below. Used by
    /// the effect dispatch's `AlphaNoDepth` / `AdditiveNoDepth` buckets, which
    /// run in a pass that HAS a depth attachment (so the attachment-less
    /// `pipeline_no_depth` is format-incompatible there).
    pub pipeline_overlay: wgpu::RenderPipeline,
    pub pipeline_additive_overlay: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
    /// Whether the depth-enabled pipelines write to the depth buffer.
    /// Entity sprites do so the post-sprite effect pass depth-tests against
    /// them (front of cylinder draws over sprite, back fails depth test —
    /// matches the original game's on-screen occlusion, where the entity
    /// sprite pass keeps depth-write on). Effect sprites (STR / ambient SPR) skip
    /// the write so per-emitter particle layering isn't blocked.
    depth_write: bool,
}

impl SpriteRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        logical_width: f32,
        logical_height: f32,
        shader_source: &str,
        depth_write: bool,
    ) -> Self {
        use wgpu::util::DeviceExt;

        let uniform_data = SpriteUniforms {
            screen_size: [logical_width, logical_height],
            zoom: 1.0,
            _pad: 0.0,
            pan: [0.0, 0.0],
            _pad2: [0.0, 0.0],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_uniforms"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite_uniforms"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite_uniforms"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_vertices"),
            size: (INITIAL_VERTEX_CAPACITY * std::mem::size_of::<SpriteVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_indices"),
            size: (INITIAL_INDEX_CAPACITY * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let alpha = wgpu::BlendState::ALPHA_BLENDING;
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let pipeline = Self::create_pipeline(
            device,
            surface_format,
            &uniform_bind_group_layout,
            texture_bind_group_layout,
            shader_source,
            alpha,
            SpriteDepth::Test { write: depth_write },
        );
        let pipeline_no_depth = Self::create_pipeline(
            device,
            surface_format,
            &uniform_bind_group_layout,
            texture_bind_group_layout,
            shader_source,
            alpha,
            SpriteDepth::None,
        );
        let pipeline_additive = Self::create_pipeline(
            device,
            surface_format,
            &uniform_bind_group_layout,
            texture_bind_group_layout,
            shader_source,
            additive,
            SpriteDepth::Test { write: depth_write },
        );
        let pipeline_additive_no_depth = Self::create_pipeline(
            device,
            surface_format,
            &uniform_bind_group_layout,
            texture_bind_group_layout,
            shader_source,
            additive,
            SpriteDepth::None,
        );
        let pipeline_overlay = Self::create_pipeline(
            device,
            surface_format,
            &uniform_bind_group_layout,
            texture_bind_group_layout,
            shader_source,
            alpha,
            SpriteDepth::Overlay,
        );
        let pipeline_additive_overlay = Self::create_pipeline(
            device,
            surface_format,
            &uniform_bind_group_layout,
            texture_bind_group_layout,
            shader_source,
            additive,
            SpriteDepth::Overlay,
        );

        Self {
            pipeline,
            pipeline_no_depth,
            pipeline_additive,
            pipeline_additive_no_depth,
            pipeline_overlay,
            pipeline_additive_overlay,
            uniform_buffer,
            uniform_bind_group,
            uniform_bind_group_layout,
            vertex_buffer,
            index_buffer,
            vertex_capacity: INITIAL_VERTEX_CAPACITY,
            index_capacity: INITIAL_INDEX_CAPACITY,
            depth_write,
        }
    }

    fn create_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        uniform_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
        blend: wgpu::BlendState,
        depth: SpriteDepth,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite"),
            bind_group_layouts: &[uniform_layout, texture_layout],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[SpriteVertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: match depth {
                SpriteDepth::None => None,
                SpriteDepth::Test { write } => Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: write,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                SpriteDepth::Overlay => Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
            },
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    pub fn recreate_pipeline(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
    ) {
        let alpha = wgpu::BlendState::ALPHA_BLENDING;
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        self.pipeline = Self::create_pipeline(
            device,
            surface_format,
            &self.uniform_bind_group_layout,
            texture_layout,
            shader_source,
            alpha,
            SpriteDepth::Test { write: self.depth_write },
        );
        self.pipeline_no_depth = Self::create_pipeline(
            device,
            surface_format,
            &self.uniform_bind_group_layout,
            texture_layout,
            shader_source,
            alpha,
            SpriteDepth::None,
        );
        self.pipeline_additive = Self::create_pipeline(
            device,
            surface_format,
            &self.uniform_bind_group_layout,
            texture_layout,
            shader_source,
            additive,
            SpriteDepth::Test { write: self.depth_write },
        );
        self.pipeline_additive_no_depth = Self::create_pipeline(
            device,
            surface_format,
            &self.uniform_bind_group_layout,
            texture_layout,
            shader_source,
            additive,
            SpriteDepth::None,
        );
        self.pipeline_overlay = Self::create_pipeline(
            device,
            surface_format,
            &self.uniform_bind_group_layout,
            texture_layout,
            shader_source,
            alpha,
            SpriteDepth::Overlay,
        );
        self.pipeline_additive_overlay = Self::create_pipeline(
            device,
            surface_format,
            &self.uniform_bind_group_layout,
            texture_layout,
            shader_source,
            additive,
            SpriteDepth::Overlay,
        );
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &SpriteUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    pub fn resize(&self, queue: &wgpu::Queue, logical_width: f32, logical_height: f32) {
        let uniforms = SpriteUniforms {
            screen_size: [logical_width, logical_height],
            zoom: 1.0,
            _pad: 0.0,
            pan: [0.0, 0.0],
            _pad2: [0.0, 0.0],
        };
        self.update_uniforms(queue, &uniforms);
    }

    /// Render sprite batches. If `clear_color` is Some, clears the target first;
    /// if None, uses LoadOp::Load to overlay on existing content.
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        clear_color: Option<wgpu::Color>,
        batches: &[SpriteBatch],
    ) {
        let total_verts: usize = batches.iter().map(|b| b.vertices.len()).sum();
        let total_indices: usize = batches.iter().map(|b| b.indices.len()).sum();
        if total_verts == 0 {
            return;
        }

        if total_verts > self.vertex_capacity {
            self.vertex_capacity = total_verts.next_power_of_two();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_vertices"),
                size: (self.vertex_capacity * std::mem::size_of::<SpriteVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if total_indices > self.index_capacity {
            self.index_capacity = total_indices.next_power_of_two();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_indices"),
                size: (self.index_capacity * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        struct DrawBatch<'a> {
            texture: &'a wgpu::BindGroup,
            index_start: u32,
            index_count: u32,
            additive: bool,
        }

        let mut all_verts = Vec::with_capacity(total_verts);
        let mut all_indices = Vec::with_capacity(total_indices);
        let mut draw_batches = Vec::with_capacity(batches.len());

        for batch in batches {
            let vertex_offset = all_verts.len() as u32;
            let index_start = all_indices.len() as u32;
            all_verts.extend_from_slice(&batch.vertices);
            all_indices.extend(batch.indices.iter().map(|i| i + vertex_offset));
            draw_batches.push(DrawBatch {
                texture: batch.texture,
                index_start,
                index_count: batch.indices.len() as u32,
                additive: batch.additive,
            });
        }

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_verts));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_indices));

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: match clear_color {
                            Some(color) => wgpu::LoadOp::Clear(color),
                            None => wgpu::LoadOp::Load,
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_view.map(|dv| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: dv,
                        depth_ops: Some(wgpu::Operations {
                            load: match clear_color {
                                Some(_) => wgpu::LoadOp::Clear(1.0),
                                None => wgpu::LoadOp::Load,
                            },
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                ..Default::default()
            });

            let has_depth = depth_view.is_some();
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            let mut current_additive = false;
            let initial_pipeline = if has_depth {
                &self.pipeline
            } else {
                &self.pipeline_no_depth
            };
            pass.set_pipeline(initial_pipeline);

            for batch in &draw_batches {
                if batch.additive != current_additive {
                    current_additive = batch.additive;
                    let pipeline = match (current_additive, has_depth) {
                        (false, true) => &self.pipeline,
                        (false, false) => &self.pipeline_no_depth,
                        (true, true) => &self.pipeline_additive,
                        (true, false) => &self.pipeline_additive_no_depth,
                    };
                    pass.set_pipeline(pipeline);
                }
                pass.set_bind_group(1, batch.texture, &[]);
                pass.draw_indexed(
                    batch.index_start..batch.index_start + batch.index_count,
                    0,
                    0..1,
                );
            }
        }
    }
}

pub struct SpriteBatch<'a> {
    pub vertices: Vec<SpriteVertex>,
    pub indices: Vec<u32>,
    pub texture: &'a wgpu::BindGroup,
    pub additive: bool,
}

pub fn build_clip_quad(
    clip: &SpriteFrame,
    textures: &SpriteTextures,
    screen_anchor: [f32; 2],
    depth: f32,
    offset: [i32; 2],
) -> Option<(Vec<SpriteVertex>, Vec<u32>, usize)> {
    if clip.sprite_index < 0 {
        return None;
    }

    let tex_index = if clip.sprite_type == 0 {
        clip.sprite_index as usize
    } else {
        textures.indexed_count + clip.sprite_index as usize
    };

    if tex_index >= textures.sizes.len() {
        return None;
    }

    let (tex_w, tex_h) = textures.sizes[tex_index];
    let (w, h) = match (clip.width, clip.height) {
        (Some(cw), Some(ch)) if cw > 0 && ch > 0 => (cw as f32, ch as f32),
        _ => (tex_w as f32, tex_h as f32),
    };

    let scaled_w = w * clip.zoom_x;
    let scaled_h = h * clip.zoom_y;
    let half_w = scaled_w / 2.0;
    let half_h = scaled_h / 2.0;

    let cx = screen_anchor[0] + (clip.x + offset[0]) as f32;
    let cy = screen_anchor[1] + (clip.y + offset[1]) as f32;

    let (mut u0, u1) = if clip.mirror != 0 {
        (1.0, 0.0)
    } else {
        (0.0, 1.0)
    };
    let (v0, v1) = (0.0f32, 1.0f32);
    let _ = &mut u0; // suppress unused_mut

    let color = [
        clip.color[0] as f32 / 255.0,
        clip.color[1] as f32 / 255.0,
        clip.color[2] as f32 / 255.0,
        clip.color[3] as f32 / 255.0,
    ];

    // Corner offsets relative to center
    let corners = [
        [-half_w, -half_h],
        [half_w, -half_h],
        [half_w, half_h],
        [-half_w, half_h],
    ];

    let angle = -(clip.angle as f32).to_radians();
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let uvs = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];

    let vertices: Vec<SpriteVertex> = corners
        .iter()
        .zip(uvs.iter())
        .map(|(corner, uv)| {
            let rx = corner[0] * cos_a - corner[1] * sin_a;
            let ry = corner[0] * sin_a + corner[1] * cos_a;
            SpriteVertex {
                position: [cx + rx, cy + ry, depth],
                tex_coord: *uv,
                color,
            }
        })
        .collect();

    let indices = vec![0, 1, 2, 0, 2, 3];

    Some((vertices, indices, tex_index))
}

pub type ClipQuad = (Vec<SpriteVertex>, Vec<u32>, usize);

pub struct CompositeClips {
    pub body: Vec<ClipQuad>,
    pub head: Vec<ClipQuad>,
    pub headgear_bottom: Vec<ClipQuad>,
    pub headgear_mid: Vec<ClipQuad>,
    pub headgear_top: Vec<ClipQuad>,
    pub weapon: Vec<ClipQuad>,
    pub weapon_trail: Vec<ClipQuad>,
    pub shield: Vec<ClipQuad>,
}

pub fn build_composite_clips(
    entity: &EntitySprite,
    action_idx: usize,
    motion_idx: usize,
    head_dir: u8,
    screen_anchor: [f32; 2],
    depth: f32,
) -> Option<CompositeClips> {
    let body_action = &entity.body_act.actions[action_idx];
    if body_action.motions.is_empty() {
        return None;
    }
    let body_motion = &body_action.motions[motion_idx % body_action.motions.len()];

    let base_action = action_idx / 8;
    let is_idle_or_sit = base_action == 0 || base_action == 2;

    let mut body = Vec::new();
    for clip in &body_motion.clips {
        if let Some((vertices, indices, tex_idx)) =
            build_clip_quad(clip, &entity.body_textures, screen_anchor, depth, [0, 0])
            && tex_idx < entity.body_textures.bind_groups.len()
        {
            body.push((vertices, indices, tex_idx));
        }
    }

    let mut head = Vec::new();
    if let (Some(head_act), Some(head_tex)) = (&entity.head_act, &entity.head_textures) {
        let head_action_idx = action_idx % head_act.actions.len();
        let head_action = &head_act.actions[head_action_idx];
        if !head_action.motions.is_empty() {
            let head_motion_idx = if is_idle_or_sit {
                head_dir as usize % head_action.motions.len()
            } else {
                motion_idx % head_action.motions.len()
            };
            let head_motion = &head_action.motions[head_motion_idx];
            let (off_x, off_y) = attachment_offset(body_motion, head_motion);
            for clip in &head_motion.clips {
                if let Some((vertices, indices, tex_idx)) =
                    build_clip_quad(clip, head_tex, screen_anchor, depth, [off_x, off_y])
                    && tex_idx < head_tex.bind_groups.len()
                {
                    head.push((vertices, indices, tex_idx));
                }
            }
        }
    }

    fn build_headgear_clips(
        act: Option<&ActFile>,
        tex: Option<&SpriteTextures>,
        action_idx: usize,
        motion_idx: usize,
        head_dir: u8,
        is_idle_or_sit: bool,
        body_motion: &Motion,
        screen_anchor: [f32; 2],
        depth: f32,
    ) -> Vec<ClipQuad> {
        let mut clips = Vec::new();
        if let (Some(act), Some(tex)) = (act, tex) {
            let hg_action_idx = action_idx % act.actions.len();
            let hg_action = &act.actions[hg_action_idx];
            if !hg_action.motions.is_empty() {
                let hg_motion_idx = if is_idle_or_sit {
                    head_dir as usize % hg_action.motions.len()
                } else {
                    motion_idx % hg_action.motions.len()
                };
                let hg_motion = &hg_action.motions[hg_motion_idx];
                let (off_x, off_y) = attachment_offset(body_motion, hg_motion);
                for clip in &hg_motion.clips {
                    if let Some((vertices, indices, tex_idx)) =
                        build_clip_quad(clip, tex, screen_anchor, depth, [off_x, off_y])
                        && tex_idx < tex.bind_groups.len()
                    {
                        clips.push((vertices, indices, tex_idx));
                    }
                }
            }
        }
        clips
    }

    let headgear_bottom = build_headgear_clips(
        entity.headgear_bottom_act.as_ref(),
        entity.headgear_bottom_textures.as_ref(),
        action_idx,
        motion_idx,
        head_dir,
        is_idle_or_sit,
        body_motion,
        screen_anchor,
        depth,
    );
    let headgear_mid = build_headgear_clips(
        entity.headgear_mid_act.as_ref(),
        entity.headgear_mid_textures.as_ref(),
        action_idx,
        motion_idx,
        head_dir,
        is_idle_or_sit,
        body_motion,
        screen_anchor,
        depth,
    );
    let headgear_top = build_headgear_clips(
        entity.headgear_top_act.as_ref(),
        entity.headgear_top_textures.as_ref(),
        action_idx,
        motion_idx,
        head_dir,
        is_idle_or_sit,
        body_motion,
        screen_anchor,
        depth,
    );

    let mut weapon = Vec::new();
    if let (Some(weapon_act), Some(weapon_tex)) = (&entity.weapon_act, &entity.weapon_textures) {
        let weapon_action_idx = action_idx % weapon_act.actions.len();
        let weapon_action = &weapon_act.actions[weapon_action_idx];
        if !weapon_action.motions.is_empty() {
            let weapon_motion_idx = motion_idx % weapon_action.motions.len();
            let weapon_motion = &weapon_action.motions[weapon_motion_idx];
            let (off_x, off_y) = attachment_offset(body_motion, weapon_motion);
            for clip in &weapon_motion.clips {
                if let Some((vertices, indices, tex_idx)) =
                    build_clip_quad(clip, weapon_tex, screen_anchor, depth, [off_x, off_y])
                    && tex_idx < weapon_tex.bind_groups.len()
                {
                    weapon.push((vertices, indices, tex_idx));
                }
            }
        }
    }

    // Weapon trail (`검광`): attaches to the body exactly like the weapon.
    let mut weapon_trail = Vec::new();
    if let (Some(trail_act), Some(trail_tex)) =
        (&entity.weapon_trail_act, &entity.weapon_trail_textures)
    {
        let trail_action_idx = action_idx % trail_act.actions.len();
        let trail_action = &trail_act.actions[trail_action_idx];
        if !trail_action.motions.is_empty() {
            let trail_motion_idx = motion_idx % trail_action.motions.len();
            let trail_motion = &trail_action.motions[trail_motion_idx];
            let (off_x, off_y) = attachment_offset(body_motion, trail_motion);
            for clip in &trail_motion.clips {
                if let Some((vertices, indices, tex_idx)) =
                    build_clip_quad(clip, trail_tex, screen_anchor, depth, [off_x, off_y])
                    && tex_idx < trail_tex.bind_groups.len()
                {
                    weapon_trail.push((vertices, indices, tex_idx));
                }
            }
        }
    }

    let mut shield = Vec::new();
    if let (Some(shield_act), Some(shield_tex)) = (&entity.shield_act, &entity.shield_textures) {
        let shield_action_idx = action_idx % shield_act.actions.len();
        let shield_action = &shield_act.actions[shield_action_idx];
        if !shield_action.motions.is_empty() {
            let shield_motion_idx = motion_idx % shield_action.motions.len();
            let shield_motion = &shield_action.motions[shield_motion_idx];
            let (off_x, off_y) = attachment_offset(body_motion, shield_motion);
            for clip in &shield_motion.clips {
                if let Some((vertices, indices, tex_idx)) =
                    build_clip_quad(clip, shield_tex, screen_anchor, depth, [off_x, off_y])
                    && tex_idx < shield_tex.bind_groups.len()
                {
                    shield.push((vertices, indices, tex_idx));
                }
            }
        }
    }

    Some(CompositeClips {
        body,
        head,
        headgear_bottom,
        headgear_mid,
        headgear_top,
        weapon,
        weapon_trail,
        shield,
    })
}

pub fn scale_clip_vertices(
    vertices: &mut [SpriteVertex],
    center: [f32; 2],
    scale: f32,
    depth_gradient: f32,
) {
    for v in vertices {
        v.position[0] = center[0] + (v.position[0] - center[0]) * scale;
        v.position[1] = center[1] + (v.position[1] - center[1]) * scale;
        v.position[2] += depth_gradient * (v.position[1] - center[1]);
    }
}

pub fn rotate_sprite_vertices(
    vertices: &mut [SpriteVertex],
    center: [f32; 2],
    angle: f32,
) {
    let (sin, cos) = angle.sin_cos();
    for v in vertices {
        let dx = v.position[0] - center[0];
        let dy = v.position[1] - center[1];
        v.position[0] = center[0] + dx * cos - dy * sin;
        v.position[1] = center[1] + dx * sin + dy * cos;
    }
}

pub struct EntitySprite {
    pub body_textures: SpriteTextures,
    pub body_act: ActFile,
    pub head_textures: Option<SpriteTextures>,
    pub head_act: Option<ActFile>,
    pub weapon_textures: Option<SpriteTextures>,
    pub weapon_act: Option<ActFile>,
    pub weapon_trail_textures: Option<SpriteTextures>,
    pub weapon_trail_act: Option<ActFile>,
    pub headgear_top_textures: Option<SpriteTextures>,
    pub headgear_top_act: Option<ActFile>,
    pub headgear_mid_textures: Option<SpriteTextures>,
    pub headgear_mid_act: Option<ActFile>,
    pub headgear_bottom_textures: Option<SpriteTextures>,
    pub headgear_bottom_act: Option<ActFile>,
    pub shield_textures: Option<SpriteTextures>,
    pub shield_act: Option<ActFile>,
    pub shadow_textures: Option<SpriteTextures>,
    pub shadow_act: Option<ActFile>,
}

fn upload_optional(
    data: Option<SpriteData>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> (Option<SpriteTextures>, Option<ActFile>) {
    match data {
        Some(d) => {
            let tex = upload_sprite_textures(&d.images, d.indexed_count, device, queue, layout);
            (Some(tex), Some(d.act))
        }
        None => (None, None),
    }
}

pub fn build_entity_sprite(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    body: SpriteData,
    head: Option<SpriteData>,
    weapon: Option<SpriteData>,
    weapon_trail: Option<SpriteData>,
    headgear_top: Option<SpriteData>,
    headgear_mid: Option<SpriteData>,
    headgear_bottom: Option<SpriteData>,
    shield: Option<SpriteData>,
    shadow: Option<SpriteData>,
) -> EntitySprite {
    let body_textures =
        upload_sprite_textures(&body.images, body.indexed_count, device, queue, layout);
    let body_act = body.act;
    let (head_textures, head_act) = upload_optional(head, device, queue, layout);
    let (weapon_textures, weapon_act) = upload_optional(weapon, device, queue, layout);
    let (weapon_trail_textures, weapon_trail_act) =
        upload_optional(weapon_trail, device, queue, layout);
    let (headgear_top_textures, headgear_top_act) =
        upload_optional(headgear_top, device, queue, layout);
    let (headgear_mid_textures, headgear_mid_act) =
        upload_optional(headgear_mid, device, queue, layout);
    let (headgear_bottom_textures, headgear_bottom_act) =
        upload_optional(headgear_bottom, device, queue, layout);
    let (shield_textures, shield_act) = upload_optional(shield, device, queue, layout);
    let (shadow_textures, shadow_act) = upload_optional(shadow, device, queue, layout);

    EntitySprite {
        body_textures,
        body_act,
        head_textures,
        head_act,
        weapon_textures,
        weapon_act,
        weapon_trail_textures,
        weapon_trail_act,
        headgear_top_textures,
        headgear_top_act,
        headgear_mid_textures,
        headgear_mid_act,
        headgear_bottom_textures,
        headgear_bottom_act,
        shield_textures,
        shield_act,
        shadow_textures,
        shadow_act,
    }
}

const MAX_PICK_WIDTH: f32 = 200.0;
const MAX_PICK_HEIGHT: f32 = 250.0;
const PICK_BOTTOM_MARGIN: f32 = 10.0;

impl EntitySprite {
    /// Compute screen-space pick bounding box from the sprite's current animation frame.
    /// Returns [left, top, right, bottom] capped to max 200x250, centered on anchor.
    /// Minimum hit-test inflation is applied separately in the picking logic.
    pub fn compute_pick_bounds(
        &self,
        animation: &ragnarok_formats::act::SpriteAnimationState,
        camera_dir: Option<u8>,
        head_dir: u8,
        screen_anchor: [f32; 2],
        depth: f32,
        scale: f32,
    ) -> [f32; 4] {
        let action_idx = match camera_dir {
            Some(dir) => animation.action_index(&self.body_act, dir),
            None => animation.flat_action_index(&self.body_act),
        };

        let clips = build_composite_clips(
            self,
            action_idx,
            animation.motion_index(),
            head_dir,
            screen_anchor,
            depth,
        );

        let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        let mut has_vertices = false;

        if let Some(clips) = clips {
            let all_groups: [&Vec<ClipQuad>; 7] = [
                &clips.body,
                &clips.head,
                &clips.headgear_bottom,
                &clips.headgear_mid,
                &clips.headgear_top,
                &clips.weapon,
                &clips.shield,
            ];
            for group in all_groups {
                for (vertices, _, _) in group {
                    for v in vertices {
                        let sx = screen_anchor[0] + (v.position[0] - screen_anchor[0]) * scale;
                        let sy = screen_anchor[1] + (v.position[1] - screen_anchor[1]) * scale;
                        min_x = min_x.min(sx);
                        min_y = min_y.min(sy);
                        max_x = max_x.max(sx);
                        max_y = max_y.max(sy);
                        has_vertices = true;
                    }
                }
            }
        }

        if !has_vertices {
            let half = 20.0 * scale;
            return [
                screen_anchor[0] - half,
                screen_anchor[1] - half * 2.0,
                screen_anchor[0] + half,
                screen_anchor[1],
            ];
        }

        let raw_w = max_x - min_x;
        let raw_h = max_y - min_y;
        let w = raw_w.min(MAX_PICK_WIDTH * scale);
        let h = raw_h.min(MAX_PICK_HEIGHT * scale);
        let bottom = max_y.min(screen_anchor[1] + PICK_BOTTOM_MARGIN);
        [
            screen_anchor[0] - w / 2.0,
            bottom - h,
            screen_anchor[0] + w / 2.0,
            bottom,
        ]
    }

    /// Distance in screen pixels from the feet anchor up to the top of the idle pose
    /// (action 0, motion 0) for the entity's current facing direction. Used to position
    /// floating overlays (chat bubbles, damage numbers, cast bar) so they don't move
    /// with per-frame animation changes.
    pub fn compute_head_offset(
        &self,
        animation: &ragnarok_formats::act::SpriteAnimationState,
        camera_dir: Option<u8>,
        head_dir: u8,
        screen_anchor: [f32; 2],
        depth: f32,
        scale: f32,
    ) -> f32 {
        let action_count = self.body_act.actions.len();
        if action_count == 0 {
            return 40.0 * scale;
        }
        let idle_action_idx = match camera_dir {
            Some(dir) => (dir as usize + 12 - animation.direction()) % 8 % action_count,
            None => animation.direction() % action_count,
        };

        let Some(clips) =
            build_composite_clips(self, idle_action_idx, 0, head_dir, screen_anchor, depth)
        else {
            return 40.0 * scale;
        };

        let all_groups: [&Vec<ClipQuad>; 7] = [
            &clips.body,
            &clips.head,
            &clips.headgear_bottom,
            &clips.headgear_mid,
            &clips.headgear_top,
            &clips.weapon,
            &clips.shield,
        ];
        let mut min_y = f32::MAX;
        let mut has_vertices = false;
        for group in all_groups {
            for (vertices, _, _) in group {
                for v in vertices {
                    let sy = screen_anchor[1] + (v.position[1] - screen_anchor[1]) * scale;
                    if sy < min_y {
                        min_y = sy;
                    }
                    has_vertices = true;
                }
            }
        }

        if !has_vertices {
            return 40.0 * scale;
        }
        (screen_anchor[1] - min_y).max(0.0)
    }

    pub fn build_batches(
        &self,
        animation: &ragnarok_formats::act::SpriteAnimationState,
        camera_dir: Option<u8>,
        head_dir: u8,
        screen_anchor: [f32; 2],
        depth: f32,
        scale: f32,
        depth_gradient: f32,
    ) -> Vec<SpriteBatch<'_>> {
        let action_idx = match camera_dir {
            Some(dir) => animation.action_index(&self.body_act, dir),
            None => animation.flat_action_index(&self.body_act),
        };

        let Some(clips) = build_composite_clips(
            self,
            action_idx,
            animation.motion_index(),
            head_dir,
            screen_anchor,
            depth,
        ) else {
            return Vec::new();
        };

        let mut batches = Vec::new();

        let effective_dir = action_idx % 8;
        let shield_behind = effective_dir > 1 && effective_dir < 6;

        let mut shield_batches = Vec::new();
        if let Some(shield_tex) = &self.shield_textures {
            for (mut vertices, indices, tex_idx) in clips.shield {
                scale_clip_vertices(&mut vertices, screen_anchor, scale, depth_gradient);
                shield_batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &shield_tex.bind_groups[tex_idx],
                    additive: false,
                });
            }
        }

        if shield_behind {
            batches.append(&mut shield_batches);
        }

        for (mut vertices, indices, tex_idx) in clips.body {
            scale_clip_vertices(&mut vertices, screen_anchor, scale, depth_gradient);
            batches.push(SpriteBatch {
                vertices,
                indices,
                texture: &self.body_textures.bind_groups[tex_idx],
                additive: false,
            });
        }
        if let Some(head_tex) = &self.head_textures {
            for (mut vertices, indices, tex_idx) in clips.head {
                scale_clip_vertices(&mut vertices, screen_anchor, scale, depth_gradient);
                batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &head_tex.bind_groups[tex_idx],
                    additive: false,
                });
            }
        }
        if let Some(hg_tex) = &self.headgear_bottom_textures {
            for (mut vertices, indices, tex_idx) in clips.headgear_bottom {
                scale_clip_vertices(&mut vertices, screen_anchor, scale, depth_gradient);
                batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &hg_tex.bind_groups[tex_idx],
                    additive: false,
                });
            }
        }
        if let Some(hg_tex) = &self.headgear_mid_textures {
            for (mut vertices, indices, tex_idx) in clips.headgear_mid {
                scale_clip_vertices(&mut vertices, screen_anchor, scale, depth_gradient);
                batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &hg_tex.bind_groups[tex_idx],
                    additive: false,
                });
            }
        }
        if let Some(hg_tex) = &self.headgear_top_textures {
            for (mut vertices, indices, tex_idx) in clips.headgear_top {
                scale_clip_vertices(&mut vertices, screen_anchor, scale, depth_gradient);
                batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &hg_tex.bind_groups[tex_idx],
                    additive: false,
                });
            }
        }
        if let Some(weapon_tex) = &self.weapon_textures {
            for (mut vertices, indices, tex_idx) in clips.weapon {
                scale_clip_vertices(&mut vertices, screen_anchor, scale, depth_gradient);
                batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &weapon_tex.bind_groups[tex_idx],
                    additive: false,
                });
            }
        }
        if !shield_behind {
            batches.extend(shield_batches);
        }

        batches
    }

    /// Render only the weapon-trail (`검광`) layer — the Quicken swing arc. The
    /// caller draws it additively on top of the body and tints it (yellow under
    /// Quicken). Empty when the weapon has no trail sprite or the current motion
    /// has no trail frames (so it only shows during the attack swing).
    pub fn build_weapon_trail_batches(
        &self,
        animation: &ragnarok_formats::act::SpriteAnimationState,
        camera_dir: Option<u8>,
        head_dir: u8,
        screen_anchor: [f32; 2],
        depth: f32,
        scale: f32,
    ) -> Vec<SpriteBatch<'_>> {
        let Some(trail_tex) = &self.weapon_trail_textures else {
            return Vec::new();
        };
        let action_idx = match camera_dir {
            Some(dir) => animation.action_index(&self.body_act, dir),
            None => animation.flat_action_index(&self.body_act),
        };
        let Some(clips) = build_composite_clips(
            self,
            action_idx,
            animation.motion_index(),
            head_dir,
            screen_anchor,
            depth,
        ) else {
            return Vec::new();
        };
        let mut batches = Vec::new();
        for (mut vertices, indices, tex_idx) in clips.weapon_trail {
            scale_clip_vertices(&mut vertices, screen_anchor, scale, 0.0);
            batches.push(SpriteBatch {
                vertices,
                indices,
                texture: &trail_tex.bind_groups[tex_idx],
                additive: true,
            });
        }
        batches
    }

    pub fn build_shadow_batches(
        &self,
        screen_anchor: [f32; 2],
        depth: f32,
        scale: f32,
    ) -> Vec<SpriteBatch<'_>> {
        let mut batches = Vec::new();
        if let (Some(shadow_act), Some(shadow_tex)) = (&self.shadow_act, &self.shadow_textures)
            && !shadow_act.actions.is_empty()
            && !shadow_act.actions[0].motions.is_empty()
        {
            let shadow_motion = &shadow_act.actions[0].motions[0];
            for clip in &shadow_motion.clips {
                if let Some((mut vertices, indices, tex_idx)) =
                    build_clip_quad(clip, shadow_tex, screen_anchor, depth, [0, 0])
                    && tex_idx < shadow_tex.bind_groups.len()
                {
                    scale_clip_vertices(&mut vertices, screen_anchor, scale, 0.0);
                    batches.push(SpriteBatch {
                        vertices,
                        indices,
                        texture: &shadow_tex.bind_groups[tex_idx],
                        additive: false,
                    });
                }
            }
        }
        batches
    }
}

/// Every per-entity body modifier resolved for one frame, bundled so the
/// actor pass fetches them in a single call and feeds them to
/// [`compose_actor_batches`]. Covers the original game's body-modifier set
/// (shake, tint, scale, yaw, roll angle, vertical lift, and the
/// multi-render copies).
#[derive(Clone, Debug)]
pub struct BodyChannels {
    pub shake: [f32; 2],
    pub tint: Option<[u8; 3]>,
    pub scale: f32,
    pub yaw: f32,
    /// Final opacity multiplier for the live body (effect fade folded with the
    /// caller's hidden/death fade).
    pub alpha: f32,
    /// Screen-space vertical lift in pixels (positive = up).
    pub lift_px: f32,
    /// Sprite-quad rotation about the anchor, radians.
    pub angle: f32,
    /// Vertical scale about the feet anchor (1.0 = none); `<1.0` presses the
    /// top toward the bottom (Pressedbody squash).
    pub squeeze: f32,
    /// Render the live body additively (light-body glow): a glowing, partly
    /// see-through body (dark pixels vanish) instead of the opaque sprite.
    pub additive: bool,
    /// Render the per-weapon swing trail (`검광`) on top of the body — the
    /// Quicken family's attack arc.
    pub weapon_trail: bool,
    pub copies: Vec<ragnarok_game::effect::BodyCopy>,
}

impl Default for BodyChannels {
    fn default() -> Self {
        Self {
            shake: [0.0, 0.0],
            tint: None,
            scale: 1.0,
            yaw: 0.0,
            alpha: 1.0,
            lift_px: 0.0,
            angle: 0.0,
            squeeze: 1.0,
            additive: false,
            weapon_trail: false,
            copies: Vec::new(),
        }
    }
}

/// Rotate (radians) and stretch (`scale` x/y) a batch's vertices about
/// `anchor` in screen space. No-op when there's nothing to do.
pub fn transform_batch_vertices(
    batches: &mut [SpriteBatch],
    anchor: [f32; 2],
    radians: f32,
    scale: [f32; 2],
) {
    if radians == 0.0 && scale == [1.0, 1.0] {
        return;
    }
    let (sin, cos) = radians.sin_cos();
    for batch in batches {
        for v in &mut batch.vertices {
            let dx = (v.position[0] - anchor[0]) * scale[0];
            let dy = (v.position[1] - anchor[1]) * scale[1];
            v.position[0] = anchor[0] + dx * cos - dy * sin;
            v.position[1] = anchor[1] + dx * sin + dy * cos;
        }
    }
}

fn apply_tint_alpha(batches: &mut [SpriteBatch], tint: Option<[u8; 3]>, alpha: f32) {
    let tint = tint.map(|t| [t[0] as f32 / 255.0, t[1] as f32 / 255.0, t[2] as f32 / 255.0]);
    for batch in batches {
        for v in &mut batch.vertices {
            if let Some([tr, tg, tb]) = tint {
                v.color[0] *= tr;
                v.color[1] *= tg;
                v.color[2] *= tb;
            }
            if alpha != 1.0 {
                v.color[3] *= alpha;
            }
        }
    }
}

/// Build the actor's sprite batches with every body channel applied in one
/// place: yaw cycles the 8-way facing, shake + lift offset the anchor, scale
/// multiplies the size, [`BodyCopy`] copies render behind the live body, and
/// the live body takes the rotation, tint and alpha. Shared by the game scene
/// and the effect viewer so the two never drift. Afterimage trails are drawn
/// separately by the caller (they need holder mutation + the un-yawed facing).
#[allow(clippy::too_many_arguments)]
pub fn compose_actor_batches<'a>(
    sprite: &'a EntitySprite,
    animation: &ragnarok_formats::act::SpriteAnimationState,
    camera_dir: u8,
    head_dir: u8,
    screen_anchor: [f32; 2],
    depth: f32,
    base_scale: f32,
    channels: &BodyChannels,
) -> Vec<SpriteBatch<'a>> {
    // Body yaw: convert the accumulated yaw to direction
    // steps and rotate the camera-relative facing index.
    let dir = if channels.yaw != 0.0 {
        let steps = (channels.yaw / (std::f32::consts::TAU / 8.0)).round() as i32;
        (((camera_dir as i32 + steps) % 8 + 8) % 8) as u8
    } else {
        camera_dir
    };
    let anchor = [
        screen_anchor[0] + channels.shake[0],
        screen_anchor[1] + channels.shake[1] - channels.lift_px,
    ];
    let scale = base_scale * channels.scale;

    let mut live = sprite.build_batches(animation, Some(dir), head_dir, anchor, depth, scale, 0.0);
    // Bounding box of the natural (untransformed) sprite — copies scale
    // concentrically about its centre (russian-doll / halo), so they surround the
    // body on all sides instead of growing off the feet.
    let (body_center, body_w, body_h) = batches_bbox(&live)
        .map(|(min, max)| {
            (
                [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5],
                (max[0] - min[0]).max(1.0),
                (max[1] - min[1]).max(1.0),
            )
        })
        .unwrap_or((anchor, 1.0, 1.0));

    let build_copy = |copy: &ragnarok_game::effect::BodyCopy| {
        let mut batches =
            sprite.build_batches(animation, Some(dir), head_dir, anchor, depth, scale, 0.0);
        // `margin_px` grows the copy by a fixed number of pixels on every edge
        // (an even border inflation) — the right knob for a small concentric
        // halo/ripple. Otherwise a uniform scale (`[s, s]`) adds a *proportional*
        // margin (fatter top/bottom than sides on a tall sprite), so convert that
        // to an even pixel margin too. Explicit non-uniform scales (hit-line
        // stretch) pass through.
        let scale_xy = if copy.margin_px != 0.0 {
            [(body_w + 2.0 * copy.margin_px) / body_w, (body_h + 2.0 * copy.margin_px) / body_h]
        } else if (copy.scale[0] - copy.scale[1]).abs() < 1e-6 && copy.scale[1] != 1.0 {
            let margin = (copy.scale[1] - 1.0) * body_h * 0.5;
            [(body_w + 2.0 * margin) / body_w, copy.scale[1]]
        } else {
            copy.scale
        };
        transform_batch_vertices(&mut batches, body_center, 0.0, scale_xy);
        // `offset_px` then slides the whole (centre-scaled) copy — the
        // ghosts that drift outward in four directions.
        if copy.offset_px != [0.0, 0.0] {
            for b in &mut batches {
                for v in &mut b.vertices {
                    v.position[0] += copy.offset_px[0];
                    v.position[1] += copy.offset_px[1];
                }
            }
        }
        apply_tint_alpha(&mut batches, Some(copy.tint), copy.alpha);
        for b in &mut batches {
            b.additive = copy.additive;
        }
        batches
    };

    let mut out = Vec::new();
    // Copies marked `behind` sit BEHIND the live sprite — larger and fainter, so
    // the parts outside the body silhouette show as a russian-doll halo / glow,
    // while the opaque body covers (and so leaves unchanged) the inner part.
    for copy in channels.copies.iter().filter(|c| c.behind) {
        out.append(&mut build_copy(copy));
    }

    if channels.squeeze != 1.0 {
        // Compress the body vertically toward the feet anchor.
        transform_batch_vertices(&mut live, anchor, 0.0, [1.0, channels.squeeze]);
    }
    if channels.angle != 0.0 {
        // A body roll pivots about the entity's centre, not its feet.
        transform_batch_vertices(&mut live, body_center, channels.angle, [1.0, 1.0]);
    }
    apply_tint_alpha(&mut live, channels.tint, channels.alpha);
    if channels.additive {
        // Light-body glow: the body itself blends additively, so dark texels add
        // nothing (see-through) and the tint reads as a glow over the scene.
        for b in &mut live {
            b.additive = true;
        }
    }
    out.append(&mut live);

    // Quicken weapon-swing trail (`검광`): drawn on top of the body, additive,
    // tinted by the buff (yellow). The trail sprite only carries frames in the
    // attack motions, so it appears only during the swing — the iconic arc.
    if channels.weapon_trail {
        let mut trail =
            sprite.build_weapon_trail_batches(animation, Some(dir), head_dir, anchor, depth, scale);
        apply_tint_alpha(&mut trail, channels.tint, channels.alpha);
        out.append(&mut trail);
    }

    // Copies marked on-top overlay the live sprite (glow rim,
    // asura halo, hit flash).
    for copy in channels.copies.iter().filter(|c| !c.behind) {
        out.append(&mut build_copy(copy));
    }
    out
}

/// Bounding box `(min, max)` of a batch set's vertices in screen space.
fn batches_bbox(batches: &[SpriteBatch]) -> Option<([f32; 2], [f32; 2])> {
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    let mut any = false;
    for batch in batches {
        for v in &batch.vertices {
            any = true;
            min_x = min_x.min(v.position[0]);
            min_y = min_y.min(v.position[1]);
            max_x = max_x.max(v.position[0]);
            max_y = max_y.max(v.position[1]);
        }
    }
    any.then_some(([min_x, min_y], [max_x, max_y]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::act::SpriteFrame;

    fn dummy_textures() -> SpriteTextures {
        SpriteTextures {
            bind_groups: Vec::new(),
            sizes: vec![(24, 24), (32, 32), (48, 48)],
            indexed_count: 2,
        }
    }

    #[test]
    fn build_clip_quad_basic_indexed_sprite() {
        let clip = SpriteFrame {
            x: 0,
            y: 0,
            sprite_index: 0,
            mirror: 0,
            color: [255, 255, 255, 255],
            zoom_x: 1.0,
            zoom_y: 1.0,
            angle: 0,
            sprite_type: 0,
            width: None,
            height: None,
        };
        let textures = dummy_textures();
        let (verts, indices, tex_idx) =
            build_clip_quad(&clip, &textures, [100.0, 100.0], 0.5, [0, 0]).unwrap();

        assert_eq!(tex_idx, 0);
        assert_eq!(verts.len(), 4);
        assert_eq!(indices, [0, 1, 2, 0, 2, 3]);
        // 24x24 sprite centered at (100, 100)
        assert!((verts[0].position[0] - 88.0).abs() < 0.01);
        assert!((verts[0].position[1] - 88.0).abs() < 0.01);
        assert!((verts[2].position[0] - 112.0).abs() < 0.01);
        assert!((verts[2].position[1] - 112.0).abs() < 0.01);
        // Depth should be passed through
        assert!((verts[0].position[2] - 0.5).abs() < 0.001);
    }

    #[test]
    fn build_clip_quad_rgba_sprite_with_offset() {
        let clip = SpriteFrame {
            x: 10,
            y: -5,
            sprite_index: 0,
            mirror: 0,
            color: [255, 255, 255, 255],
            zoom_x: 1.0,
            zoom_y: 1.0,
            angle: 0,
            sprite_type: 1,
            width: None,
            height: None,
        };
        let textures = dummy_textures();
        let (_, _, tex_idx) =
            build_clip_quad(&clip, &textures, [200.0, 200.0], 0.0, [0, 0]).unwrap();
        // sprite_type 1 → indexed_count(2) + 0 = 2
        assert_eq!(tex_idx, 2);
    }

    #[test]
    fn build_clip_quad_mirrored_flips_uvs() {
        let clip = SpriteFrame {
            x: 0,
            y: 0,
            sprite_index: 0,
            mirror: 1,
            color: [255, 255, 255, 255],
            zoom_x: 1.0,
            zoom_y: 1.0,
            angle: 0,
            sprite_type: 0,
            width: None,
            height: None,
        };
        let textures = dummy_textures();
        let (verts, _, _) = build_clip_quad(&clip, &textures, [100.0, 100.0], 0.0, [0, 0]).unwrap();
        // Top-left UV should be (1,0) when mirrored
        assert!((verts[0].tex_coord[0] - 1.0).abs() < 0.01);
        assert!((verts[1].tex_coord[0] - 0.0).abs() < 0.01);
    }

    #[test]
    fn build_clip_quad_negative_index_returns_none() {
        let clip = SpriteFrame {
            x: 0,
            y: 0,
            sprite_index: -1,
            mirror: 0,
            color: [255, 255, 255, 255],
            zoom_x: 1.0,
            zoom_y: 1.0,
            angle: 0,
            sprite_type: 0,
            width: None,
            height: None,
        };
        let textures = dummy_textures();
        assert!(build_clip_quad(&clip, &textures, [100.0, 100.0], 0.0, [0, 0]).is_none());
    }

    #[test]
    fn build_clip_quad_with_zoom_scales_dimensions() {
        let clip = SpriteFrame {
            x: 0,
            y: 0,
            sprite_index: 0,
            mirror: 0,
            color: [255, 255, 255, 255],
            zoom_x: 2.0,
            zoom_y: 0.5,
            angle: 0,
            sprite_type: 0,
            width: None,
            height: None,
        };
        let textures = dummy_textures();
        let (verts, _, _) = build_clip_quad(&clip, &textures, [100.0, 100.0], 0.0, [0, 0]).unwrap();
        // 24 * 2.0 = 48 wide, 24 * 0.5 = 12 tall
        let w = verts[1].position[0] - verts[0].position[0];
        let h = verts[3].position[1] - verts[0].position[1];
        assert!((w - 48.0).abs() < 0.01);
        assert!((h - 12.0).abs() < 0.01);
    }
}
