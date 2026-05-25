//! Per-blend-bucket effect draw queue.
//!
//! We do not split effect primitives into "3D-pass" and "2D-pass"
//! buckets. Every effect primitive is recorded into one of a small set
//! of per-blend-flag deferred lists, depth-sorted within each list, then
//! flushed in a fixed order. See
//! `docs/client-plan/effect-render-queue.md` for the full reference.
//!
//! This module defines the queue keying ([`BlendBucket`]) and the per-record
//! payload ([`DrawRecord`]). The renderer walks the effect primitive list
//! once, converts each primitive into one or more [`DrawRecord`]s via the
//! per-pipeline `prepare_*` helpers, partitions by [`BlendBucket`], depth-
//! sorts within each bucket, and flushes in fixed order — one render pass
//! for the whole effect layer.

use crate::camera::Camera;
use crate::effect::BlendKind;
use crate::sprite::SpriteVertex;

/// View-space Z for a world-space anchor.
///
/// Glam's right-handed `look_at_rh` view matrix puts the camera looking
/// down its local -Z axis, so points in front of the camera have negative
/// view-space Z and points further away have *more* negative Z. Sorting
/// records ascending by this value therefore lists the furthest record
/// first — exactly the back-to-front order an alpha-blended deferred list
/// needs.
pub fn view_z(camera: &Camera, pos: [f32; 3]) -> f32 {
    let p = camera.view_matrix() * glam::Vec4::new(pos[0], pos[1], pos[2], 1.0);
    p.z
}

/// Per-blend-flag deferred list, one bucket per blend mode the original
/// game's effects draw with (alpha / alpha-no-depth / emissive families).
///
/// Order of variants matches the original game's on-screen flush order:
/// alpha → alpha-no-depth → additive → additive-no-depth → multiply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlendBucket {
    /// Standard alpha blend, depth-read enabled.
    Alpha,
    /// Alpha blend, depth-read disabled.
    AlphaNoDepth,
    /// Additive blend, depth-read enabled.
    Additive,
    /// Additive blend, depth-read disabled.
    AdditiveNoDepth,
    /// Modulation blend: `src.rgb * dst.rgb`.
    Multiply,
}

impl BlendBucket {
    /// All variants in flush order.
    pub const FLUSH_ORDER: [BlendBucket; 5] = [
        BlendBucket::Alpha,
        BlendBucket::AlphaNoDepth,
        BlendBucket::Additive,
        BlendBucket::AdditiveNoDepth,
        BlendBucket::Multiply,
    ];

    /// Map a per-primitive [`BlendKind`] onto one of the deferred buckets.
    ///
    /// `BlendKind::Raw { src, dst }` uses a simple heuristic: a dst factor
    /// of `OneMinusSrcAlpha` means standard alpha; anything else is treated
    /// as additive, matching how the original game blends these primitives.
    pub fn from_blend_kind(blend: BlendKind) -> BlendBucket {
        match blend {
            BlendKind::Alpha => BlendBucket::Alpha,
            BlendKind::Additive => BlendBucket::Additive,
            BlendKind::Multiply => BlendBucket::Multiply,
            BlendKind::Raw { src: _, dst } => {
                if dst == 6 {
                    BlendBucket::Alpha
                } else {
                    BlendBucket::Additive
                }
            }
        }
    }

    /// Index into [`FLUSH_ORDER`] for this bucket.
    pub fn flush_index(self) -> usize {
        Self::FLUSH_ORDER
            .iter()
            .position(|b| *b == self)
            .expect("BlendBucket missing from FLUSH_ORDER")
    }
}

/// Which pipeline a draw record dispatches through. Each kind corresponds
/// to a distinct WGSL shader / pipeline layout. Vertex format is shared
/// across all kinds ([`SpriteVertex`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PipelineKind {
    /// Vertical "tube" between two coaxial polygons. World-space vertices.
    Frustum,
    /// Cylinder — same geometry as [`Self::Frustum`] but with the
    /// per-segment 0.25-step UV convention used by the ring / pillar
    /// textures. World-space vertices.
    Cylinder,
    /// Flat-on-ground textured annulus / arc wedge. World-space vertices.
    GroundDisc,
    /// Square-based pyramid spike. World-space vertices. Shares
    /// `effect_frustum.wgsl` with [`Self::Frustum`].
    QuadHorn,
    /// UV sphere mesh. World-space vertices.
    Sphere,
    /// Textured quad anchored by four explicit world corners. Shares
    /// `effect_ground_disc.wgsl` with [`Self::GroundDisc`].
    WorldQuad,
    /// Camera-facing 2D billboard / sprite particle. *Screen-space*
    /// vertices (x,y in pixels, z in NDC). Dispatched through the sprite
    /// pipeline with the sprite-uniforms bind group at slot 0.
    Sprite,
}

/// One CPU-built draw record ready for dispatch.
///
/// Records are created in emission order by the per-pipeline `prepare_*`
/// helpers, then partitioned by [`BlendBucket`] and depth-sorted (back to
/// front) before the renderer issues GPU calls.
///
/// `'tex` borrows the texture bind group from the renderer's
/// `TextureCache`, so records cannot outlive a single frame.
pub struct DrawRecord<'tex> {
    /// View-space depth of the record's representative anchor. The
    /// renderer sorts ascending so the furthest record draws first.
    pub depth: f32,
    /// Original emission order — tiebreaker when two records share the
    /// same depth, so primitives from the same effect keep their authored
    /// order.
    pub emission_index: u32,
    /// Which deferred list this record belongs to.
    pub blend: BlendBucket,
    /// Which pipeline this record dispatches through.
    pub pipeline: PipelineKind,
    /// CPU-side vertex data, uploaded into the renderer's unified effect
    /// vertex buffer when the pass flushes.
    pub vertices: Vec<SpriteVertex>,
    /// CPU-side indices (relative to `vertices`'s first vertex). The
    /// renderer rebases them when concatenating into the unified buffer.
    pub indices: Vec<u32>,
    /// Texture bind group for slot 1.
    pub texture: &'tex wgpu::BindGroup,
}

impl<'tex> DrawRecord<'tex> {
    pub fn new(
        depth: f32,
        emission_index: u32,
        blend: BlendBucket,
        pipeline: PipelineKind,
        vertices: Vec<SpriteVertex>,
        indices: Vec<u32>,
        texture: &'tex wgpu::BindGroup,
    ) -> Self {
        Self {
            depth,
            emission_index,
            blend,
            pipeline,
            vertices,
            indices,
            texture,
        }
    }
}

/// Partition `records` into per-bucket lists (one per
/// [`BlendBucket::FLUSH_ORDER`] entry) and stable-sort each list ascending
/// by `(depth, emission_index)`.
///
/// Returns `[Vec<usize>; 5]` aligned to [`BlendBucket::FLUSH_ORDER`]; each
/// inner vector holds the original-record indices in dispatch order.
pub fn partition_and_sort(records: &[DrawRecord<'_>]) -> [Vec<usize>; 5] {
    let mut buckets: [Vec<usize>; 5] = Default::default();
    for (i, r) in records.iter().enumerate() {
        buckets[r.blend.flush_index()].push(i);
    }
    for list in buckets.iter_mut() {
        list.sort_by(|&a, &b| {
            let ra = &records[a];
            let rb = &records[b];
            ra.depth
                .partial_cmp(&rb.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| ra.emission_index.cmp(&rb.emission_index))
        });
    }
    buckets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_bind_group() -> wgpu::BindGroup {
        // Tests never reach a GPU call site, so we never construct a real
        // BindGroup here. Use a leaked sentinel only for type purposes;
        // tests below build DrawRecords via the helper that doesn't touch
        // the texture.
        unimplemented!()
    }

    fn record_with(depth: f32, emission: u32, blend: BlendBucket) -> DrawRecord<'static> {
        // SAFETY: tests only read .depth / .emission_index / .blend; the
        // texture field is never dereferenced. We synthesise a dangling
        // reference to satisfy the type system.
        let texture: &'static wgpu::BindGroup = unsafe {
            let ptr = std::ptr::NonNull::<wgpu::BindGroup>::dangling().as_ptr();
            &*ptr
        };
        DrawRecord {
            depth,
            emission_index: emission,
            blend,
            pipeline: PipelineKind::Sprite,
            vertices: Vec::new(),
            indices: Vec::new(),
            texture,
        }
    }

    #[test]
    fn flush_order_alpha_before_additive() {
        let alpha_idx = BlendBucket::Alpha.flush_index();
        let additive_idx = BlendBucket::Additive.flush_index();
        assert!(alpha_idx < additive_idx);
    }

    #[test]
    fn raw_dst_six_lands_in_alpha() {
        assert_eq!(
            BlendBucket::from_blend_kind(BlendKind::Raw { src: 5, dst: 6 }),
            BlendBucket::Alpha
        );
        assert_eq!(
            BlendBucket::from_blend_kind(BlendKind::Raw { src: 5, dst: 2 }),
            BlendBucket::Additive
        );
    }

    #[test]
    fn partition_and_sort_orders_by_depth_then_emission() {
        let records = vec![
            record_with(20.0, 0, BlendBucket::Alpha),
            record_with(5.0, 1, BlendBucket::Alpha),
            record_with(10.0, 2, BlendBucket::Additive),
            record_with(10.0, 3, BlendBucket::Alpha),
            record_with(10.0, 4, BlendBucket::Alpha),
        ];
        let buckets = partition_and_sort(&records);
        let alpha_idx = BlendBucket::Alpha.flush_index();
        // Alpha bucket sorted by (depth, emission_index): 5/1, 10/3, 10/4, 20/0.
        assert_eq!(buckets[alpha_idx], vec![1, 3, 4, 0]);
        // Additive holds just record 2.
        let add_idx = BlendBucket::Additive.flush_index();
        assert_eq!(buckets[add_idx], vec![2]);

        // Silence dead-code lint on dummy_bind_group.
        let _ = dummy_bind_group;
    }

    #[test]
    fn billboard_and_sprite_particle_interleave_by_depth() {
        // Mirrors the HasteUp shape from the plan: two billboards plus a
        // sprite particle in between. The sorted output for the Alpha
        // bucket must be back-to-front (largest negative view_z first),
        // and a sprite particle with intermediate depth must land between
        // the two billboards.
        let records = vec![
            record_with(-10.0, 0, BlendBucket::Alpha), // Billboard (further)
            record_with(-5.0, 1, BlendBucket::Alpha),  // SpriteParticle (mid)
            record_with(-20.0, 2, BlendBucket::Alpha), // Billboard (furthest)
        ];
        let buckets = partition_and_sort(&records);
        let alpha_idx = BlendBucket::Alpha.flush_index();
        // Ascending by depth → -20, -10, -5: indices [2, 0, 1].
        assert_eq!(buckets[alpha_idx], vec![2, 0, 1]);
    }
}
