use crate::{App, ClipData};
use ragnarok_game::cursor::{RenderEntry, RenderEntryKind};
use ragnarok_game::shadow::shadow_size;
use ragnarok_renderer::ui_renderer::UiVertex;
use ragnarok_renderer::{SpriteBatch, UiDrawCall, UiTextureRef, build_clip_quad, scale_clip_vertices};

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
            b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal)
        });

        for entry in &unified_list {
            match entry.kind {
                RenderEntryKind::Entity => {
                    if let (Some(sprite), Some(entity)) = (
                        self.game.sprites.get(&entry.id),
                        self.game.entities.get(entry.id),
                    ) {
                        let alpha = entity.alpha();
                        let is_fading = alpha < 1.0;

                        if !is_fading {
                            let shadow_scale = entry.sprite_scale * shadow_size(entity.job);
                            let mut shadow = sprite.build_shadow_batches(
                                entry.screen_anchor, entry.depth, shadow_scale,
                            );
                            sprite_batches.append(&mut shadow);
                        }

                        let mut batches = sprite.build_batches(
                            &entity.animation,
                            Some(entry.camera_dir),
                            entity.head_dir,
                            entry.screen_anchor,
                            entry.depth,
                            entry.sprite_scale,
                            entry.depth_gradient,
                        );
                        if is_fading {
                            for batch in &mut batches {
                                for vertex in &mut batch.vertices {
                                    vertex.color[3] *= alpha;
                                }
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
                                let delay_ms = emo_act.delays
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
                                    let emo_center = [
                                        entry.screen_anchor[0],
                                        entry.screen_anchor[1] - 100.0,
                                    ];
                                    for clip in &motion.clips {
                                        if let Some((vertices, indices, tex_idx)) =
                                            build_clip_quad(clip, emo_tex, emo_center, entry.depth, [0, 0])
                                            && tex_idx < emo_tex.bind_groups.len() {
                                                sprite_batches.push(SpriteBatch {
                                                    vertices,
                                                    indices,
                                                    texture: &emo_tex.bind_groups[tex_idx],
                                                });
                                            }
                                    }
                                }
                            }
                        }
                    }
                }
                RenderEntryKind::FloorItem => {
                    if let Some(floor_item) = self.game.floor_items.get(&entry.id)
                        && let Some((tex, act)) = self.game.floor_item_sprites.get(&entry.id) {
                            let y_offset = if floor_item.is_falling {
                                let t = (elapsed - floor_item.drop_time) * 1000.0 / 24.0;
                                let fall_y = -15.0 + (-0.6 + 0.083 * t as f64) * t as f64;
                                (fall_y.min(0.0) as f32) * entry.sprite_scale
                            } else {
                                0.0
                            };

                            let blink_frame = ((elapsed * 1000.0 / 24.0) as u32) % 92;
                            let blink_active = blink_frame >= 90;

                            let center = [
                                entry.screen_anchor[0],
                                entry.screen_anchor[1] + y_offset,
                            ];

                            if !act.actions.is_empty() {
                                let action = &act.actions[0];
                                let motion_count = action.motions.len();
                                let delay_ms = act.delays
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
                                                &mut vertices, center, entry.sprite_scale, entry.depth_gradient,
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
                && let Some(sprite) = self.game.sprites.get(&player_id) {
                    let idle_anim = ragnarok_formats::act::SpriteAnimationState::new(0);
                    let batches = sprite.build_batches(&idle_anim, None, 0, center, 0.0, 1.0, 0.0);
                    for batch in batches {
                        let idx = inline_textures.len();
                        inline_textures.push(batch.texture);
                        paperdoll_calls.push(UiDrawCall {
                            vertices: batch.vertices.iter().map(|sv| UiVertex {
                                position: [sv.position[0], sv.position[1]],
                                tex_coord: sv.tex_coord,
                                color: sv.color,
                            }).collect(),
                            indices: batch.indices,
                            texture: UiTextureRef::Inline(idx),
                        });
                    }
                }

        {
            use ragnarok_game::damage_number::{DamageNumberRenderEntry, build_damage_number_quads};
            let entries: Vec<DamageNumberRenderEntry> = self.game.damage_numbers.numbers.iter_mut()
                .filter_map(|dmg| {
                    let (screen_x, screen_y, scale) = if let Some(entry) = render_list.iter().find(|e| e.id == dmg.entity_id) {
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
            if let (Some(num_tex), Some(num_act)) = (&self.game.damage_number_textures, &self.game.damage_number_act) {
                let quads = build_damage_number_quads(
                    &entries,
                    num_act,
                    &num_tex.sizes,
                    num_tex.indexed_count,
                    self.game.damage_msg_textures.as_ref().map(|t| t.sizes.as_slice()),
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
                });
            }
            for (vertices, indices, tex_idx) in cursor_clips {
                cursor_batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &cursor_tex.bind_groups[tex_idx],
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
            renderer.render(
                &all_ui_calls,
                &sprite_batches,
                &cursor_batches,
                &inline_textures,
                elapsed,
            );
        }
    }
}
