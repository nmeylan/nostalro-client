//! Shared effect-frame composer used by both the game client and `tools/viewer`.
//!
//! Collects sprite-emitter draws, STR snapshots, and custom primitive draws
//! from an `EffectHolder` plus any caller-supplied extras (the client's RSW
//! ambient emitters live in `EffectManager`, not `EffectHolder`, so they
//! arrive through `extra_*_emitters`). Returns the resulting sprite batches
//! and primitive draw list ready to hand to `Renderer::render`.

use ragnarok_game::effect::CameraView;

use crate::camera::Camera;
use crate::effect::holder::EffectHolder;
use crate::effect::queue::DrawRecord;
use crate::effect::str_pipeline::{StrEffectCache, StrEmitterInput, build_str_effect_batches};
use crate::effect_sprite::{
    EffectSpriteCache, SpriteEffectEmitter, build_emitter_batches, collect_sprite_effect_draws,
    prepare_sprite_particle_records,
};
use crate::sprite::SpriteBatch;
use ragnarok_game::effect::{EffectDrawList, EffectRenderCtx};

/// `'cache` ties together the borrows that survive into the output
/// (`effect_sprites`, `str_effects`). `'tmp` covers borrows that are only
/// needed during the call (the camera, the effect holder, the extras).
/// Splitting these lifetimes lets the caller drop the camera/holder borrow
/// as soon as the function returns, freeing up `&mut renderer` for the
/// subsequent render call.
pub struct EffectFrameInputs<'cache, 'tmp> {
    pub effect_holder: &'tmp EffectHolder,
    pub effect_sprites: &'cache EffectSpriteCache,
    pub str_effects: &'cache StrEffectCache,
    pub camera: &'tmp Camera,
    pub screen_w: f32,
    pub screen_h: f32,
    pub zoom: f32,
    pub elapsed: f32,
    /// Caller-owned SPR/Smoke3D emitters (currently the client's RSW
    /// ambient emitters from `EffectManager`). Viewer passes `&[]`.
    pub extra_spr_emitters: &'tmp [SpriteEffectEmitter<'tmp>],
    /// Caller-owned STR emitters (RSW STR ambient effects). Viewer passes `&[]`.
    pub extra_str_emitters: &'tmp [StrEmitterInput<'tmp>],
}

pub struct EffectFrameOutputs<'cache> {
    /// Batches that stay outside the unified effect queue: STR keyframe
    /// animations and ambient SPR / Smoke3D emitters. They render in their
    /// own dedicated sprite pass.
    pub effect_batches: Vec<SpriteBatch<'cache>>,
    /// Custom-effect primitive draws (Billboard, BillboardDisc,
    /// SpriteParticle, Frustum, GroundDisc, …). Consumed by the unified
    /// `EffectDispatcher` pass inside the renderer.
    pub effect_draws: EffectDrawList,
    /// `SpriteParticle` records pre-projected against the camera. These
    /// reference textures inside the [`EffectSpriteCache`] (which only the
    /// caller borrows), so the renderer can't build them itself.
    pub sprite_particle_records: Vec<DrawRecord<'cache>>,
}

/// Build the per-frame effect sprite batches and custom-primitive draw list.
///
/// Pipeline mirrors what the client and viewer were duplicating inline:
/// 1. Project caller-supplied SPR/Smoke3D emitters into sprite batches.
/// 2. Merge caller STR inputs with `EffectHolder::collect_str_emitters` and
///    project the union into sprite batches.
/// 3. Collect custom-effect primitive draws (Ring/Frustum/...) into
///    `effect_draws`.
/// 4. Project any `SpriteParticle` primitives produced by step 3 and append
///    them to the same sprite batch list.
pub fn compose_effect_frame<'cache, 'tmp>(
    input: &EffectFrameInputs<'cache, 'tmp>,
) -> EffectFrameOutputs<'cache> {
    let mut effect_batches: Vec<SpriteBatch<'cache>> = Vec::new();

    let spr_draws = collect_sprite_effect_draws(
        input.extra_spr_emitters,
        input.effect_sprites,
        input.camera,
        input.screen_w,
        input.screen_h,
    );
    effect_batches.extend(build_emitter_batches(&spr_draws));

    let holder_str_snapshots = input.effect_holder.collect_str_emitters(&|_| None);
    let mut str_inputs: Vec<StrEmitterInput<'_>> =
        Vec::with_capacity(input.extra_str_emitters.len() + holder_str_snapshots.len());
    for emitter in input.extra_str_emitters {
        str_inputs.push(StrEmitterInput {
            str_name: emitter.str_name,
            position: emitter.position,
            anim_time: emitter.anim_time,
        });
    }
    for snap in &holder_str_snapshots {
        str_inputs.push(StrEmitterInput {
            str_name: &snap.name,
            position: snap.position,
            anim_time: snap.anim_time,
        });
    }
    let str_batches = build_str_effect_batches(
        &str_inputs,
        input.str_effects,
        input.camera,
        input.screen_w,
        input.screen_h,
        input.zoom,
    );
    effect_batches.extend(str_batches);

    let mut effect_draws = EffectDrawList::new();
    let render_ctx = EffectRenderCtx {
        camera: CameraView {
            eye: input.camera.eye().to_array(),
            target: input.camera.target.to_array(),
            up: [0.0, -1.0, 0.0],
        },
        screen_w: input.screen_w,
        screen_h: input.screen_h,
        elapsed: input.elapsed,
    };
    input
        .effect_holder
        .collect_custom_draws(&mut effect_draws, &render_ctx);

    // SpriteParticle entries now flow through the unified effect queue so
    // they can depth-sort against Billboard / 3D records. Project them
    // here while the caller's `EffectSpriteCache` is still borrowed.
    let sprite_particle_records = prepare_sprite_particle_records(
        &effect_draws,
        input.effect_sprites,
        input.camera,
        input.screen_w,
        input.screen_h,
    );

    EffectFrameOutputs {
        effect_batches,
        effect_draws,
        sprite_particle_records,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::effect_id::EffectId;
    use ragnarok_game::effect::{Attach, EffectUpdateCtx};

    #[test]
    fn compose_effect_frame_collects_custom_primitive_draws() {
        let mut holder = EffectHolder::new();
        holder
            .spawn(EffectId::Warp, Attach::WorldPos([0.0, 0.0, 0.0]), Some(2000))
            .expect("spawn warp");
        // Warp spawns a ring at age 0, but its outer_radius starts at 0.
        // Tick the effect so the ring grows and `collect_draws` emits.
        holder.update(&EffectUpdateCtx { delta: 0.1, camera_target: None });

        let effect_sprites = EffectSpriteCache::new();
        let str_effects = StrEffectCache::new();
        let camera = Camera::default();

        let out = compose_effect_frame(&EffectFrameInputs {
            effect_holder: &holder,
            effect_sprites: &effect_sprites,
            str_effects: &str_effects,
            camera: &camera,
            screen_w: 800.0,
            screen_h: 600.0,
            zoom: 10.0,
            elapsed: 0.1,
            extra_spr_emitters: &[],
            extra_str_emitters: &[],
        });

        assert!(
            !out.effect_draws.primitives.is_empty(),
            "Warp should emit at least one primitive draw after one update tick"
        );
        // No SPR/STR caches loaded, so the sprite batch list stays empty —
        // the assertion just exercises the path without crashing.
        assert!(out.effect_batches.is_empty());
        // SpriteParticle records require sprites in the cache; none are
        // loaded so the records list is empty too. The field exists and
        // is wired through.
        assert!(out.sprite_particle_records.is_empty());
    }
}
