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
use ragnarok_game::effect::{EffectDrawList, EffectPrimitiveDraw, EffectRenderCtx};

pub struct EffectFrameInputs<'cache, 'tmp> {
    pub effect_holder: &'tmp EffectHolder,
    pub effect_sprites: &'cache EffectSpriteCache,
    pub str_effects: &'cache StrEffectCache,
    pub camera: &'tmp Camera,
    pub screen_w: f32,
    pub screen_h: f32,
    pub zoom: f32,
    pub elapsed: f32,
    pub resolve_entity: &'tmp dyn Fn(u32) -> Option<[f32; 3]>,
    pub extra_sprite_particles: &'tmp [EffectPrimitiveDraw],
}

pub struct EffectFrameOutputs<'cache> {
    pub effect_batches: Vec<SpriteBatch<'cache>>,
    pub effect_draws: EffectDrawList,
    pub sprite_particle_records: Vec<DrawRecord<'cache>>,
}

pub fn compose_effect_frame<'cache, 'tmp>(
    input: &EffectFrameInputs<'cache, 'tmp>,
) -> EffectFrameOutputs<'cache> {
    let mut effect_batches: Vec<SpriteBatch<'cache>> = Vec::new();

    let spr_snapshots = input
        .effect_holder
        .collect_spr_emitters(input.resolve_entity);
    let burst_snapshots = input
        .effect_holder
        .collect_spr_burst_emitters(input.resolve_entity);
    let mut holder_spr_inputs: Vec<SpriteEffectEmitter<'_>> = spr_snapshots
        .iter()
        .map(|s| SpriteEffectEmitter::Spr {
            sprite_path: &s.sprite,
            duration_ms: s.duration_ms,
            position: s.position,
            color: s.tint,
            size_scale: s.size_scale,
            anim_speed: s.anim_speed,
            repeat: s.repeat,
            anim_time: s.anim_time,
            action_index: s.action_index,
        })
        .collect();
    holder_spr_inputs.extend(
        burst_snapshots
            .iter()
            .map(|b| SpriteEffectEmitter::Smoke3D {
                sprite_path: &b.sprite,
                alpha_max: b.alpha_max,
                color: [1.0, 1.0, 1.0, 1.0],
                size_scale: b.size_scale,
                anim_speed: b.anim_speed,
                size_shrink: b.size_shrink,
                twinkle: b.twinkle,
                particles: b.particles.clone(),
            }),
    );
    let holder_spr_draws = collect_sprite_effect_draws(
        &holder_spr_inputs,
        input.effect_sprites,
        input.camera,
        input.screen_w,
        input.screen_h,
    );
    effect_batches.extend(build_emitter_batches(&holder_spr_draws));

    let holder_str_snapshots = input
        .effect_holder
        .collect_str_emitters(input.resolve_entity);
    let mut str_inputs: Vec<StrEmitterInput<'_>> = Vec::with_capacity(holder_str_snapshots.len());
    for snap in &holder_str_snapshots {
        str_inputs.push(StrEmitterInput {
            str_name: &snap.name,
            position: snap.position,
            anim_time: snap.anim_time,
            repeat: snap.repeat,
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

    for prim in input.extra_sprite_particles {
        effect_draws.push(prim.clone());
    }

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
            .spawn(
                EffectId::Warp,
                Attach::WorldPos([0.0, 0.0, 0.0]),
                Some(2000),
            )
            .expect("spawn warp");
        holder.update(
            &EffectUpdateCtx {
                delta: 0.1,
                camera_target: None,
                caster_yaw: None,
            },
            &|_| None,
            &|_| None,
        );

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
            resolve_entity: &|_| None,
            extra_sprite_particles: &[],
        });

        assert!(!out.effect_draws.primitives.is_empty());
        assert!(out.effect_batches.is_empty());
        assert!(out.sprite_particle_records.is_empty());
    }
}
