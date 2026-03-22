use ragnarok_formats::act::{ActFile, SprClip, SpriteAnimationState, attachment_offset};
use ragnarok_formats::spr::RgbaImageData;

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
            device, queue, &img.data, img.width, img.height, layout, &label,
            wgpu::FilterMode::Nearest,
        );
        sizes.push((img.width, img.height));
        bind_groups.push(bg);
    }

    SpriteTextures { bind_groups, sizes, indexed_count }
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

pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
}

impl SpriteRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
        shader_source: &str,
    ) -> Self {
        use wgpu::util::DeviceExt;

        let uniform_data = SpriteUniforms {
            screen_size: [width as f32, height as f32],
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

        let pipeline = Self::create_pipeline(
            device, surface_format, &uniform_bind_group_layout,
            texture_bind_group_layout, shader_source,
        );

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            uniform_bind_group_layout,
            vertex_buffer,
            index_buffer,
            vertex_capacity: INITIAL_VERTEX_CAPACITY,
            index_capacity: INITIAL_INDEX_CAPACITY,
        }
    }

    fn create_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        uniform_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
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

    pub fn recreate_pipeline(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
    ) {
        self.pipeline = Self::create_pipeline(
            device, surface_format, &self.uniform_bind_group_layout,
            texture_layout, shader_source,
        );
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &SpriteUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    pub fn resize(&self, queue: &wgpu::Queue, width: u32, height: u32) {
        let uniforms = SpriteUniforms {
            screen_size: [width as f32, height as f32],
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
        depth_view: &wgpu::TextureView,
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: match clear_color {
                            Some(_) => wgpu::LoadOp::Clear(1.0),
                            None => wgpu::LoadOp::Load,
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            for batch in &draw_batches {
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
}

pub fn build_clip_quad(
    clip: &SprClip,
    textures: &SpriteTextures,
    screen_center: [f32; 2],
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

    let cx = screen_center[0] + (clip.x + offset[0]) as f32;
    let cy = screen_center[1] + (clip.y + offset[1]) as f32;

    let (mut u0, u1) = if clip.mirror != 0 { (1.0, 0.0) } else { (0.0, 1.0) };
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
        [ half_w, -half_h],
        [ half_w,  half_h],
        [-half_w,  half_h],
    ];

    let angle = -(clip.angle as f32).to_radians();
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let uvs = [
        [u0, v0],
        [u1, v0],
        [u1, v1],
        [u0, v1],
    ];

    let vertices: Vec<SpriteVertex> = corners.iter().zip(uvs.iter()).map(|(corner, uv)| {
        let rx = corner[0] * cos_a - corner[1] * sin_a;
        let ry = corner[0] * sin_a + corner[1] * cos_a;
        SpriteVertex {
            position: [cx + rx, cy + ry, depth],
            tex_coord: *uv,
            color,
        }
    }).collect();

    let indices = vec![0, 1, 2, 0, 2, 3];

    Some((vertices, indices, tex_index))
}

pub type ClipQuad = (Vec<SpriteVertex>, Vec<u32>, usize);

pub struct CompositeClips {
    pub body: Vec<ClipQuad>,
    pub head: Vec<ClipQuad>,
    pub weapon: Vec<ClipQuad>,
}

pub fn build_composite_clips(
    body_act: &ActFile,
    body_textures: &SpriteTextures,
    head_act: Option<&ActFile>,
    head_textures: Option<&SpriteTextures>,
    weapon_act: Option<&ActFile>,
    weapon_textures: Option<&SpriteTextures>,
    action_idx: usize,
    motion_idx: usize,
    screen_center: [f32; 2],
    depth: f32,
) -> Option<CompositeClips> {
    let body_action = &body_act.actions[action_idx];
    if body_action.motions.is_empty() {
        return None;
    }
    let body_motion = &body_action.motions[motion_idx % body_action.motions.len()];

    let mut body = Vec::new();
    for clip in &body_motion.clips {
        if let Some((vertices, indices, tex_idx)) = build_clip_quad(clip, body_textures, screen_center, depth, [0, 0]) {
            if tex_idx < body_textures.bind_groups.len() {
                body.push((vertices, indices, tex_idx));
            }
        }
    }

    let mut head = Vec::new();
    if let (Some(head_act), Some(head_tex)) = (head_act, head_textures) {
        let head_action_idx = action_idx % head_act.actions.len();
        let head_action = &head_act.actions[head_action_idx];
        if !head_action.motions.is_empty() {
            let head_motion = &head_action.motions[0];
            let (off_x, off_y) = attachment_offset(body_motion, head_motion);
            for clip in &head_motion.clips {
                if let Some((vertices, indices, tex_idx)) = build_clip_quad(clip, head_tex, screen_center, depth, [off_x, off_y]) {
                    if tex_idx < head_tex.bind_groups.len() {
                        head.push((vertices, indices, tex_idx));
                    }
                }
            }
        }
    }

    let mut weapon = Vec::new();
    if let (Some(weapon_act), Some(weapon_tex)) = (weapon_act, weapon_textures) {
        let weapon_action_idx = action_idx % weapon_act.actions.len();
        let weapon_action = &weapon_act.actions[weapon_action_idx];
        if !weapon_action.motions.is_empty() {
            let weapon_motion_idx = motion_idx % weapon_action.motions.len();
            let weapon_motion = &weapon_action.motions[weapon_motion_idx];
            let (off_x, off_y) = attachment_offset(body_motion, weapon_motion);
            for clip in &weapon_motion.clips {
                if let Some((vertices, indices, tex_idx)) = build_clip_quad(clip, weapon_tex, screen_center, depth, [off_x, off_y]) {
                    if tex_idx < weapon_tex.bind_groups.len() {
                        weapon.push((vertices, indices, tex_idx));
                    }
                }
            }
        }
    }

    Some(CompositeClips { body, head, weapon })
}

pub fn scale_clip_vertices(vertices: &mut [SpriteVertex], center: [f32; 2], scale: f32) {
    for v in vertices {
        v.position[0] = center[0] + (v.position[0] - center[0]) * scale;
        v.position[1] = center[1] + (v.position[1] - center[1]) * scale;
    }
}

pub struct EntitySprite {
    pub body_textures: SpriteTextures,
    pub body_act: ActFile,
    pub head_textures: Option<SpriteTextures>,
    pub head_act: Option<ActFile>,
    pub weapon_textures: Option<SpriteTextures>,
    pub weapon_act: Option<ActFile>,
    pub animation: SpriteAnimationState,
    pub shadow_textures: Option<SpriteTextures>,
    pub shadow_act: Option<ActFile>,
}

impl EntitySprite {
    pub fn update_animation(&mut self, dt_secs: f32, camera_dir: Option<u8>) {
        match camera_dir {
            Some(dir) => self.animation.update(dt_secs, &self.body_act, dir),
            None => self.animation.update_flat(dt_secs, &self.body_act),
        }
    }

    pub fn build_batches(
        &self,
        camera_dir: Option<u8>,
        screen_center: [f32; 2],
        depth: f32,
        scale: f32,
    ) -> Vec<SpriteBatch<'_>> {
        let action_idx = match camera_dir {
            Some(dir) => self.animation.action_index(&self.body_act, dir),
            None => self.animation.flat_action_index(&self.body_act),
        };

        let Some(clips) = build_composite_clips(
            &self.body_act, &self.body_textures,
            self.head_act.as_ref(), self.head_textures.as_ref(),
            self.weapon_act.as_ref(), self.weapon_textures.as_ref(),
            action_idx, self.animation.motion_index(), screen_center, depth,
        ) else {
            return Vec::new();
        };

        let mut batches = Vec::new();

        for (mut vertices, indices, tex_idx) in clips.body {
            scale_clip_vertices(&mut vertices, screen_center, scale);
            batches.push(SpriteBatch { vertices, indices, texture: &self.body_textures.bind_groups[tex_idx] });
        }
        if let Some(head_tex) = &self.head_textures {
            for (mut vertices, indices, tex_idx) in clips.head {
                scale_clip_vertices(&mut vertices, screen_center, scale);
                batches.push(SpriteBatch { vertices, indices, texture: &head_tex.bind_groups[tex_idx] });
            }
        }
        if let Some(weapon_tex) = &self.weapon_textures {
            for (mut vertices, indices, tex_idx) in clips.weapon {
                scale_clip_vertices(&mut vertices, screen_center, scale);
                batches.push(SpriteBatch { vertices, indices, texture: &weapon_tex.bind_groups[tex_idx] });
            }
        }

        batches
    }

    pub fn build_shadow_batches(
        &self,
        screen_center: [f32; 2],
        depth: f32,
        scale: f32,
    ) -> Vec<SpriteBatch<'_>> {
        let mut batches = Vec::new();
        if let (Some(shadow_act), Some(shadow_tex)) = (&self.shadow_act, &self.shadow_textures) {
            if !shadow_act.actions.is_empty() && !shadow_act.actions[0].motions.is_empty() {
                let shadow_motion = &shadow_act.actions[0].motions[0];
                for clip in &shadow_motion.clips {
                    if let Some((mut vertices, indices, tex_idx)) = build_clip_quad(clip, shadow_tex, screen_center, depth, [0, 0]) {
                        if tex_idx < shadow_tex.bind_groups.len() {
                            scale_clip_vertices(&mut vertices, screen_center, scale);
                            batches.push(SpriteBatch { vertices, indices, texture: &shadow_tex.bind_groups[tex_idx] });
                        }
                    }
                }
            }
        }
        batches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::act::SprClip;

    fn dummy_textures() -> SpriteTextures {
        SpriteTextures {
            bind_groups: Vec::new(),
            sizes: vec![(24, 24), (32, 32), (48, 48)],
            indexed_count: 2,
        }
    }

    #[test]
    fn build_clip_quad_basic_indexed_sprite() {
        let clip = SprClip {
            x: 0, y: 0, sprite_index: 0, mirror: 0,
            color: [255, 255, 255, 255],
            zoom_x: 1.0, zoom_y: 1.0, angle: 0,
            sprite_type: 0, width: None, height: None,
        };
        let textures = dummy_textures();
        let (verts, indices, tex_idx) = build_clip_quad(&clip, &textures, [100.0, 100.0], 0.5, [0, 0]).unwrap();

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
        let clip = SprClip {
            x: 10, y: -5, sprite_index: 0, mirror: 0,
            color: [255, 255, 255, 255],
            zoom_x: 1.0, zoom_y: 1.0, angle: 0,
            sprite_type: 1, width: None, height: None,
        };
        let textures = dummy_textures();
        let (_, _, tex_idx) = build_clip_quad(&clip, &textures, [200.0, 200.0], 0.0, [0, 0]).unwrap();
        // sprite_type 1 → indexed_count(2) + 0 = 2
        assert_eq!(tex_idx, 2);
    }

    #[test]
    fn build_clip_quad_mirrored_flips_uvs() {
        let clip = SprClip {
            x: 0, y: 0, sprite_index: 0, mirror: 1,
            color: [255, 255, 255, 255],
            zoom_x: 1.0, zoom_y: 1.0, angle: 0,
            sprite_type: 0, width: None, height: None,
        };
        let textures = dummy_textures();
        let (verts, _, _) = build_clip_quad(&clip, &textures, [100.0, 100.0], 0.0, [0, 0]).unwrap();
        // Top-left UV should be (1,0) when mirrored
        assert!((verts[0].tex_coord[0] - 1.0).abs() < 0.01);
        assert!((verts[1].tex_coord[0] - 0.0).abs() < 0.01);
    }

    #[test]
    fn build_clip_quad_negative_index_returns_none() {
        let clip = SprClip {
            x: 0, y: 0, sprite_index: -1, mirror: 0,
            color: [255, 255, 255, 255],
            zoom_x: 1.0, zoom_y: 1.0, angle: 0,
            sprite_type: 0, width: None, height: None,
        };
        let textures = dummy_textures();
        assert!(build_clip_quad(&clip, &textures, [100.0, 100.0], 0.0, [0, 0]).is_none());
    }

    #[test]
    fn build_clip_quad_with_zoom_scales_dimensions() {
        let clip = SprClip {
            x: 0, y: 0, sprite_index: 0, mirror: 0,
            color: [255, 255, 255, 255],
            zoom_x: 2.0, zoom_y: 0.5, angle: 0,
            sprite_type: 0, width: None, height: None,
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
