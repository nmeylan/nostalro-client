use crate::{App, ClipData};
use ragnarok_game::ailment;
use ragnarok_game::cursor::{RenderEntry, RenderEntryKind};
use ragnarok_game::effect::{BlendKind, EffectPrimitiveDraw};
use ragnarok_game::entity::EntityState;
use ragnarok_game::shadow::shadow_size;
use ragnarok_game::sprite_path::{HIDDEN_BODY_ALPHA, is_hidden};
use ragnarok_renderer::effect::holder::AfterimageSnapshot;
use ragnarok_renderer::effect::{EffectFrameInputs, compose_effect_frame};
use ragnarok_renderer::ui_renderer::UiVertex;
use ragnarok_renderer::{
    SpriteBatch, UiDrawCall, UiTextureRef, build_clip_quad, scale_clip_vertices,
};

impl App {
    pub(crate) fn compose_and_render(
        &mut self,
        render_list: &[RenderEntry],
        floor_item_render_list: &[RenderEntry],
        elapsed: f32,
        cursor_clips: Vec<ClipData>,
        lock_cursor_clips: Vec<ClipData>,
        mut world_overlay_calls: Vec<UiDrawCall>,
        skill_level_calls: Vec<UiDrawCall>,
        ui_draw_calls: Vec<UiDrawCall>,
    ) {
        let mut sprite_batches: Vec<SpriteBatch> = Vec::new();
        let mut cursor_batches: Vec<SpriteBatch> = Vec::new();

        let mut unified_list: Vec<&RenderEntry> = render_list
            .iter()
            .chain(floor_item_render_list.iter())
            .collect();
        unified_list.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for entry in &unified_list {
            match entry.kind {
                RenderEntryKind::Entity => {
                    if let (Some(sprite), Some(entity)) = (
                        self.game.sprites.get(&entry.id),
                        self.game.entities.get(entry.id),
                    ) {
                        // Hiding / Cloaking / Chase Walk: the body stays visible
                        // but translucent (alpha ~135/255) and loses its
                        // shadow. Folds into the death /
                        // vanish fade alpha.
                        let hidden = is_hidden(entity.effect_state);
                        let fade_alpha = entity.alpha();
                        let body_alpha =
                            fade_alpha * if hidden { HIDDEN_BODY_ALPHA } else { 1.0 };
                        let is_fading = fade_alpha < 1.0;

                        // All per-entity body modifiers (shake / tint / scale /
                        // yaw / spin / lift / copies — the original game's body
                        // light effects) resolved in one call; the shared composer
                        // applies them so the scene and effect viewer never
                        // drift. Fold the hidden / death fade into the alpha.
                        let mut body_channels =
                            self.effect_holder.body_channels_for_entity(entry.id);
                        // A status ailment holds a fixed body ARGB that overrides
                        // any buff tint (the original's `m_isSprArgbFixed`).
                        if let Some(rgb) =
                            ailment::ailment_visual(entity.body_state, entity.health_state).tint
                        {
                            body_channels.tint = Some(rgb);
                        }
                        body_channels.alpha *= body_alpha;

                        if !is_fading && !hidden {
                            let shadow_scale = entry.sprite_scale * shadow_size(entity.job);
                            let mut shadow = sprite.build_shadow_batches(
                                entry.screen_anchor,
                                entry.depth,
                                shadow_scale,
                            );
                            sprite_batches.append(&mut shadow);
                        }

                        let mut batches = ragnarok_renderer::compose_actor_batches(
                            sprite,
                            &entity.animation,
                            entry.camera_dir,
                            entity.head_dir,
                            entry.screen_anchor,
                            entry.depth,
                            entry.sprite_scale,
                            &body_channels,
                        );

                        // Quicken afterimage: drop fading sprite copies behind
                        // the actor while it moves, attacks, or casts a skill
                        // (the speed-buff blur is iconic on the fast attacks, not
                        // just walking). Drop one copy each time the displayed
                        // animation frame changes, so the trail is a sequence of
                        // distinct past poses rather than duplicates of the
                        // current one. Copies draw *before* the sprite so the
                        // live one stays on top.
                        if let Some(ai) = self.effect_holder.afterimage_params_for_entity(entry.id) {
                            let trailing = entity.state == EntityState::Moving;
                            let action = entity.animation.action();
                            let motion = entity.animation.motion_index();
                            let last = self
                                .effect_holder
                                .afterimages_for_entity(entry.id)
                                .last()
                                .map(|i| (i.anim.action(), i.anim.motion_index()));
                            // Frames to drop this render. A fast (Quicken-boosted)
                            // attack can advance several frames in one render, so
                            // fill every frame the animation passed through — the
                            // swing arc would otherwise have gaps. Same-frame ⇒
                            // nothing; a new action / looped-back frame ⇒ just the
                            // current one.
                            let frames: Vec<usize> = match last {
                                Some((a, m)) if a == action && motion > m => (m + 1..=motion).collect(),
                                Some((a, m)) if a == action && motion == m => Vec::new(),
                                _ => vec![motion],
                            };
                            if trailing {
                                for frame in frames {
                                    let mut anim = entity.animation.clone();
                                    anim.set_motion_index(frame);
                                    self.effect_holder.push_afterimage(AfterimageSnapshot::new(
                                        entry.id,
                                        anim,
                                        Some(entry.camera_dir),
                                        entity.head_dir,
                                        entity.movement.position(),
                                        entry.screen_anchor,
                                        entry.depth,
                                        entry.sprite_scale,
                                        &ai,
                                    ));
                                }
                            }
                            for img in self.effect_holder.afterimages_for_entity(entry.id) {
                                // Re-project the frozen world position with the
                                // live camera so the copy stays where the actor
                                // was and trails behind it; a frozen screen
                                // anchor would follow the camera and clump on the
                                // actor instead.
                                let (anchor, depth, scale) = self
                                    .renderer
                                    .as_ref()
                                    .zip(self.game.map_coords.as_ref())
                                    .and_then(|(r, coords)| {
                                        let sw = r.device.surface_config.width as f32
                                            / r.dpi_scale;
                                        let sh = r.device.surface_config.height as f32
                                            / r.dpi_scale;
                                        crate::input::entity_screen_params(
                                            img.world_pos,
                                            self.game.gat.as_ref(),
                                            coords,
                                            &r.camera,
                                            sw,
                                            sh,
                                        )
                                    })
                                    .map(|(a, d, _cd, s, _dg)| (a, d, s))
                                    .unwrap_or((entry.screen_anchor, entry.depth, entry.sprite_scale));
                                let mut copy = sprite.build_batches(
                                    &img.anim,
                                    img.camera_dir,
                                    img.head_dir,
                                    anchor,
                                    depth,
                                    scale,
                                    0.0,
                                );
                                let (tr, tg, tb) = (
                                    img.tint[0] as f32 / 255.0,
                                    img.tint[1] as f32 / 255.0,
                                    img.tint[2] as f32 / 255.0,
                                );
                                for batch in &mut copy {
                                    // Blend additively so the tinted copies read
                                    // as a glowing speed-trail (dark texels add
                                    // nothing), not solid duplicate bodies.
                                    batch.additive = true;
                                    for vertex in &mut batch.vertices {
                                        vertex.color[0] *= tr;
                                        vertex.color[1] *= tg;
                                        vertex.color[2] *= tb;
                                        vertex.color[3] *= img.alpha;
                                    }
                                }
                                sprite_batches.append(&mut copy);
                            }
                        }

                        sprite_batches.append(&mut batches);

                        if let (Some(emo), Some(emo_act), Some(emo_tex)) = (
                            &entity.emotion,
                            &self.game.emotion_act,
                            &self.game.emotion_textures,
                        ) {
                            let action_idx = emo.emotion_type as usize;
                            if action_idx < emo_act.actions.len() {
                                let delay_ms = emo_act
                                    .delays
                                    .get(action_idx)
                                    .map(|d| d * 25.0)
                                    .filter(|d| *d > 0.0)
                                    .unwrap_or(150.0);
                                let motion_count = emo_act.actions[action_idx].motions.len();
                                let motion_idx = if motion_count > 0 {
                                    ((emo.elapsed * 1000.0) / delay_ms) as usize % motion_count
                                } else {
                                    0
                                };
                                if motion_idx < motion_count {
                                    let motion = &emo_act.actions[action_idx].motions[motion_idx];
                                    let emo_center =
                                        [entry.screen_anchor[0], entry.screen_anchor[1] - 100.0];
                                    for clip in &motion.clips {
                                        if let Some((vertices, indices, tex_idx)) = build_clip_quad(
                                            clip,
                                            emo_tex,
                                            emo_center,
                                            entry.depth,
                                            [0, 0],
                                        ) && tex_idx < emo_tex.bind_groups.len()
                                        {
                                            sprite_batches.push(SpriteBatch {
                                                vertices,
                                                indices,
                                                texture: &emo_tex.bind_groups[tex_idx],
                                                additive: false,
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        // Status-ailment overlays (stun stars / sleep Z's / curse
                        // mark / angelus halo): persistent head-anchored billboards
                        // while the ailment holds. Derived from the entity's
                        // body/health state, so no spawn/despawn bookkeeping.
                        for overlay in
                            ailment::ailment_overlays(entity.body_state, entity.health_state)
                        {
                            let Some((tex, act)) = self.game.status_overlay_sprites.get(&overlay)
                            else {
                                continue;
                            };
                            let action_idx = overlay.sprite().1;
                            if action_idx >= act.actions.len() {
                                continue;
                            }
                            let delay_ms = act
                                .delays
                                .get(action_idx)
                                .map(|d| d * 25.0)
                                .filter(|d| *d > 0.0)
                                .unwrap_or(100.0);
                            let motion_count = act.actions[action_idx].motions.len();
                            if motion_count == 0 {
                                continue;
                            }
                            let motion_idx =
                                ((elapsed * 1000.0) / delay_ms) as usize % motion_count;
                            let motion = &act.actions[action_idx].motions[motion_idx];
                            let center =
                                [entry.screen_anchor[0], entry.screen_anchor[1] - 100.0];
                            for clip in &motion.clips {
                                if let Some((vertices, indices, tex_idx)) =
                                    build_clip_quad(clip, tex, center, entry.depth, [0, 0])
                                    && tex_idx < tex.bind_groups.len()
                                {
                                    sprite_batches.push(SpriteBatch {
                                        vertices,
                                        indices,
                                        texture: &tex.bind_groups[tex_idx],
                                        additive: false,
                                    });
                                }
                            }
                        }
                    }
                }
                RenderEntryKind::FloorItem => {
                    if let Some(floor_item) = self.game.floor_items.get(&entry.id)
                        && let Some((tex, act)) = self.game.floor_item_sprites.get(&entry.id)
                    {
                        let y_offset = if floor_item.is_falling {
                            let t = (elapsed - floor_item.drop_time) * 1000.0 / 24.0;
                            let fall_y = -15.0 + (-0.6 + 0.083 * t as f64) * t as f64;
                            (fall_y.min(0.0) as f32) * entry.sprite_scale
                        } else {
                            0.0
                        };

                        let blink_frame = ((elapsed * 1000.0 / 24.0) as u32) % 92;
                        let blink_active = blink_frame >= 90;

                        let center = [entry.screen_anchor[0], entry.screen_anchor[1] + y_offset];

                        if !act.actions.is_empty() {
                            let action = &act.actions[0];
                            let motion_count = action.motions.len();
                            let delay_ms = act
                                .delays
                                .first()
                                .map(|d| d * 25.0)
                                .filter(|d| *d > 0.0)
                                .unwrap_or(150.0);
                            let item_elapsed = elapsed - floor_item.drop_time;
                            let motion_idx = if motion_count > 0 {
                                ((item_elapsed * 1000.0) / delay_ms) as usize % motion_count
                            } else {
                                0
                            };
                            if motion_idx < motion_count {
                                let motion = &action.motions[motion_idx];
                                for clip in &motion.clips {
                                    if let Some((mut vertices, indices, tex_idx)) =
                                        build_clip_quad(clip, tex, center, entry.depth, [0, 0])
                                    {
                                        scale_clip_vertices(
                                            &mut vertices,
                                            center,
                                            entry.sprite_scale,
                                            0.0,
                                        );
                                        if blink_active {
                                            for v in &mut vertices {
                                                v.color = [1.0, 1.0, 1.0, 1.0];
                                            }
                                        }
                                        if tex_idx < tex.bind_groups.len() {
                                            sprite_batches.push(SpriteBatch {
                                                vertices,
                                                indices,
                                                texture: &tex.bind_groups[tex_idx],
                                                additive: false,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut inline_textures = Vec::new();
        let mut paperdoll_calls: Vec<UiDrawCall> = Vec::new();
        if let Some(center) = self.game.equipment_window.character_center()
            && let Some(player_id) = self.game.entities.player_id()
            && let Some(sprite) = self.game.sprites.get(&player_id)
        {
            let idle_anim = ragnarok_formats::act::SpriteAnimationState::new(0);
            let batches = sprite.build_batches(&idle_anim, None, 0, center, 0.0, 1.0, 0.0);
            for batch in batches {
                let idx = inline_textures.len();
                inline_textures.push(batch.texture);
                paperdoll_calls.push(UiDrawCall {
                    vertices: batch
                        .vertices
                        .iter()
                        .map(|sv| UiVertex {
                            position: [sv.position[0], sv.position[1]],
                            tex_coord: sv.tex_coord,
                            color: sv.color,
                        })
                        .collect(),
                    indices: batch.indices,
                    texture: UiTextureRef::Inline(idx),
                });
            }
        }

        {
            use ragnarok_game::damage_number::{
                DamageNumberRenderEntry, build_damage_number_quads,
            };
            let entries: Vec<DamageNumberRenderEntry> = self
                .game
                .damage_numbers
                .numbers
                .iter_mut()
                .filter_map(|dmg| {
                    let (screen_x, screen_y, scale) =
                        if let Some(entry) = render_list.iter().find(|e| e.id == dmg.entity_id) {
                            let pos = (
                                entry.screen_anchor[0],
                                entry.screen_anchor[1] - entry.head_offset,
                                entry.sprite_scale,
                            );
                            dmg.last_screen_pos = Some(pos);
                            pos
                        } else {
                            dmg.last_screen_pos?
                        };
                    let data = dmg.render_data()?;
                    Some(DamageNumberRenderEntry {
                        entity_id: dmg.entity_id,
                        screen_x,
                        screen_y,
                        scale,
                        data,
                    })
                })
                .collect();
            if let (Some(num_tex), Some(num_act)) = (
                &self.game.damage_number_textures,
                &self.game.damage_number_act,
            ) {
                let quads = build_damage_number_quads(
                    &entries,
                    num_act,
                    &num_tex.sizes,
                    num_tex.indexed_count,
                    self.game
                        .damage_msg_textures
                        .as_ref()
                        .map(|t| t.sizes.as_slice()),
                );
                ragnarok_renderer::render_damage_number_quads(
                    &quads,
                    num_tex,
                    self.game.damage_msg_textures.as_ref(),
                    &mut world_overlay_calls,
                    &mut inline_textures,
                );
            }
        }

        if let Some(cursor_tex) = &self.game.cursor_textures {
            for (vertices, indices, tex_idx) in lock_cursor_clips {
                cursor_batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &cursor_tex.bind_groups[tex_idx],
                    additive: false,
                });
            }
            for (vertices, indices, tex_idx) in cursor_clips {
                cursor_batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &cursor_tex.bind_groups[tex_idx],
                    additive: false,
                });
            }
        }

        let mut all_ui_calls = world_overlay_calls;
        let overlay_len = all_ui_calls.len();
        all_ui_calls.extend(ui_draw_calls);

        if let Some(insert_idx) = self.game.equipment_window.paperdoll_insert_index() {
            let abs_idx = (overlay_len + insert_idx).min(all_ui_calls.len());
            for (i, dc) in paperdoll_calls.into_iter().enumerate() {
                all_ui_calls.insert(abs_idx + i, dc);
            }
        }

        all_ui_calls.extend(skill_level_calls);

        if let Some(renderer) = &mut self.renderer {
            let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
            let screen_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            let arrow_draws: Vec<EffectPrimitiveDraw> = self
                .game
                .arrows
                .iter()
                .filter(|a| a.is_visible())
                .map(|a| EffectPrimitiveDraw::SpriteParticle {
                    sprite_path: a.sprite_path(),
                    position: a.current_position(),
                    action_index: 0,
                    motion_index: 0,
                    size_scale: 1.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                    blend: BlendKind::Alpha,
                    aim_target: Some(a.target_pos()),
                    no_depth: false,
                })
                .collect();
            let zoom = self
                .game
                .map_coords
                .as_ref()
                .map_or(10.0, |c| c.zoom());
            // Live entity→world resolver for entity-anchored STR/Spr effects
            // (cast glyphs, buff overlays). Mirrors the holder's update-time
            // resolver: interpolated cell → world at the ground. Without it
            // these effects can't resolve their position and never draw.
            let entities = &self.game.entities;
            let gat = self.game.gat.as_ref();
            let map_coords = self.game.map_coords.as_ref();
            let resolve_entity = |id: u32| {
                let (gat, coords) = (gat?, map_coords?);
                let (cx, cy) = entities.get(id)?.movement.position();
                let (wx, _, wz) = coords.cell_to_world(cx + 0.5, cy + 0.5);
                Some([wx, gat.get_height(cx + 0.5, cy + 0.5), wz])
            };
            let frame = compose_effect_frame(&EffectFrameInputs {
                effect_holder: &self.effect_holder,
                effect_sprites: &self.effect_sprites,
                str_effects: &self.str_effects,
                camera: &renderer.camera,
                screen_w,
                screen_h,
                zoom,
                elapsed,
                resolve_entity: &resolve_entity,
                extra_sprite_particles: &arrow_draws,
            });

            let custom = self.effect_holder.custom_count();
            if custom > 0 {
                let frustums = frame
                    .effect_draws
                    .primitives
                    .iter()
                    .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
                    .count();
                let billboards = frame
                    .effect_draws
                    .primitives
                    .iter()
                    .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { .. }))
                    .count();
            }

            renderer.render(
                &all_ui_calls,
                &frame.effect_batches,
                &frame.effect_draws,
                frame.sprite_particle_records,
                &sprite_batches,
                &cursor_batches,
                &inline_textures,
                elapsed,
            );
        }
    }
}

