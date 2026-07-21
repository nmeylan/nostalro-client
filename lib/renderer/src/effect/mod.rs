pub mod blend;
pub mod dispatch;
pub mod holder;
pub mod pipeline;
pub mod primitives;
pub mod queue;
pub mod registry;
pub mod scene;
pub mod str_pipeline;

pub use blend::d3d_blend_to_wgpu;
pub use dispatch::EffectDispatcher;
pub use holder::{EffectHandle, EffectHolder, ExternalCustomBackend, SpawnOutcome, SpawnStatus};
pub use primitives::{
    CylinderRenderer, FrustumRenderer, FullscreenOverlayRenderer, GroundDiscRenderer,
    LineStripRenderer, QuadHornRenderer, RadialRingRenderer, SphereRenderer, Texture3DRenderer,
    WorldQuadRenderer, prepare_billboard_records, prepare_cylinder_records,
    prepare_frustum_records, prepare_ground_disc_records, prepare_line_strip_records,
    prepare_quad_horn_records, prepare_radial_ring_records, prepare_screen_quad_records,
    prepare_sphere_records, prepare_texture3d_records, prepare_world_quad_records,
};
pub use queue::{BlendBucket, DrawRecord, PipelineKind, partition_and_sort};
pub use registry::{EffectPrimitiveRegistry, EffectPrimitiveRenderer};
pub use ragnarok_effects::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectRenderCtx, EffectStatus, EffectUpdateCtx,
    effect_texture_path,
};
pub use scene::{EffectFrameInputs, EffectFrameOutputs, compose_effect_frame};
pub use str_pipeline::{StrEffectCache, StrEffectEntry, StrEmitterInput, build_str_effect_batches};
