use crate::camera::Camera;
use crate::effect::BlendKind;
use crate::sprite::SpriteVertex;

pub fn view_z(camera: &Camera, pos: [f32; 3]) -> f32 {
    let p = camera.view_matrix() * glam::Vec4::new(pos[0], pos[1], pos[2], 1.0);
    p.z
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlendBucket {
    Alpha,
    AlphaNoDepth,
    Additive,
    AdditiveNoDepth,
    Multiply,
}

impl BlendBucket {
    pub const FLUSH_ORDER: [BlendBucket; 5] = [
        BlendBucket::Alpha,
        BlendBucket::AlphaNoDepth,
        BlendBucket::Additive,
        BlendBucket::AdditiveNoDepth,
        BlendBucket::Multiply,
    ];

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

    pub fn flush_index(self) -> usize {
        Self::FLUSH_ORDER
            .iter()
            .position(|b| *b == self)
            .expect("BlendBucket missing from FLUSH_ORDER")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PipelineKind {
    Frustum,
    Cylinder,
    GroundDisc,
    QuadHorn,
    Sphere,
    WorldQuad,
    Texture3D,
    RadialRing,
    LineStrip,
    Sprite,
    FullscreenOverlay,
}

pub struct DrawRecord<'tex> {
    pub depth: f32,
    pub emission_index: u32,
    pub blend: BlendBucket,
    pub pipeline: PipelineKind,
    pub vertices: Vec<SpriteVertex>,
    pub indices: Vec<u32>,
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
        unimplemented!()
    }

    fn record_with(depth: f32, emission: u32, blend: BlendBucket) -> DrawRecord<'static> {
        // SAFETY: texture is never dereferenced in these tests.
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
        assert_eq!(buckets[alpha_idx], vec![1, 3, 4, 0]);
        let add_idx = BlendBucket::Additive.flush_index();
        assert_eq!(buckets[add_idx], vec![2]);
        let _ = dummy_bind_group;
    }

    #[test]
    fn billboard_and_sprite_particle_interleave_by_depth() {
        let records = vec![
            record_with(-10.0, 0, BlendBucket::Alpha),
            record_with(-5.0, 1, BlendBucket::Alpha),
            record_with(-20.0, 2, BlendBucket::Alpha),
        ];
        let buckets = partition_and_sort(&records);
        let alpha_idx = BlendBucket::Alpha.flush_index();
        assert_eq!(buckets[alpha_idx], vec![2, 0, 1]);
    }
}
