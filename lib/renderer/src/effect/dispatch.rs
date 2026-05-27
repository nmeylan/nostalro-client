//! Unified effect-primitive dispatch.
//!
//! Walks the deferred [`DrawRecord`] list — produced by the per-pipeline
//! `prepare_*_records` helpers — and emits a single `wgpu::RenderPass`
//! that draws every effect primitive in bucket-then-depth order. Every
//! primitive lives in one of a small set of per-blend-flag deferred
//! lists, sorted back-to-front, then flushed alpha-before-emissive — the
//! same batched ordering the original game shows on screen.
//!
//! All effect primitives share [`crate::sprite::SpriteVertex`] as their
//! vertex layout, so the dispatcher uses one GPU vertex / index buffer for
//! every kind. Pipeline switching at draw time honours the per-record
//! [`PipelineKind`]; group-0 binding switches between the camera uniform
//! (3D pipelines) and the sprite uniform (camera-facing 2D pipelines).

use wgpu::util::DeviceExt;

use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, partition_and_sort};
use crate::effect::primitives::{
    CylinderRenderer, FrustumRenderer, GroundDiscRenderer, LineStripRenderer, QuadHornRenderer,
    RadialRingRenderer, SphereRenderer, Texture3DRenderer, WorldQuadRenderer,
};
use crate::sprite::{SpriteRenderer, SpriteVertex};

const INITIAL_VERTEX_CAPACITY: usize = 4096;
const INITIAL_INDEX_CAPACITY: usize = 8192;

/// Owns the unified vertex / index buffer that every effect primitive
/// writes into each frame, plus the dispatch loop itself.
///
/// Pipelines live on the existing per-primitive renderer structs
/// ([`FrustumRenderer`], [`GroundDiscRenderer`], …, [`SpriteRenderer`]);
/// the dispatcher just picks the right one for each `(kind × bucket)`
/// combination at draw time.
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

    /// One bound view onto the per-primitive renderer pipelines.
    ///
    /// Borrows are split per-renderer rather than handing in `&Renderer`
    /// so the caller (the top-level frame builder) can still keep its own
    /// `&mut` borrows on the other parts of the renderer.
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
    ) {
        if records.is_empty() {
            return;
        }

        // Partition + sort: per-bucket index lists into `records`.
        let buckets = partition_and_sort(&records);
        let total_active: usize = buckets.iter().map(|b| b.len()).sum();
        if total_active == 0 {
            return;
        }

        // Concatenate every record's vertex / index data into one upload,
        // rebasing indices. Build a parallel `Span` list that knows which
        // pipeline + texture to bind for each record's draw range.
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
        // Spans are filled out in dispatch order so they can be iterated
        // sequentially below — one entry per record in flush order.
        let mut spans: Vec<Span<'tex>> = Vec::with_capacity(total_active);

        // First pass: walk all records in the *original* emission order
        // to lay out vertex / index data. Spans get written later in
        // sorted order via a per-record-index lookup table.
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

        // Second pass: iterate buckets in flush order and emit spans in
        // sorted order.
        for bucket in BlendBucket::FLUSH_ORDER {
            let list = &buckets[bucket.flush_index()];
            for &record_idx in list {
                let r = &records[record_idx];
                let range = &ranges[record_idx];
                let _ = range.vertex_offset; // already baked into all_indices
                spans.push(Span {
                    kind: r.pipeline,
                    bucket,
                    texture: r.texture,
                    index_start: range.index_start,
                    index_count: range.index_count,
                });
            }
        }

        // Grow buffers if needed; reupload contents.
        if all_verts.len() > self.vertex_capacity {
            self.vertex_capacity = all_verts.len().next_power_of_two();
            self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("effect_dispatch_vertices"),
                contents: bytemuck::cast_slice(&all_verts),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        } else {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_verts));
        }
        if all_indices.len() > self.index_capacity {
            self.index_capacity = all_indices.len().next_power_of_two();
            self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("effect_dispatch_indices"),
                contents: bytemuck::cast_slice(&all_indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
        } else {
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_indices));
        }

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

        // Track current state so we can skip redundant bind / set_pipeline
        // calls when consecutive spans share state.
        let mut current_kind: Option<PipelineKind> = None;
        let mut current_bucket: Option<BlendBucket> = None;
        let mut current_group0_kind: Option<PipelineKind> = None;

        for span in spans {
            // Bind group 0 swap when crossing the 3D vs Sprite boundary.
            // 3D pipelines (Frustum, GroundDisc, QuadHorn, Sphere,
            // WorldQuad) share the same camera bind group layout, so we
            // only re-set when we move into or out of the Sprite kind.
            let group0_kind = match span.kind {
                PipelineKind::Sprite => PipelineKind::Sprite,
                // 3D pipelines (Frustum, Cylinder, GroundDisc, QuadHorn,
                // Sphere, WorldQuad) all bind the camera UBO at group 0;
                // pick any 3D variant as the representative.
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

            // Pipeline swap.
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
) -> &'a wgpu::RenderPipeline {
    // No-depth buckets currently fall through to their depth-read sibling
    // because no callers emit them yet; once an `AlphaNoDepth` user lands
    // we add dedicated depth-disabled pipelines here.
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
        PipelineKind::GroundDisc => {
            if additive {
                &ground_disc.pipeline_additive
            } else {
                &ground_disc.pipeline_alpha
            }
        }
        PipelineKind::QuadHorn => {
            if additive {
                &quad_horn.pipeline_additive
            } else {
                &quad_horn.pipeline_alpha
            }
        }
        PipelineKind::Sphere => {
            if additive {
                &sphere.pipeline_additive
            } else {
                &sphere.pipeline_alpha
            }
        }
        PipelineKind::WorldQuad => {
            if additive {
                &world_quad.pipeline_additive
            } else {
                &world_quad.pipeline_alpha
            }
        }
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
            BlendBucket::AlphaNoDepth => &sprite_renderer.pipeline_no_depth,
            BlendBucket::Additive => &sprite_renderer.pipeline_additive,
            BlendBucket::AdditiveNoDepth => &sprite_renderer.pipeline_additive_no_depth,
            BlendBucket::Multiply => &sprite_renderer.pipeline,
        },
    }
}
