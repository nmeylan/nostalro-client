use crate::App;
use ragnarok_formats::act::{MotionType, SpriteActionType};
use ragnarok_game::ailment;
use ragnarok_game::entity::{
    DEATH_FADE_DURATION, EntityFade, EntityState, EntityType, ForcedAnimation,
};

impl App {
    pub(crate) fn update_entity_state(&mut self, delta: f32) {
        for entity in self.game.entities.iter_mut() {
            entity.update_state(delta);
            if let Some(move_dir) = entity.movement.movement_direction() {
                entity.direction = move_dir;
            }
        }
    }

    pub(crate) fn update_sprite_animation(&mut self, delta: f32) {
        let camera_dir = self.renderer.as_ref().map(|r| r.camera.direction_index());
        let sprites = &self.game.sprites;
        for entity in self.game.entities.iter_mut() {
            if let Some(sprite) = sprites.get(&entity.id) {
                if entity.state == EntityState::Dead && entity.animation.is_finished() {
                    continue;
                }
                // Freeze / fully-petrified: hold the current pose by skipping the
                // frame advance (the original's motion freeze). Stun/Sleep/petrify-
                // delay keep animating.
                if ailment::ailment_visual(entity.body_state, entity.health_state).motion_locked {
                    continue;
                }
                let dir = camera_dir.unwrap_or(0);

                // One-shot forced animation from a body effect (Jumpkick): arm
                // it, then play and hold it until the OneShot finishes,
                // suppressing the state-driven selection meanwhile — mirroring
                // the original game's force-state, which reverts to the real
                // state action when done.
                if let Some(ba) = self.effect_holder.take_body_action_for_entity(entity.id) {
                    entity.forced_animation =
                        Some(ForcedAnimation::new(ba.action_index, ba.start_frame, ba.duration_ms));
                }
                if let Some(mut forced) = entity.forced_animation {
                    if !forced.started() {
                        forced.mark_started();
                        entity
                            .animation
                            .play(forced.action, forced.duration_ms, forced.start_frame);
                    }
                    entity.animation.set_direction(entity.direction);
                    entity.animation.update(delta, &sprite.body_act, dir);
                    entity.forced_animation =
                        (!entity.animation.is_finished()).then_some(forced);
                    continue;
                }

                let action = entity.action_index();
                let is_transient = matches!(
                    entity.state,
                    EntityState::Hurt
                        | EntityState::Attacking
                        | EntityState::SkillExec
                        | EntityState::Dead
                        | EntityState::Pickup
                );
                if let Some(duration) = entity.animation_duration.take() {
                    let start_frame = entity.animation_start_frame.take().unwrap_or_else(|| {
                        if entity.state == EntityState::SkillExec {
                            entity.skill_exec_start_frame()
                        } else {
                            0
                        }
                    });
                    entity
                        .animation
                        .play(action, duration * 1000.0, start_frame);
                } else if entity.state == EntityState::Casting {
                    entity.animation.set_action(action, MotionType::Static);
                    entity.animation.set_direction(entity.direction);
                    continue;
                } else if is_transient {
                    entity.animation.set_action(action, MotionType::OneShot);
                } else {
                    entity.animation.set_action(action, MotionType::Loop);
                }
                entity.animation.set_direction(entity.direction);
                let is_composite = entity.entity_type == EntityType::Player;
                let animated = !is_composite
                    || SpriteActionType::from_index(entity.animation.action())
                        .is_none_or(|a| a.is_animated());
                if animated {
                    entity.animation.update(delta, &sprite.body_act, dir);
                }
            }
        }
    }

    pub(crate) fn update_fades(&mut self, delta: f32) {
        for entity in self.game.entities.iter_mut() {
            if entity.state == EntityState::Dead
                && entity.fade.is_none()
                && entity.animation.is_finished()
                && entity.scheduled_hits.is_empty()
                && entity.entity_type != EntityType::Player
            {
                entity.fade = Some(EntityFade {
                    elapsed: 0.0,
                    duration: DEATH_FADE_DURATION,
                });
            }

            if let Some(ref mut fade) = entity.fade {
                fade.elapsed += delta;
            }
        }

        let expired: Vec<u32> = self
            .game
            .entities
            .iter()
            .filter(|e| e.should_remove())
            .map(|e| e.id)
            .collect();
        for gid in expired {
            self.despawn_entity_effects(gid);
            self.game.entities.remove(gid);
            self.game.sprites.remove(&gid);
        }
    }
}
