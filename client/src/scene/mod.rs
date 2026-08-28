mod render_list;

use crate::game_state::CastMark;
use crate::game_updates::CREATE_PREVIEW_GID;
use crate::{App, ClipData};
use ragnarok_game::ailment;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::{RenderEntry, RenderEntryKind};
use ragnarok_game::effect::{BlendKind, EffectPrimitiveDraw};
use ragnarok_game::entity::EntityState;
use ragnarok_game::pk_rank::pk_rank_hud_quads;
use ragnarok_game::shadow::shadow_size;
use ragnarok_game::sprite_path::{HiddenRender, HiddenViewer, hidden_render, is_hidden};
use ragnarok_renderer::effect::holder::AfterimageSnapshot;
use ragnarok_renderer::effect::{EffectFrameInputs, compose_effect_frame};
use ragnarok_renderer::ui_renderer::UiVertex;
use ragnarok_renderer::{
    FrameInputs, SpriteBatch, UiDrawCall, UiTextureRef, build_clip_quad, build_clip_quad_scaled,
    scale_clip_vertices,
};

/// Action index of the ice-shatter in `얼음땡.act` (action 0 is the block).
const FREEZE_SHATTER_ACTION: usize = 1;

/// Negative Y is up, so this lifts the graffiti decal clear of the terrain.
const GRAFFITI_GROUND_LIFT: f32 = -0.3;

/// Base widening every shadow blob gets, on top of the per-job factor.
const SHADOW_SCALE: f32 = 1.2;

/// Floor items take a much smaller shadow than actors: `1.2 * 0.4`.
const FLOOR_ITEM_SHADOW_SCALE: f32 = SHADOW_SCALE * 0.4;

impl App {
    /// Stealth-visibility relationship of the local player to `gid`: their own
    /// body, a party member's, or a stranger's.
    pub(crate) fn hidden_viewer_for(&self, gid: u32) -> HiddenViewer {
        if self.game.world.entities.player_id() == Some(gid) {
            return HiddenViewer::Own;
        }
        let is_ally = self
            .game
            .party
            .as_ref()
            .is_some_and(|p| p.members.iter().any(|m| m.aid == gid));
        if is_ally {
            HiddenViewer::Ally
        } else {
            HiddenViewer::Other
        }
    }

    /// maya purple effect
    pub(crate) fn player_clairvoyant(&self) -> bool {
        self.game
            .character
            .has_status(ragnarok_game::sprite_path::EFST_CLAIRVOYANCE)
    }

    /// Ground-lightmap tint for a sprite standing at `cell` (GAT coordinates).
    fn actor_light(&self, cell: (f32, f32)) -> [f32; 3] {
        let lightmap_on = self.renderer.as_ref().is_some_and(|r| r.lightmap_enabled());
        if !lightmap_on {
            return [1.0; 3];
        }
        match &self.game.session.actor_lightmap {
            Some(lm) => lm.intensity_at_pos(cell.0, cell.1),
            None => [1.0; 3],
        }
    }

    pub(crate) fn compose_and_render(
        &mut self,
        render_list: &[RenderEntry],
        floor_item_render_list: &[RenderEntry],
        cart_render_list: &[RenderEntry],
        elapsed: f32,
        delta: f32,
        cursor_clips: Vec<ClipData>,
        lock_cursor_clips: Vec<ClipData>,
        mut world_overlay_calls: Vec<UiDrawCall>,
        skill_level_calls: Vec<UiDrawCall>,
        ui_draw_calls: Vec<UiDrawCall>,
        tooltip_draw_calls: Vec<UiDrawCall>,
    ) {
        ragnarok_profiling::profile_function!();
        let mut sprite_batches: Vec<SpriteBatch> = Vec::new();
        // Shadows lie flat on the terrain and belong under every sprite. World
        // sprites write no depth of their own, so pass order is the only thing
        // that puts them there: these are prepended once the loop is done.
        let mut shadow_batches: Vec<SpriteBatch> = Vec::new();
        // Flat feet-depth body silhouettes, stamped into depth after the colour
        // pass so effects occlude against the body (gradient `[0,0]` => uniform z).
        let mut silhouette_batches: Vec<SpriteBatch> = Vec::new();
        let mut cursor_batches: Vec<SpriteBatch> = Vec::new();
        let camera_distance = self
            .renderer
            .as_ref()
            .map(|r| r.camera.distance)
            .unwrap_or(ragnarok_renderer::camera::DEFAULT_DISTANCE);
        // Body copies track the zoom target, not the eased distance, so a halo
        // reaches its new width the moment the wheel moves.
        let copy_margin_scale = 400.0
            / self
                .renderer
                .as_ref()
                .map(|r| r.camera.dest_distance)
                .unwrap_or(ragnarok_renderer::camera::DEFAULT_DISTANCE)
                .max(1.0);

        // TODO refactor
        if !self.game.world.freeze_shatters.is_empty() {
            let anim = self
                .game
                .assets
                .status_overlay_sprites
                .get(&ailment::AilmentOverlay::Freeze)
                .and_then(|(_, act)| {
                    let motion_count = act.actions.get(FREEZE_SHATTER_ACTION)?.motions.len();
                    let delay_ms = act
                        .delays
                        .get(FREEZE_SHATTER_ACTION)
                        .copied()
                        .map(|d| d * 25.0)
                        .filter(|d| *d > 0.0)
                        .unwrap_or(100.0);
                    Some((delay_ms, motion_count))
                });
            match anim {
                Some((delay_ms, motion_count)) if motion_count > 0 => {
                    self.game.world.freeze_shatters.retain_mut(|s| {
                        let start = *s.started_at.get_or_insert(elapsed);
                        let frame = ((elapsed - start) * 1000.0 / delay_ms) as usize;
                        frame < motion_count
                    });
                }
                _ => self.game.world.freeze_shatters.clear(),
            }
        }

        let mut unified_list: Vec<&RenderEntry> = render_list
            .iter()
            .chain(floor_item_render_list.iter())
            .chain(cart_render_list.iter())
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
                        self.game.sprite_caches.sprites.get(&entry.id),
                        self.game.world.entities.get(entry.id),
                    ) {
                        let render = hidden_render(
                            entity.effect_state,
                            self.hidden_viewer_for(entry.id),
                            self.player_clairvoyant(),
                        );
                        if render == HiddenRender::Skip {
                            continue;
                        }
                        let fade_alpha = entity.alpha();
                        let body_alpha = fade_alpha
                            * match render {
                                HiddenRender::Alpha(a) => a,
                                _ => 1.0,
                            };
                        let is_fading = fade_alpha < 1.0;

                        // A hidden self keeps its shadow (visible gliding under
                        // Tunnel Drive) but no body.
                        let visible_body = !is_fading
                            && matches!(render, HiddenRender::Visible | HiddenRender::ShadowOnly);
                        let sits_or_lies =
                            matches!(entity.state, EntityState::Sitting | EntityState::Dead);
                        if visible_body && !sits_or_lies {
                            let shadow_scale =
                                entry.sprite_scale * SHADOW_SCALE * shadow_size(entity.job);
                            let mut shadow = sprite.build_shadow_batches(
                                entry.screen_anchor,
                                entry.depth,
                                shadow_scale,
                                entry.flat_depth_gradient,
                            );
                            shadow_batches.append(&mut shadow);
                        }

                        // Gradient [0,0] gives the whole sprite one depth: its
                        // feet. So effects occlude against the body at foot level.
                        // Skip for the dead: their big death-pop frame would stamp
                        // that depth over a ground effect they lie in and erase it.
                        if visible_body
                            && render == HiddenRender::Visible
                            && entity.state != EntityState::Dead
                        {
                            let mut sil = sprite.build_batches(
                                &entity.animation,
                                Some(entry.camera_dir),
                                entity.head_dir,
                                entry.screen_anchor,
                                entry.depth,
                                entry.sprite_scale,
                                [0.0, 0.0],
                            );
                            silhouette_batches.append(&mut sil);
                        }

                        if render == HiddenRender::ShadowOnly {
                            continue;
                        }

                        let mut body_channels =
                            self.effect_holder.body_channels_for_entity(entry.id);
                        body_channels.copy_margin_scale = copy_margin_scale;
                        body_channels.scale *=
                            ragnarok_game::sprite_path::baby_body_scale(entity.job);
                        if let Some(rgb) = ailment::ailment_visual(
                            entity.body_state,
                            entity.health_state,
                            entity.rooted,
                        )
                        .tint
                        {
                            body_channels.tint = Some(rgb);
                        }
                        if render == HiddenRender::Silhouette {
                            body_channels.tint = Some([0, 0, 0]);
                        }
                        body_channels.alpha *= body_alpha;
                        body_channels.lift_px += entity.hover_lift_px(camera_distance);
                        body_channels.light = self.actor_light(entity.movement.position());

                        // A living sprite stands upright (depth varies head-to-feet).
                        // A corpse lies flat, so its depth follows the ground plane.
                        let body_gradient = if entity.state == EntityState::Dead {
                            entry.flat_depth_gradient
                        } else {
                            entry.depth_gradient
                        };
                        let mut batches = ragnarok_renderer::compose_actor_batches(
                            sprite,
                            &entity.animation,
                            entry.camera_dir,
                            entity.head_dir,
                            entry.screen_anchor,
                            entry.depth,
                            entry.sprite_scale,
                            body_gradient,
                            &body_channels,
                        );

                        if let Some(ai) = self.effect_holder.afterimage_params_for_entity(entry.id)
                        {
                            let trailing = entity.state == EntityState::Moving;
                            let action = entity.animation.action();
                            let motion = entity.animation.motion_index();
                            let last = self
                                .effect_holder
                                .afterimages_for_entity(entry.id)
                                .last()
                                .map(|i| (i.anim.action(), i.anim.motion_index()));
                            let frames: Vec<usize> = match last {
                                Some((a, m)) if a == action && motion > m => {
                                    (m + 1..=motion).collect()
                                }
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
                                let (anchor, depth, scale, depth_gradient) = self
                                    .renderer
                                    .as_ref()
                                    .zip(self.game.session.map_coords.as_ref())
                                    .and_then(|(r, coords)| {
                                        let sw = r.device.surface_config.width as f32 / r.dpi_scale;
                                        let sh =
                                            r.device.surface_config.height as f32 / r.dpi_scale;
                                        crate::input::entity_screen_params(
                                            img.world_pos,
                                            self.game.session.gat.as_ref(),
                                            coords,
                                            &r.camera,
                                            sw,
                                            sh,
                                        )
                                    })
                                    .map(|(a, d, _cd, s, dg)| (a, d, s, dg))
                                    .unwrap_or((
                                        entry.screen_anchor,
                                        entry.depth,
                                        entry.sprite_scale,
                                        entry.depth_gradient,
                                    ));
                                let mut copy = sprite.build_batches(
                                    &img.anim,
                                    img.camera_dir,
                                    img.head_dir,
                                    anchor,
                                    depth,
                                    scale,
                                    depth_gradient,
                                );
                                let (tr, tg, tb) = (
                                    img.tint[0] as f32 / 255.0,
                                    img.tint[1] as f32 / 255.0,
                                    img.tint[2] as f32 / 255.0,
                                );
                                for batch in &mut copy {
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

                        // TODO Move emotion in dedicated place
                        if let (Some(emo), Some(emo_act), Some(emo_tex)) = (
                            &entity.emotion,
                            &self.game.assets.emotion_act,
                            &self.game.assets.emotion_textures,
                        ) {
                            if let Some((action_idx, delay_ms, frames)) =
                                ragnarok_game::emotion::emote_timing(emo_act, emo.emotion_type)
                            {
                                let motion_idx =
                                    (((emo.elapsed * 1000.0) / delay_ms) as usize).min(frames - 1);
                                let motion = &emo_act.actions[action_idx].motions[motion_idx];
                                let emo_center = [
                                    entry.screen_anchor[0],
                                    entry.screen_anchor[1]
                                        - entry.head_offset
                                        - 6.0 * entry.sprite_scale,
                                ];
                                for clip in &motion.clips {
                                    if let Some((vertices, indices, tex_idx)) = build_clip_quad(
                                        clip,
                                        emo_tex,
                                        emo_center,
                                        entry.depth,
                                        [0.0, 0.0],
                                    ) && tex_idx < emo_tex.bind_groups.len()
                                    {
                                        sprite_batches.push(SpriteBatch {
                                            vertices,
                                            indices,
                                            texture: &emo_tex.bind_groups[tex_idx],
                                            additive: false,
                                            no_depth: false,
                                        });
                                    }
                                }
                            }
                        }

                        if let (Some(marker), Some(emo_act), Some(emo_tex)) = (
                            self.game.quest_markers.get(&entry.id),
                            &self.game.assets.emotion_act,
                            &self.game.assets.emotion_textures,
                        ) {
                            let action_idx =
                                ragnarok_game::quest::marker_sprite_action(marker.effect);
                            if action_idx < emo_act.actions.len() {
                                let delay_ms = emo_act
                                    .delays
                                    .get(action_idx)
                                    .map(|d| d * 25.0)
                                    .filter(|d| *d > 0.0)
                                    .unwrap_or(150.0);
                                let motion_count = emo_act.actions[action_idx].motions.len();
                                if motion_count > 0 {
                                    let motion_idx =
                                        ((elapsed * 1000.0) / delay_ms) as usize % motion_count;
                                    let motion = &emo_act.actions[action_idx].motions[motion_idx];
                                    let emo_center = [
                                        entry.screen_anchor[0],
                                        entry.screen_anchor[1]
                                            - entry.head_offset
                                            - 6.0 * entry.sprite_scale,
                                    ];
                                    for clip in &motion.clips {
                                        if let Some((vertices, indices, tex_idx)) = build_clip_quad(
                                            clip,
                                            emo_tex,
                                            emo_center,
                                            entry.depth,
                                            [0.0, 0.0],
                                        ) && tex_idx < emo_tex.bind_groups.len()
                                        {
                                            sprite_batches.push(SpriteBatch {
                                                vertices,
                                                indices,
                                                texture: &emo_tex.bind_groups[tex_idx],
                                                additive: false,
                                                no_depth: false,
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        // Move ailment in a dedicated place
                        for overlay in
                            ailment::ailment_overlays(entity.body_state, entity.health_state)
                        {
                            let Some((tex, act)) =
                                self.game.assets.status_overlay_sprites.get(&overlay)
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
                            let center = if overlay.on_body() {
                                entry.screen_anchor
                            } else {
                                [
                                    entry.screen_anchor[0],
                                    entry.screen_anchor[1]
                                        - entry.head_offset
                                        - 6.0 * entry.sprite_scale,
                                ]
                            };
                            for clip in &motion.clips {
                                if let Some((vertices, indices, tex_idx)) = build_clip_quad_scaled(
                                    clip,
                                    tex,
                                    center,
                                    entry.depth,
                                    [0.0, 0.0],
                                    entry.sprite_scale,
                                ) && tex_idx < tex.bind_groups.len()
                                {
                                    sprite_batches.push(SpriteBatch {
                                        vertices,
                                        indices,
                                        texture: &tex.bind_groups[tex_idx],
                                        additive: false,
                                        no_depth: false,
                                    });
                                }
                            }
                        }

                        // TODO refactor and move this in another place
                        for shatter in &self.game.world.freeze_shatters {
                            if shatter.gid != entry.id {
                                continue;
                            }
                            let Some((tex, act)) = self
                                .game
                                .assets
                                .status_overlay_sprites
                                .get(&ailment::AilmentOverlay::Freeze)
                            else {
                                continue;
                            };
                            let Some(action) = act.actions.get(FREEZE_SHATTER_ACTION) else {
                                continue;
                            };
                            let motion_count = action.motions.len();
                            if motion_count == 0 {
                                continue;
                            }
                            let delay_ms = act
                                .delays
                                .get(FREEZE_SHATTER_ACTION)
                                .copied()
                                .map(|d| d * 25.0)
                                .filter(|d| *d > 0.0)
                                .unwrap_or(100.0);
                            let start = shatter.started_at.unwrap_or(elapsed);
                            let frame = ((elapsed - start) * 1000.0 / delay_ms) as usize;
                            if frame >= motion_count {
                                continue;
                            }
                            for clip in &action.motions[frame].clips {
                                if let Some((vertices, indices, tex_idx)) = build_clip_quad_scaled(
                                    clip,
                                    tex,
                                    entry.screen_anchor,
                                    entry.depth,
                                    [0.0, 0.0],
                                    entry.sprite_scale,
                                ) && tex_idx < tex.bind_groups.len()
                                {
                                    sprite_batches.push(SpriteBatch {
                                        vertices,
                                        indices,
                                        texture: &tex.bind_groups[tex_idx],
                                        additive: false,
                                        no_depth: false,
                                    });
                                }
                            }
                        }
                    }
                }
                RenderEntryKind::FloorItem => {
                    if let Some(floor_item) = self.game.world.floor_items.get(&entry.id)
                        && let Some((tex, act)) = self.game.assets.floor_item_sprites.get(&entry.id)
                    {
                        let blink_active = floor_item.blink_active(elapsed);
                        let light = self.actor_light((floor_item.x as f32, floor_item.y as f32));

                        let center = entry.screen_anchor;

                        let motion = act.actions.first().and_then(|action| {
                            let motion_count = action.motions.len();
                            if motion_count == 0 {
                                return None;
                            }
                            let delay_ms = act
                                .delays
                                .first()
                                .map(|d| d * 25.0)
                                .filter(|d| *d > 0.0)
                                .unwrap_or(150.0);
                            let item_elapsed = elapsed - floor_item.drop_time;
                            let motion_idx =
                                ((item_elapsed * 1000.0) / delay_ms) as usize % motion_count;
                            action.motions.get(motion_idx)
                        });

                        // The shadow stays on the floor while the item arcs above it,
                        // and sits at the sprite's base rather than its centre.
                        if let Some((shadow_tex, shadow_act)) = &self.game.assets.shadow_sprite {
                            let (anchor, depth, scale) = self
                                .floor_item_ground_projection(floor_item)
                                .unwrap_or((entry.screen_anchor, entry.depth, entry.sprite_scale));
                            let base_drop = motion.map_or(0.0, |m| {
                                m.clips
                                    .iter()
                                    .map(|clip| {
                                        ragnarok_renderer::sprite::clip_bottom_offset(clip, tex)
                                    })
                                    .fold(0.0, f32::max)
                            });
                            let mut shadow = ragnarok_renderer::sprite::build_shadow_batches(
                                shadow_act,
                                shadow_tex,
                                [anchor[0], anchor[1] + base_drop * entry.sprite_scale],
                                depth,
                                scale * FLOOR_ITEM_SHADOW_SCALE,
                                entry.flat_depth_gradient,
                            );
                            shadow_batches.append(&mut shadow);
                        }

                        if let Some(motion) = motion {
                            for clip in &motion.clips {
                                if let Some((mut vertices, indices, tex_idx)) =
                                    build_clip_quad(clip, tex, center, entry.depth, [0.0, 0.0])
                                {
                                    scale_clip_vertices(
                                        &mut vertices,
                                        center,
                                        entry.sprite_scale,
                                        entry.depth_gradient,
                                    );
                                    if blink_active {
                                        for v in &mut vertices {
                                            v.color = [1.0, 0.0, 0.0, 1.0];
                                        }
                                    } else {
                                        for v in &mut vertices {
                                            v.color[0] *= light[0];
                                            v.color[1] *= light[1];
                                            v.color[2] *= light[2];
                                        }
                                    }
                                    if tex_idx < tex.bind_groups.len() {
                                        sprite_batches.push(SpriteBatch {
                                            vertices,
                                            indices,
                                            texture: &tex.bind_groups[tex_idx],
                                            additive: false,
                                            no_depth: false,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                RenderEntryKind::Cart => {
                    if let (Some(cart), Some(entity)) = (
                        self.game.sprite_caches.carts.get(&entry.id),
                        self.game.world.entities.get(entry.id),
                    ) {
                        if is_hidden(entity.effect_state) {
                            continue;
                        }
                        let mut body_channels =
                            self.effect_holder.body_channels_for_entity(entry.id);
                        body_channels.copy_margin_scale = copy_margin_scale;
                        body_channels.alpha *= entity.alpha();
                        body_channels.light = self.actor_light(entity.movement.position());

                        // Flat feet-depth silhouette so effects (e.g. the level 99
                        // aura) occlude against the cart instead of bleeding
                        // through it, matching the player body.
                        if entity.alpha() >= 1.0 {
                            let mut sil = cart.sprite.build_batches(
                                &cart.animation,
                                Some(entry.camera_dir),
                                entity.direction,
                                entry.screen_anchor,
                                entry.depth,
                                entry.sprite_scale,
                                [0.0, 0.0],
                            );
                            silhouette_batches.append(&mut sil);
                        }
                        let mut batches = ragnarok_renderer::compose_actor_batches(
                            &cart.sprite,
                            &cart.animation,
                            entry.camera_dir,
                            entity.direction,
                            entry.screen_anchor,
                            entry.depth,
                            entry.sprite_scale,
                            entry.depth_gradient,
                            &body_channels,
                        );
                        sprite_batches.append(&mut batches);
                    }
                }
                RenderEntryKind::Falcon => {
                    if let (Some(falcon), Some(entity)) = (
                        self.game.sprite_caches.falcons.get(&entry.id),
                        self.game.world.entities.get(entry.id),
                    ) {
                        if is_hidden(entity.effect_state) {
                            continue;
                        }
                        let mut body_channels =
                            self.effect_holder.body_channels_for_entity(entry.id);
                        body_channels.copy_margin_scale = copy_margin_scale;
                        body_channels.alpha *= entity.alpha();
                        body_channels.light = self.actor_light(entity.movement.position());

                        // Flat feet-depth silhouette so effects occlude against the
                        // falcon instead of bleeding through it (see the cart arm).
                        if entity.alpha() >= 1.0 {
                            let mut sil = falcon.sprite.build_batches(
                                &falcon.animation,
                                Some(entry.camera_dir),
                                falcon.motion.direction,
                                entry.screen_anchor,
                                entry.depth,
                                entry.sprite_scale,
                                [0.0, 0.0],
                            );
                            silhouette_batches.append(&mut sil);
                        }
                        let mut batches = ragnarok_renderer::compose_actor_batches(
                            &falcon.sprite,
                            &falcon.animation,
                            entry.camera_dir,
                            falcon.motion.direction,
                            entry.screen_anchor,
                            entry.depth,
                            entry.sprite_scale,
                            entry.depth_gradient,
                            &body_channels,
                        );
                        sprite_batches.append(&mut batches);
                    }
                }
            }
        }

        shadow_batches.append(&mut sprite_batches);
        let sprite_batches = shadow_batches;

        let mut inline_textures = Vec::new();
        let mut paperdoll_calls: Vec<UiDrawCall> = Vec::new();
        if let Some(center) = self.windows.equipment_window.character_center()
            && let Some(player_id) = self.game.world.entities.player_id()
            && let Some(sprite) = self.game.sprite_caches.sprites.get(&player_id)
        {
            let idle_anim = ragnarok_formats::act::SpriteAnimationState::new(0);
            let batches = sprite.build_batches(&idle_anim, None, 0, center, 0.0, 1.0, [0.0, 0.0]);
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

        if let Some(center) = self.windows.equipment_window.cart_slot_center()
            && let Some(player_id) = self.game.world.entities.player_id()
            && let Some(cart) = self.game.sprite_caches.carts.get(&player_id)
        {
            let idle_anim = ragnarok_formats::act::SpriteAnimationState::new(0);
            let batches =
                cart.sprite
                    .build_batches(&idle_anim, None, 0, center, 0.0, 0.5, [0.0, 0.0]);
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

        let mut cart_select_calls: Vec<UiDrawCall> = Vec::new();
        for &(design, center) in self.windows.cart_select_window.model_previews() {
            let Some(sprite) = self.game.sprite_caches.cart_preview_sprites.get(&design) else {
                continue;
            };
            let idle_anim = ragnarok_formats::act::SpriteAnimationState::new(0);
            let batches = sprite.build_batches(
                &idle_anim,
                None,
                0,
                center,
                0.0,
                ragnarok_ui_component::game::cart_select_window::PREVIEW_SCALE,
                [0.0, 0.0],
            );
            for batch in batches {
                let idx = inline_textures.len();
                inline_textures.push(batch.texture);
                cart_select_calls.push(UiDrawCall {
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

        let mut account_calls: Vec<UiDrawCall> = Vec::new();
        match self.game.session.app_state {
            AppState::CharacterSelect => {
                if let Some(win) = &self.char_select_window {
                    for view in win.visible_slot_views() {
                        let Some(ch) = win.characters.get(view.char_index) else {
                            continue;
                        };
                        let Some(sprite) = self.game.sprite_caches.sprites.get(&ch.gid) else {
                            continue;
                        };
                        let Some(anim) = self.account_anims.get(&ch.gid) else {
                            continue;
                        };
                        let batches =
                            sprite.build_batches(anim, None, 0, view.anchor, 0.0, 1.0, [0.0, 0.0]);
                        for batch in batches {
                            let idx = inline_textures.len();
                            inline_textures.push(batch.texture);
                            account_calls.push(UiDrawCall {
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
                }
            }
            AppState::CharacterCreate => {
                if let (Some(win), Some(sprite), Some(anim)) = (
                    &self.char_create_window,
                    self.game.sprite_caches.sprites.get(&CREATE_PREVIEW_GID),
                    self.account_anims.get(&CREATE_PREVIEW_GID),
                ) {
                    let batches = sprite.build_batches(
                        anim,
                        None,
                        0,
                        win.preview_anchor(),
                        0.0,
                        1.0,
                        [0.0, 0.0],
                    );
                    for batch in batches {
                        let idx = inline_textures.len();
                        inline_textures.push(batch.texture);
                        account_calls.push(UiDrawCall {
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
            }
            _ => {}
        }

        let mut guild_head_calls: Vec<UiDrawCall> = Vec::new();
        if self.game.session.app_state == AppState::InGame && self.windows.guild_window.is_open() {
            let idle = ragnarok_formats::act::SpriteAnimationState::new(0);
            for &(gid, center) in self.windows.guild_window.member_head_slots() {
                let Some(sprite) = self.game.sprite_caches.guild_head_sprites.get(&gid) else {
                    continue;
                };
                for batch in sprite.build_head_batches(&idle, None, 0, center, 26.0, 0.0) {
                    let idx = inline_textures.len();
                    inline_textures.push(batch.texture);
                    guild_head_calls.push(UiDrawCall {
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
        }

        let mut roulette_calls: Vec<UiDrawCall> = Vec::new();
        if let (Some(roulette), Some(act), Some(tex), Some(renderer)) = (
            &self.game.companions.pet_roulette,
            &self.roulette_act,
            &self.roulette_textures,
            &self.renderer,
        ) {
            let sw = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
            let sh = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            let center = [sw / 2.0, sh / 2.0];
            let (action_idx, motion_idx) = roulette.frame;
            if let Some(action) = act.actions.get(action_idx)
                && let Some(motion) = action.motions.get(motion_idx)
            {
                for clip in &motion.clips {
                    if let Some((vertices, indices, tex_idx)) =
                        build_clip_quad(clip, tex, center, 0.0, [0.0, 0.0])
                        && tex_idx < tex.bind_groups.len()
                    {
                        let idx = inline_textures.len();
                        inline_textures.push(&tex.bind_groups[tex_idx]);
                        roulette_calls.push(UiDrawCall {
                            vertices: vertices
                                .iter()
                                .map(|sv| UiVertex {
                                    position: [sv.position[0], sv.position[1]],
                                    tex_coord: sv.tex_coord,
                                    color: sv.color,
                                })
                                .collect(),
                            indices,
                            texture: UiTextureRef::Inline(idx),
                        });
                    }
                }
            }
        }

        {
            use ragnarok_game::damage_number::{
                build_damage_number_entries, build_damage_number_quads,
            };
            use ragnarok_renderer::sprite_projection::project_cell_offset;
            // Borrow the fields separately: the numbers need &mut for their
            // cached fallback position while the projection reads the world.
            let camera = self.renderer.as_ref().map(|r| {
                let w = r.device.surface_config.width as f32 / r.dpi_scale;
                let h = r.device.surface_config.height as f32 / r.dpi_scale;
                (&r.camera, w, h)
            });
            let coords = self.game.session.map_coords.as_ref();
            let gat = self.game.session.gat.as_ref();
            let entities = &self.game.world.entities;
            let units = &self.game.world.trap_units;
            let entries = build_damage_number_entries(
                &mut self.game.combat.damage_numbers.numbers,
                |entity_id, offset| {
                    let (camera, screen_w, screen_h) = camera?;
                    let (cx, cy) = match entities.get(entity_id) {
                        Some(entity) => entity.movement.position(),
                        None => {
                            let cell = units.get(&entity_id)?.cell;
                            (cell.0 as f32, cell.1 as f32)
                        }
                    };
                    project_cell_offset(
                        (cx as f32, cy as f32),
                        offset,
                        gat,
                        coords?,
                        camera,
                        screen_w,
                        screen_h,
                    )
                },
            );
            if let (Some(num_tex), Some(num_act)) = (
                &self.game.assets.damage_number_textures,
                &self.game.assets.damage_number_act,
            ) {
                let quads = build_damage_number_quads(
                    &entries,
                    num_act,
                    &num_tex.sizes,
                    num_tex.indexed_count,
                    self.game
                        .assets
                        .damage_msg_textures
                        .as_ref()
                        .map(|t| t.sizes.as_slice()),
                );
                ragnarok_renderer::render_damage_number_quads(
                    &quads,
                    num_tex,
                    self.game.assets.damage_msg_textures.as_ref(),
                    &mut world_overlay_calls,
                    &mut inline_textures,
                );
            }
        }

        if self.game.session.map_properties.is_pk_zone()
            && let Some(player) = self.game.world.entities.player()
            && let (Some(act), Some(tex), Some(renderer)) = (
                &self.game.assets.rank_font_act,
                &self.game.assets.rank_font_textures,
                &self.renderer,
            )
        {
            let sw = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
            let sh = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            for quad in pk_rank_hud_quads(player.pk_rank, player.pk_total, sw, sh) {
                let Some(motion) = act
                    .actions
                    .get(quad.action)
                    .and_then(|action| action.motions.first())
                else {
                    continue;
                };
                for clip in &motion.clips {
                    if let Some((vertices, indices, tex_idx)) =
                        build_clip_quad(clip, tex, [quad.x, quad.y], 0.0, [0.0, 0.0])
                        && tex_idx < tex.bind_groups.len()
                    {
                        let idx = inline_textures.len();
                        inline_textures.push(&tex.bind_groups[tex_idx]);
                        world_overlay_calls.push(UiDrawCall {
                            vertices: vertices
                                .iter()
                                .map(|sv| UiVertex {
                                    position: [sv.position[0], sv.position[1]],
                                    tex_coord: sv.tex_coord,
                                    color: sv.color,
                                })
                                .collect(),
                            indices,
                            texture: UiTextureRef::Inline(idx),
                        });
                    }
                }
            }
        }

        if let Some(clock) = &self.game.show_digit
            && let (Some(act), Some(tex), Some(renderer)) = (
                &self.game.assets.time_font_act,
                &self.game.assets.time_font_textures,
                &self.renderer,
            )
        {
            let sw = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
            for quad in clock.quads(sw) {
                let Some(motion) = act
                    .actions
                    .get(quad.action)
                    .and_then(|action| action.motions.first())
                else {
                    continue;
                };
                for clip in &motion.clips {
                    if let Some((vertices, indices, tex_idx)) =
                        build_clip_quad(clip, tex, [quad.x, quad.y], 0.0, [0.0, 0.0])
                        && tex_idx < tex.bind_groups.len()
                    {
                        let idx = inline_textures.len();
                        inline_textures.push(&tex.bind_groups[tex_idx]);
                        world_overlay_calls.push(UiDrawCall {
                            vertices: vertices
                                .iter()
                                .map(|sv| UiVertex {
                                    position: [sv.position[0], sv.position[1]],
                                    tex_coord: sv.tex_coord,
                                    color: sv.color,
                                })
                                .collect(),
                            indices,
                            texture: UiTextureRef::Inline(idx),
                        });
                    }
                }
            }
        }

        if let Some(cursor_tex) = &self.game.assets.cursor_textures {
            for (vertices, indices, tex_idx) in lock_cursor_clips {
                cursor_batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &cursor_tex.bind_groups[tex_idx],
                    additive: false,
                    no_depth: false,
                });
            }
            for (vertices, indices, tex_idx) in cursor_clips {
                cursor_batches.push(SpriteBatch {
                    vertices,
                    indices,
                    texture: &cursor_tex.bind_groups[tex_idx],
                    additive: false,
                    no_depth: false,
                });
            }
        }

        let mut all_ui_calls = world_overlay_calls;
        let overlay_len = all_ui_calls.len();
        all_ui_calls.extend(ui_draw_calls);

        let paperdoll_abs = self
            .windows
            .equipment_window
            .paperdoll_insert_index()
            .map(|idx| (overlay_len + idx).min(all_ui_calls.len()));
        let paperdoll_len = paperdoll_calls.len();
        if let Some(abs_idx) = paperdoll_abs {
            for (i, dc) in paperdoll_calls.into_iter().enumerate() {
                all_ui_calls.insert(abs_idx + i, dc);
            }
        }

        let cart_len = cart_select_calls.len();
        let mut cart_abs: Option<usize> = None;
        if let Some(insert_idx) = self.windows.cart_select_window.preview_insert_index() {
            let mut abs_idx = (overlay_len + insert_idx).min(all_ui_calls.len());
            // The paperdoll insertion above shifts later indices forward.
            if paperdoll_abs.is_some_and(|pd| abs_idx >= pd) {
                abs_idx += paperdoll_len;
            }
            let abs_idx = abs_idx.min(all_ui_calls.len());
            cart_abs = Some(abs_idx);
            for (i, dc) in cart_select_calls.into_iter().enumerate() {
                all_ui_calls.insert(abs_idx + i, dc);
            }
        }

        if let Some(insert_idx) = self.windows.guild_window.head_insert_index()
            && !guild_head_calls.is_empty()
        {
            let mut abs_idx = overlay_len + insert_idx;
            if paperdoll_abs.is_some_and(|pd| abs_idx >= pd) {
                abs_idx += paperdoll_len;
            }
            if cart_abs.is_some_and(|c| abs_idx >= c) {
                abs_idx += cart_len;
            }
            let abs_idx = abs_idx.min(all_ui_calls.len());
            for (i, dc) in guild_head_calls.into_iter().enumerate() {
                all_ui_calls.insert(abs_idx + i, dc);
            }
        }

        let account_insert_idx = match self.game.session.app_state {
            AppState::CharacterSelect => self
                .char_select_window
                .as_ref()
                .and_then(|w| w.sprite_insert_index()),
            _ => None,
        }
        .map(|idx| (overlay_len + idx).min(all_ui_calls.len()));
        if let Some(abs_idx) = account_insert_idx {
            for (i, dc) in account_calls.into_iter().enumerate() {
                all_ui_calls.insert(abs_idx + i, dc);
            }
        } else {
            all_ui_calls.extend(account_calls);
        }
        all_ui_calls.extend(skill_level_calls);
        all_ui_calls.extend(roulette_calls);

        let screen_ripple = self.game.session.screen_ripple
            && self.config.show_skill_effects
            && self.game.session.app_state == AppState::InGame;

        if let Some(renderer) = &mut self.renderer {
            renderer.screen_distortion.set_active(screen_ripple);
            let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
            let screen_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
            let mut arrow_draws: Vec<EffectPrimitiveDraw> = self
                .game
                .world
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
            if let (Some(gat), Some(coords)) = (
                self.game.session.gat.as_ref(),
                self.game.session.map_coords.as_ref(),
            ) {
                let cell_size = coords.cell_to_world(1.0, 0.0).0 - coords.cell_to_world(0.0, 0.0).0;
                for (aid, g) in &self.game.world.graffiti {
                    let (cx, cy) = (g.cell_x as f32 + 0.5, g.cell_y as f32 + 0.5);
                    let (wx, _, wz) = coords.cell_to_world(cx, cy);
                    let center = [wx, gat.get_height(cx, cy) + GRAFFITI_GROUND_LIFT, wz];
                    let (corners, uv) =
                        ragnarok_game::graffiti::decal_quad(center, g.yaw, cell_size);
                    arrow_draws.push(EffectPrimitiveDraw::KeyedWorldQuad {
                        corners,
                        uv,
                        texture_key: ragnarok_renderer::graffiti::texture_key(*aid),
                        color: [1.0, 1.0, 1.0, 1.0],
                        blend: BlendKind::Alpha,
                        no_depth: false,
                    });
                }
                for mark in self.game.world.cast_marks.values() {
                    let CastMark::Scope(scope) = mark else {
                        continue;
                    };
                    let (ox, oy) = scope.origin_cell();
                    let color = scope.color();
                    for row in 0..scope.size {
                        for col in 0..scope.size {
                            let (cx, cy) = (ox + col as i32, oy + row as i32);
                            if !coords.is_valid_cell(cx, cy) {
                                continue;
                            }
                            let c = coords.cell_corners_world(gat, cx, cy);
                            let uv = scope.cell_uv(col, row);
                            arrow_draws.push(EffectPrimitiveDraw::WorldQuad {
                                corners: [c[0], c[1], c[3], c[2]],
                                uv: [uv[0], uv[1], uv[3], uv[2]],
                                texture: ragnarok_game::cast_scope::SCOPE_TEXTURE,
                                color,
                                blend: BlendKind::Alpha,
                                no_depth: false,
                            });
                        }
                    }
                }
            }
            let zoom = self
                .game
                .session
                .map_coords
                .as_ref()
                .map_or(10.0, |c| c.zoom());
            let entities = &self.game.world.entities;
            let gat = self.game.session.gat.as_ref();
            let map_coords = self.game.session.map_coords.as_ref();
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
                let _frustums = frame
                    .effect_draws
                    .primitives
                    .iter()
                    .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
                    .count();
                let _billboards = frame
                    .effect_draws
                    .primitives
                    .iter()
                    .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { .. }))
                    .count();
            }

            renderer.render(FrameInputs {
                ui_draw_calls: &all_ui_calls,
                tooltip_draw_calls: &tooltip_draw_calls,
                effect_sprite_batches: &frame.effect_batches,
                effect_draws: &frame.effect_draws,
                sprite_particle_records: frame.sprite_particle_records,
                sprite_batches: &sprite_batches,
                silhouette_batches: &silhouette_batches,
                cursor_batches,
                inline_textures: &inline_textures,
                elapsed,
                delta,
            });
        }
    }
}
