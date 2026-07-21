use crate::camera::Camera;
use crate::effect::EffectDrawList;
use crate::effect::primitives::{
    CylinderRenderer, FrustumRenderer, FullscreenOverlayRenderer, GroundDiscRenderer,
    LineStripRenderer, QuadHornRenderer, RadialRingRenderer, SphereRenderer, Texture3DRenderer,
    WorldQuadRenderer, prepare_cylinder_records, prepare_frustum_records,
    prepare_ground_disc_records, prepare_line_strip_records, prepare_quad_horn_records,
    prepare_radial_ring_records, prepare_screen_quad_records, prepare_sphere_records,
    prepare_texture3d_records, prepare_world_quad_records,
};
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind};

pub const PRIMITIVE_KIND_COUNT: usize = 11;

pub type TextureLookup<'a, 'tex> = &'a dyn Fn(&str) -> Option<&'tex wgpu::BindGroup>;

pub trait EffectPrimitiveRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>>;

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline;

    fn recreate(
        &mut self,
        _device: &wgpu::Device,
        _surface_format: wgpu::TextureFormat,
        _camera_bgl: &wgpu::BindGroupLayout,
        _texture_bgl: &wgpu::BindGroupLayout,
        _source: &str,
    ) {
    }
}

fn is_additive(bucket: BlendBucket) -> bool {
    matches!(
        bucket,
        BlendBucket::Additive | BlendBucket::AdditiveNoDepth | BlendBucket::Multiply
    )
}

impl EffectPrimitiveRenderer for FrustumRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_frustum_records(list, camera, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        if is_additive(bucket) {
            &self.pipeline_additive
        } else {
            &self.pipeline_alpha
        }
    }

    fn recreate(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bgl: &wgpu::BindGroupLayout,
        texture_bgl: &wgpu::BindGroupLayout,
        source: &str,
    ) {
        self.recreate_pipelines(device, surface_format, camera_bgl, texture_bgl, source);
    }
}

impl EffectPrimitiveRenderer for CylinderRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_cylinder_records(list, camera, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        if is_additive(bucket) {
            &self.pipeline_additive
        } else {
            &self.pipeline_alpha
        }
    }

    fn recreate(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bgl: &wgpu::BindGroupLayout,
        texture_bgl: &wgpu::BindGroupLayout,
        source: &str,
    ) {
        self.recreate_pipelines(device, surface_format, camera_bgl, texture_bgl, source);
    }
}

impl EffectPrimitiveRenderer for GroundDiscRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_ground_disc_records(list, camera, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        match bucket {
            BlendBucket::AlphaNoDepth => &self.pipeline_alpha_no_depth,
            BlendBucket::AdditiveNoDepth => &self.pipeline_additive_no_depth,
            _ if is_additive(bucket) => &self.pipeline_additive,
            _ => &self.pipeline_alpha,
        }
    }

    fn recreate(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bgl: &wgpu::BindGroupLayout,
        texture_bgl: &wgpu::BindGroupLayout,
        source: &str,
    ) {
        self.recreate_pipelines(device, surface_format, camera_bgl, texture_bgl, source);
    }
}

impl EffectPrimitiveRenderer for QuadHornRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_quad_horn_records(list, camera, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        if is_additive(bucket) {
            &self.pipeline_additive
        } else {
            &self.pipeline_alpha
        }
    }

    fn recreate(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bgl: &wgpu::BindGroupLayout,
        texture_bgl: &wgpu::BindGroupLayout,
        source: &str,
    ) {
        self.recreate_pipelines(device, surface_format, camera_bgl, texture_bgl, source);
    }
}

impl EffectPrimitiveRenderer for SphereRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_sphere_records(list, camera, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        match bucket {
            BlendBucket::AlphaNoDepth => &self.pipeline_alpha_no_depth,
            BlendBucket::AdditiveNoDepth => &self.pipeline_additive_no_depth,
            _ if is_additive(bucket) => &self.pipeline_additive,
            _ => &self.pipeline_alpha,
        }
    }

    fn recreate(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bgl: &wgpu::BindGroupLayout,
        texture_bgl: &wgpu::BindGroupLayout,
        source: &str,
    ) {
        self.recreate_pipelines(device, surface_format, camera_bgl, texture_bgl, source);
    }
}

impl EffectPrimitiveRenderer for WorldQuadRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_world_quad_records(list, camera, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        match bucket {
            BlendBucket::Alpha => &self.pipeline_alpha,
            BlendBucket::AlphaNoDepth => &self.pipeline_alpha_no_depth,
            BlendBucket::AdditiveNoDepth => &self.pipeline_additive_no_depth,
            BlendBucket::Additive | BlendBucket::Multiply => &self.pipeline_additive,
        }
    }
}

impl EffectPrimitiveRenderer for Texture3DRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_texture3d_records(list, camera, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        if is_additive(bucket) {
            &self.pipeline_additive
        } else {
            &self.pipeline_alpha
        }
    }
}

impl EffectPrimitiveRenderer for RadialRingRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_radial_ring_records(list, camera, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        if is_additive(bucket) {
            &self.pipeline_additive
        } else {
            &self.pipeline_alpha
        }
    }
}

impl EffectPrimitiveRenderer for LineStripRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_line_strip_records(list, camera, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        if is_additive(bucket) {
            &self.pipeline_additive
        } else {
            &self.pipeline_alpha
        }
    }

    fn recreate(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bgl: &wgpu::BindGroupLayout,
        texture_bgl: &wgpu::BindGroupLayout,
        source: &str,
    ) {
        self.recreate_pipelines(device, surface_format, camera_bgl, texture_bgl, source);
    }
}

impl EffectPrimitiveRenderer for FullscreenOverlayRenderer {
    fn prepare<'tex>(
        &self,
        list: &EffectDrawList,
        _camera: &Camera,
        fallback: &'tex wgpu::BindGroup,
        lookup: TextureLookup<'_, 'tex>,
    ) -> Vec<DrawRecord<'tex>> {
        prepare_screen_quad_records(list, fallback, lookup)
    }

    fn pipeline(&self, bucket: BlendBucket) -> &wgpu::RenderPipeline {
        if is_additive(bucket) {
            &self.pipeline_additive
        } else {
            &self.pipeline_alpha
        }
    }
}

pub struct EffectPrimitiveRegistry {
    slots: [Option<Box<dyn EffectPrimitiveRenderer>>; PRIMITIVE_KIND_COUNT],
}

impl EffectPrimitiveRegistry {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bgl: &wgpu::BindGroupLayout,
        texture_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let mut slots: [Option<Box<dyn EffectPrimitiveRenderer>>; PRIMITIVE_KIND_COUNT] =
            Default::default();
        slots[PipelineKind::Frustum as usize] = Some(Box::new(FrustumRenderer::new(
            device,
            surface_format,
            camera_bgl,
            texture_bgl,
        )));
        slots[PipelineKind::Cylinder as usize] = Some(Box::new(CylinderRenderer::new(
            device,
            surface_format,
            camera_bgl,
            texture_bgl,
        )));
        slots[PipelineKind::GroundDisc as usize] = Some(Box::new(GroundDiscRenderer::new(
            device,
            surface_format,
            camera_bgl,
            texture_bgl,
        )));
        slots[PipelineKind::QuadHorn as usize] = Some(Box::new(QuadHornRenderer::new(
            device,
            surface_format,
            camera_bgl,
            texture_bgl,
        )));
        slots[PipelineKind::Sphere as usize] = Some(Box::new(SphereRenderer::new(
            device,
            surface_format,
            camera_bgl,
            texture_bgl,
        )));
        slots[PipelineKind::WorldQuad as usize] = Some(Box::new(WorldQuadRenderer::new(
            device,
            surface_format,
            camera_bgl,
            texture_bgl,
        )));
        slots[PipelineKind::Texture3D as usize] = Some(Box::new(Texture3DRenderer::new(
            device,
            surface_format,
            camera_bgl,
            texture_bgl,
        )));
        slots[PipelineKind::RadialRing as usize] = Some(Box::new(RadialRingRenderer::new(
            device,
            surface_format,
            camera_bgl,
            texture_bgl,
        )));
        slots[PipelineKind::LineStrip as usize] = Some(Box::new(LineStripRenderer::new(
            device,
            surface_format,
            camera_bgl,
            texture_bgl,
        )));
        slots[PipelineKind::FullscreenOverlay as usize] =
            Some(Box::new(FullscreenOverlayRenderer::new(
                device,
                surface_format,
                camera_bgl,
                texture_bgl,
            )));
        Self { slots }
    }

    pub fn get(&self, kind: PipelineKind) -> Option<&dyn EffectPrimitiveRenderer> {
        self.slots[kind as usize].as_deref()
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn EffectPrimitiveRenderer> {
        self.slots.iter().filter_map(|slot| slot.as_deref())
    }

    pub fn recreate(
        &mut self,
        kind: PipelineKind,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        camera_bgl: &wgpu::BindGroupLayout,
        texture_bgl: &wgpu::BindGroupLayout,
        source: &str,
    ) {
        if let Some(renderer) = self.slots[kind as usize].as_deref_mut() {
            renderer.recreate(device, surface_format, camera_bgl, texture_bgl, source);
        }
    }
}
