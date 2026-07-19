use crate::effect::primitives::{
    CylinderRenderer, FrustumRenderer, FullscreenOverlayRenderer, GroundDiscRenderer,
    LineStripRenderer, QuadHornRenderer, RadialRingRenderer, SphereRenderer, Texture3DRenderer,
    WorldQuadRenderer,
};
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, partition_and_sort};
use crate::sprite::{SpriteRenderer, SpriteVertex};

const INITIAL_VERTEX_CAPACITY: usize = 4096;
const INITIAL_INDEX_CAPACITY: usize = 8192;

pub struct EffectDispatcher {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
}

impl EffectDispatcher {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effect_dispatch_vertices"),
            size: (INITIAL_VERTEX_CAPACITY * std::mem::size_of::<SpriteVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effect_dispatch_indices"),
            size: (INITIAL_INDEX_CAPACITY * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            vertex_buffer,
            index_buffer,
            vertex_capacity: INITIAL_VERTEX_CAPACITY,
            index_capacity: INITIAL_INDEX_CAPACITY,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn dispatch<'tex>(
        &mut self,
        records: Vec<DrawRecord<'tex>>,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group: &wgpu::BindGroup,
        sprite_uniform_bind_group: &wgpu::BindGroup,
        sprite_renderer: &SpriteRenderer,
        frustum: &FrustumRenderer,
        cylinder: &CylinderRenderer,
        ground_disc: &GroundDiscRenderer,
        quad_horn: &QuadHornRenderer,
        sphere: &SphereRenderer,
        world_quad: &WorldQuadRenderer,
        texture3d: &Texture3DRenderer,
        radial_ring: &RadialRingRenderer,
        line_strip: &LineStripRenderer,
        fullscreen: &FullscreenOverlayRenderer,
    ) {
        if records.is_empty() {
            return;
        }

        let buckets = partition_and_sort(&records);
        let total_active: usize = buckets.iter().map(|b| b.len()).sum();
        if total_active == 0 {
            return;
        }

        struct Span<'tex> {
            kind: PipelineKind,
            bucket: BlendBucket,
            texture: &'tex wgpu::BindGroup,
            index_start: u32,
            index_count: u32,
        }

        let total_verts: usize = records.iter().map(|r| r.vertices.len()).sum();
        let total_indices: usize = records.iter().map(|r| r.indices.len()).sum();
        if total_verts == 0 || total_indices == 0 {
            return;
        }

        let mut all_verts: Vec<SpriteVertex> = Vec::with_capacity(total_verts);
        let mut all_indices: Vec<u32> = Vec::with_capacity(total_indices);
        let mut spans: Vec<Span<'tex>> = Vec::with_capacity(total_active);

        struct VtxRange {
            vertex_offset: u32,
            index_start: u32,
            index_count: u32,
        }
        let mut ranges: Vec<VtxRange> = Vec::with_capacity(records.len());
        for r in &records {
            let vertex_offset = all_verts.len() as u32;
            let index_start = all_indices.len() as u32;
            all_verts.extend_from_slice(&r.vertices);
            all_indices.extend(r.indices.iter().map(|i| i + vertex_offset));
            ranges.push(VtxRange {
                vertex_offset,
                index_start,
                index_count: r.indices.len() as u32,
            });
        }

        for bucket in BlendBucket::FLUSH_ORDER {
            let list = &buckets[bucket.flush_index()];
            for &record_idx in list {
                let r = &records[record_idx];
                let range = &ranges[record_idx];
                spans.push(Span {
                    kind: r.pipeline,
                    bucket,
                    texture: r.texture,
                    index_start: range.index_start,
                    index_count: range.index_count,
                });
            }
        }

        if all_verts.len() > self.vertex_capacity {
            self.vertex_capacity = all_verts.len().next_power_of_two();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("effect_dispatch_vertices"),
                size: (self.vertex_capacity * std::mem::size_of::<SpriteVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_verts));
        if all_indices.len() > self.index_capacity {
            self.index_capacity = all_indices.len().next_power_of_two();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("effect_dispatch_indices"),
                size: (self.index_capacity * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_indices));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("effect_unified"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        let mut current_kind: Option<PipelineKind> = None;
        let mut current_bucket: Option<BlendBucket> = None;
        let mut current_group0_kind: Option<PipelineKind> = None;

        for span in spans {
            let group0_kind = match span.kind {
                PipelineKind::Sprite => PipelineKind::Sprite,
                _ => PipelineKind::Frustum,
            };
            if current_group0_kind != Some(group0_kind) {
                match group0_kind {
                    PipelineKind::Sprite => {
                        pass.set_bind_group(0, sprite_uniform_bind_group, &[]);
                    }
                    _ => {
                        pass.set_bind_group(0, camera_bind_group, &[]);
                    }
                }
                current_group0_kind = Some(group0_kind);
            }

            if current_kind != Some(span.kind) || current_bucket != Some(span.bucket) {
                let pipeline = pipeline_for(
                    span.kind,
                    span.bucket,
                    sprite_renderer,
                    frustum,
                    cylinder,
                    ground_disc,
                    quad_horn,
                    sphere,
                    world_quad,
                    texture3d,
                    radial_ring,
                    line_strip,
                    fullscreen,
                );
                pass.set_pipeline(pipeline);
                current_kind = Some(span.kind);
                current_bucket = Some(span.bucket);
            }

            pass.set_bind_group(1, span.texture, &[]);
            pass.draw_indexed(
                span.index_start..span.index_start + span.index_count,
                0,
                0..1,
            );
        }
    }
}

fn pipeline_for<'a>(
    kind: PipelineKind,
    bucket: BlendBucket,
    sprite_renderer: &'a SpriteRenderer,
    frustum: &'a FrustumRenderer,
    cylinder: &'a CylinderRenderer,
    ground_disc: &'a GroundDiscRenderer,
    quad_horn: &'a QuadHornRenderer,
    sphere: &'a SphereRenderer,
    world_quad: &'a WorldQuadRenderer,
    texture3d: &'a Texture3DRenderer,
    radial_ring: &'a RadialRingRenderer,
    line_strip: &'a LineStripRenderer,
    fullscreen: &'a FullscreenOverlayRenderer,
) -> &'a wgpu::RenderPipeline {
    let additive = matches!(bucket, BlendBucket::Additive | BlendBucket::AdditiveNoDepth)
        || matches!(bucket, BlendBucket::Multiply);
    match kind {
        PipelineKind::Frustum => {
            if additive {
                &frustum.pipeline_additive
            } else {
                &frustum.pipeline_alpha
            }
        }
        PipelineKind::Cylinder => {
            if additive {
                &cylinder.pipeline_additive
            } else {
                &cylinder.pipeline_alpha
            }
        }
        PipelineKind::GroundDisc => match bucket {
            BlendBucket::AlphaNoDepth => &ground_disc.pipeline_alpha_no_depth,
            BlendBucket::AdditiveNoDepth => &ground_disc.pipeline_additive_no_depth,
            _ if additive => &ground_disc.pipeline_additive,
            _ => &ground_disc.pipeline_alpha,
        },
        PipelineKind::QuadHorn => {
            if additive {
                &quad_horn.pipeline_additive
            } else {
                &quad_horn.pipeline_alpha
            }
        }
        PipelineKind::Sphere => match bucket {
            BlendBucket::AlphaNoDepth => &sphere.pipeline_alpha_no_depth,
            BlendBucket::AdditiveNoDepth => &sphere.pipeline_additive_no_depth,
            _ if additive => &sphere.pipeline_additive,
            _ => &sphere.pipeline_alpha,
        },
        PipelineKind::WorldQuad => match bucket {
            BlendBucket::Alpha => &world_quad.pipeline_alpha,
            BlendBucket::AlphaNoDepth => &world_quad.pipeline_alpha_no_depth,
            BlendBucket::AdditiveNoDepth => &world_quad.pipeline_additive_no_depth,
            BlendBucket::Additive | BlendBucket::Multiply => &world_quad.pipeline_additive,
        },
        PipelineKind::Texture3D => {
            if additive {
                &texture3d.pipeline_additive
            } else {
                &texture3d.pipeline_alpha
            }
        }
        PipelineKind::RadialRing => {
            if additive {
                &radial_ring.pipeline_additive
            } else {
                &radial_ring.pipeline_alpha
            }
        }
        PipelineKind::LineStrip => {
            if additive {
                &line_strip.pipeline_additive
            } else {
                &line_strip.pipeline_alpha
            }
        }
        PipelineKind::Sprite => match bucket {
            BlendBucket::Alpha => &sprite_renderer.pipeline,
            BlendBucket::AlphaNoDepth => &sprite_renderer.pipeline_overlay,
            BlendBucket::Additive => &sprite_renderer.pipeline_additive,
            BlendBucket::AdditiveNoDepth => &sprite_renderer.pipeline_additive_overlay,
            BlendBucket::Multiply => &sprite_renderer.pipeline,
        },
        PipelineKind::FullscreenOverlay => {
            if additive {
                &fullscreen.pipeline_additive
            } else {
                &fullscreen.pipeline_alpha
            }
        }
    }
}
