use crate::App;
use ragnarok_formats::act::{ActFile, MotionType, SpriteActionType};
use ragnarok_game::ailment;
use ragnarok_game::entity::{
    DEATH_FADE_DURATION, EntityFade, EntityState, EntityType, ForcedAnimation,
};
use ragnarok_game::gr2_model::{self, Gr2Action};
use ragnarok_game::sound::SoundQueue;

/// Resolve ACT frame sound-events to queued sounds, positional at the actor.
/// Each `.wav`-named frame event (weapon swing, hurt cry, footstep) plays on
/// every crossing.
fn emit_act_events(event_ids: &[i32], body_act: &ActFile, world_pos: Option<[f32; 3]>, queue: &mut SoundQueue) {
    let Some(pos) = world_pos else { return };
    for &id in event_ids {
        let Some(name) = body_act.events.get(id as usize) else {
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".wav") {
            continue;
        }
        queue.world(name.clone(), pos);
    }
}

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
        let gat = self.game.gat.as_ref();
        let map_coords = self.game.map_coords.as_ref();
        let sound_queue = &mut self.sound_queue;
        let world_of = |cx: f32, cy: f32| match (gat, map_coords) {
            (Some(g), Some(c)) => {
                let (wx, _, wz) = c.cell_to_world(cx + 0.5, cy + 0.5);
                Some([wx, g.get_height(cx + 0.5, cy + 0.5), wz])
            }
            _ => None,
        };
        for entity in self.game.entities.iter_mut() {
            if let Some(sprite) = sprites.get(&entity.id) {
                if entity.state == EntityState::Dead
                    && entity.animation.action() == entity.action_index()
                    && entity.animation.is_finished()
                {
                    continue;
                }
                if entity.forced_animation.is_none()
                    && ailment::ailment_visual(entity.body_state, entity.health_state, entity.rooted)
                        .motion_locked
                {
                    continue;
                }
                let dir = camera_dir.unwrap_or(0);

                if let Some(ba) = self.effect_holder.take_body_action_for_entity(entity.id) {
                    entity.forced_animation = Some(ForcedAnimation::new(
                        ba.action_index,
                        ba.start_frame,
                        ba.duration_ms,
                    ));
                }
                if let Some(mut forced) = entity.forced_animation {
                    if !forced.started() {
                        forced.mark_started();
                        if forced.hold {
                            entity.animation.set_action(forced.action, MotionType::Static);
                            entity.animation.set_motion_index(forced.start_frame);
                        } else {
                            entity.animation.play(
                                forced.action,
                                forced.duration_ms,
                                forced.start_frame,
                            );
                        }
                    }
                    entity.animation.set_direction(entity.direction);
                    entity.forced_animation = if forced.hold {
                        Some(forced)
                    } else {
                        entity.animation.update(delta, &sprite.body_act, dir);
                        let action_idx = entity.animation.action_index(&sprite.body_act, dir);
                        let events = entity.animation.crossed_event_ids(&sprite.body_act, action_idx);
                        let (cx, cy) = entity.movement.position();
                        emit_act_events(&events, &sprite.body_act, world_of(cx, cy), sound_queue);
                        (!entity.animation.is_finished()).then_some(forced)
                    };
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
                let is_composite = matches!(
                    entity.entity_type,
                    EntityType::Player | EntityType::Mercenary
                );
                let animated = !is_composite
                    || SpriteActionType::from_index(entity.animation.action())
                        .is_none_or(|a| a.is_animated());
                let (cx, cy) = entity.movement.position();
                if animated {
                    if entity.state == EntityState::Moving {
                        let (lx, ly) = entity.anim_last_pos;
                        let dist = ((cx - lx).powi(2) + (cy - ly).powi(2)).sqrt();
                        entity.animation.update_by_distance(dist, &sprite.body_act, dir);
                    } else {
                        entity.animation.update(delta, &sprite.body_act, dir);
                    }
                    let action_idx = entity.animation.action_index(&sprite.body_act, dir);
                    let events = entity.animation.crossed_event_ids(&sprite.body_act, action_idx);
                    emit_act_events(&events, &sprite.body_act, world_of(cx, cy), sound_queue);
                }
                entity.anim_last_pos = (cx, cy);
            }
        }
    }

    /// Drive GR2 model entities: pick the action from the entity state, place
    /// the model at the entity's cell, upload the skinning palette, and start
    /// the death fade once the dead clip has played through (their sprite
    /// animation never ticks, so `update_fades` alone would never fire).
    pub(crate) fn update_gr2_models(&mut self, elapsed: f32) {
        if self.game.gr2_models.is_empty() {
            return;
        }
        let Some(renderer) = &self.renderer else {
            return;
        };
        let (Some(gat), Some(coords)) = (self.game.gat.as_ref(), self.game.map_coords.as_ref())
        else {
            return;
        };
        let queue = &renderer.device.queue;
        for (gid, instance) in self.game.gr2_models.iter_mut() {
            let Some(entity) = self.game.entities.get_mut(*gid) else {
                continue;
            };
            instance.set_action(Gr2Action::from_state(entity.state), elapsed);

            if entity.state == EntityState::Dead
                && entity.fade.is_none()
                && entity.scheduled_hits.is_empty()
                && instance.action_completed(elapsed)
            {
                entity.fade = Some(EntityFade {
                    elapsed: 0.0,
                    duration: DEATH_FADE_DURATION,
                });
            }

            let Some(model) = renderer.gr2_models.get(gid) else {
                continue;
            };
            let (cx, cy) = entity.movement.position();
            let (wx, _, wz) = coords.cell_to_world(cx + 0.5, cy + 0.5);
            let wy = gat.get_height(cx + 0.5, cy + 0.5);
            // The trailing X rotation stands the Z-up model upright (world up
            // is negative Y).
            let yaw = gr2_model::model_facing_yaw(entity.direction);
            let transform = glam::Mat4::from_translation(glam::Vec3::new(wx, wy, wz))
                * glam::Mat4::from_rotation_y(yaw)
                * glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
            model.set_transform(queue, transform);
            model.set_palette(queue, &instance.skinning_palette(elapsed));
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
            self.remove_gr2_model(gid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act_with_events(events: &[&str]) -> ActFile {
        ActFile {
            version: (2, 4),
            actions: Vec::new(),
            events: events.iter().map(|s| s.to_string()).collect(),
            delays: Vec::new(),
        }
    }

    #[test]
    fn wav_named_frame_events_play_and_non_wav_are_ignored() {
        let act = act_with_events(&["attack_sword.wav", "atk", "player_clothes.wav"]);
        let mut queue = SoundQueue::new();
        emit_act_events(&[0, 1, 2], &act, Some([0.0, 0.0, 0.0]), &mut queue);
        let played: Vec<&str> = queue.pending.iter().map(|r| r.name.as_ref()).collect();
        assert_eq!(played, vec!["attack_sword.wav", "player_clothes.wav"]);
    }
}
