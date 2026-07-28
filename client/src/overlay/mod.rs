use crate::App;
use ragnarok_game::cursor::RenderEntry;
use ragnarok_game::entity::{Entity, EntityState, EntityType};
use ragnarok_game::targeting::{GM_TEXT_COLOR, pk_name_color};
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui_component::game::chat_room_board;
use ragnarok_ui_component::game::vending_board;
use ragnarok_ui_component::helper::dialog_container::DialogContainer;

/// Screen-space `[x0, y0, x1, y1]` of a vendor's shop board, centered above its
/// head. Shared by the overlay renderer and the hover hit-test so they agree.
pub(crate) fn vending_board_rect(entry: &RenderEntry) -> [f32; 4] {
    vending_board::board_rect(
        entry.screen_anchor[0],
        entry.screen_anchor[1],
        entry.head_offset,
    )
}

/// Screen-space `[x0, y0, x1, y1]` of a chat room board, centered above the
/// owner's head. Shared by the overlay renderer and the hover hit-test.
pub(crate) fn chat_room_board_rect(entry: &RenderEntry) -> [f32; 4] {
    chat_room_board::board_rect(
        entry.screen_anchor[0],
        entry.screen_anchor[1],
        entry.head_offset,
    )
}

const HP_BAR_WIDTH: f32 = 60.0;
pub(crate) const HP_BAR_HEIGHT: f32 = 5.0;
const SP_BAR_COLOR: [f32; 4] = [0.063, 0.094, 0.61, 1.0];
const CAST_BAR_COLOR: [f32; 4] = [0.0, 0.8, 0.0, 1.0];
const GUILD_NAME_COLOR: [f32; 4] = [0.8, 1.0, 0.753, 1.0];
const MOB_INFO_COLOR: [f32; 4] = [0.9, 0.9, 0.9, 1.0];
const EMBLEM_HOVER_SIZE: f32 = 24.0;
const EMBLEM_HEAD_SIZE: f32 = 24.0;

impl App {
    pub(crate) fn build_world_overlays(
        &self,
        render_list: &[RenderEntry],
        floor_item_render_list: &[RenderEntry],
        hovered_entity_id: Option<u32>,
        hovered_floor_item_id: Option<u32>,
    ) -> Vec<UiDrawCall> {
        let mut calls = Vec::new();

        self.build_npc_cutins(&mut calls);
        self.build_hovered_entity_overlay(hovered_entity_id, render_list, &mut calls);
        self.build_guild_emblems(render_list, &mut calls);
        self.build_player_bars(hovered_entity_id, render_list, &mut calls);
        self.build_persistent_bars(hovered_entity_id, render_list, &mut calls);
        self.build_cast_bars(render_list, &mut calls);
        self.build_chat_bubbles(render_list, &mut calls);
        self.build_vending_boards(render_list, &mut calls);
        self.build_chat_room_boards(render_list, &mut calls);
        self.build_floor_item_tooltip(hovered_floor_item_id, floor_item_render_list, &mut calls);
        self.build_debug_pick_bounds(render_list, floor_item_render_list, &mut calls);

        calls
    }

    /// NPC cutin illustrations, anchored along the bottom of the screen at their
    /// native size: slot 0 left, 1 middle, 2 right. Drawn before the hover overlay
    /// so it sits under the NPC dialog.
    fn build_npc_cutins(&self, calls: &mut Vec<UiDrawCall>) {
        let Some(renderer) = &self.renderer else {
            return;
        };
        let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
        let screen_h = renderer.device.surface_config.height as f32 / renderer.dpi_scale;
        for (slot, image) in self.game.npc_cutins.iter().enumerate() {
            let Some(image) = image else { continue };
            let path = crate::events::marriage::cutin_texture_path(image);
            let Some((w, h)) = renderer.texture_cache.texture_size(&path) else {
                continue;
            };
            let (w, h) = (w as f32, h as f32);
            let x = match slot {
                0 => 0.0,
                1 => ((screen_w - w) / 2.0).max(0.0),
                _ => (screen_w - w).max(0.0),
            };
            let y = (screen_h - h).max(0.0);
            let (verts, indices) = ragnarok_ui::draw::quad_vertices(x, y, w, h, [1.0; 4]);
            calls.push(UiDrawCall {
                vertices: verts.to_vec(),
                indices: indices.to_vec(),
                texture: UiTextureRef::Named(path),
            });
        }
    }

    fn build_hovered_entity_overlay(
        &self,
        hovered_entity_id: Option<u32>,
        render_list: &[RenderEntry],
        calls: &mut Vec<UiDrawCall>,
    ) {
        let (entity_id, renderer) = match (hovered_entity_id, &self.renderer) {
            (Some(id), Some(r)) => (id, r),
            _ => return,
        };
        let entity = match self.game.world.entities.get(entity_id) {
            Some(e) => e,
            None => return,
        };
        let entry = match render_list.iter().find(|e| e.id == entity_id) {
            Some(e) => e,
            None => return,
        };

        let mut bar_y = entry.pick_bounds[3] + 5.0;
        let hp_ratio = self.entity_hp_ratio(entity_id);
        if let Some(ratio) = hp_ratio {
            let (_x, y) = render_hp_bar(entry, ratio, entity.entity_type, calls);
            bar_y = y;
            if let Some(sp_ratio) = self.entity_sp_ratio(entity_id) {
                let sp_y = y + HP_BAR_HEIGHT;
                render_bar(entry.screen_anchor[0], sp_y, sp_ratio, SP_BAR_COLOR, calls);
                bar_y = sp_y;
            }
        }
        if let Some(name) = &entity.name
            && !self.name_hidden(entity.entity_type)
        {
            let text_width = renderer.font_atlas.measure_text(name);
            let text_x = entry.screen_anchor[0] - text_width / 2.0;
            let name_y = bar_y + HP_BAR_HEIGHT + 13.0;
            let mut text_y = name_y;
            build_outlined_text(
                name,
                text_x,
                text_y,
                entity_name_color(entity),
                &renderer.font_atlas,
                calls,
            );

            let mut leftmost_x = text_x;
            if let Some((line, color)) = second_plate_line(entity) {
                let line_width = renderer.font_atlas.measure_text(&line);
                let line_x = entry.screen_anchor[0] - line_width / 2.0;
                leftmost_x = leftmost_x.min(line_x);
                text_y += renderer.font_atlas.line_height;
                build_outlined_text(&line, line_x, text_y, color, &renderer.font_atlas, calls);
            }

            if entity.guild_id != 0 && entity.guild_emblem_version != 0 {
                let block_center_y = (name_y + text_y) / 2.0;
                let emblem_x = leftmost_x - EMBLEM_HOVER_SIZE - 3.0;
                let emblem_y = block_center_y - EMBLEM_HOVER_SIZE / 2.0;
                push_emblem(
                    entity.guild_id,
                    entity.guild_emblem_version,
                    emblem_x,
                    emblem_y,
                    EMBLEM_HOVER_SIZE,
                    renderer,
                    calls,
                );
            }
        }
    }

    /// Guild emblems above the head of every guilded character and guardian mob
    /// (castle guardians, Emperium), shown only on WoE/siege maps.
    fn build_guild_emblems(&self, render_list: &[RenderEntry], calls: &mut Vec<UiDrawCall>) {
        if !self.game.session.map_properties.is_siege() {
            return;
        }
        let Some(renderer) = &self.renderer else {
            return;
        };
        for entry in render_list {
            let Some(entity) = self.game.world.entities.get(entry.id) else {
                continue;
            };
            if entity.guild_id == 0
                || entity.guild_emblem_version == 0
                || ragnarok_game::sprite_path::is_hidden(entity.effect_state)
            {
                continue;
            }
            let head_top = entry.screen_anchor[1] - entry.head_offset;
            let emblem_x = entry.screen_anchor[0] - EMBLEM_HEAD_SIZE / 2.0;
            let emblem_y = head_top - EMBLEM_HEAD_SIZE - 4.0;
            push_emblem(
                entity.guild_id,
                entity.guild_emblem_version,
                emblem_x,
                emblem_y,
                EMBLEM_HEAD_SIZE,
                renderer,
                calls,
            );
        }
    }

    fn build_player_bars(
        &self,
        hovered_entity_id: Option<u32>,
        render_list: &[RenderEntry],
        calls: &mut Vec<UiDrawCall>,
    ) {
        if self.renderer.is_none() || self.game.world.entities.player().is_none() {
            return;
        }
        if hovered_entity_id == self.game.world.entities.player_id() {
            return;
        }
        let ratio = self.game.character.hp_percentage();
        if let Some(entry) = render_list
            .iter()
            .find(|e| Some(e.id) == self.game.world.entities.player_id())
        {
            let (_x, y) = render_hp_bar(entry, ratio, EntityType::Player, calls);
            render_bar(
                entry.screen_anchor[0],
                y + HP_BAR_HEIGHT,
                self.game.character.sp_percentage(),
                SP_BAR_COLOR,
                calls,
            );
        }
    }

    /// HP ratio for an entity, sourcing companion HP from companion state (which is not
    /// mirrored onto `Entity.hp`), monster HP from `Entity.mob_info` and party-member
    /// HP from `Entity.hp`.
    fn entity_hp_ratio(&self, entity_id: u32) -> Option<f32> {
        if self.game.world.entities.is_player(entity_id) {
            return Some(self.game.character.hp_percentage());
        }
        if let Some(h) = self
            .game
            .companions
            .homunculus
            .as_ref()
            .filter(|h| h.gid == entity_id)
        {
            return Some(h.hp_percentage());
        }
        if let Some(m) = self
            .game
            .companions
            .mercenary
            .as_ref()
            .filter(|m| m.gid == entity_id)
        {
            return Some(m.hp_percentage());
        }
        self.game.world.entities.get(entity_id).and_then(|e| {
            e.mob_info
                .as_ref()
                .and_then(|info| info.hp_ratio())
                .or_else(|| e.hp_percentage())
        })
    }

    /// SP ratio for an entity that shows an SP bar below it: the player and the
    /// player's companions. Other entities (party members, monsters) return None.
    fn entity_sp_ratio(&self, entity_id: u32) -> Option<f32> {
        if self.game.world.entities.is_player(entity_id) {
            return Some(self.game.character.sp_percentage());
        }
        if let Some(h) = self
            .game
            .companions
            .homunculus
            .as_ref()
            .filter(|h| h.gid == entity_id)
        {
            return Some(h.sp_percentage());
        }
        if let Some(m) = self
            .game
            .companions
            .mercenary
            .as_ref()
            .filter(|m| m.gid == entity_id)
        {
            return Some(m.sp_percentage());
        }
        None
    }

    /// Always-on HP bars below party members and the player's mercenary/homunculus.
    /// The player is handled by `build_player_bars`; the hovered entity by the hover
    /// overlay — both are skipped here to avoid drawing twice.
    fn build_persistent_bars(
        &self,
        hovered_entity_id: Option<u32>,
        render_list: &[RenderEntry],
        calls: &mut Vec<UiDrawCall>,
    ) {
        if self.renderer.is_none() {
            return;
        }
        let player_id = self.game.world.entities.player_id();
        for entry in render_list {
            if Some(entry.id) == player_id || Some(entry.id) == hovered_entity_id {
                continue;
            }
            let is_companion = self
                .game
                .companions
                .homunculus
                .as_ref()
                .is_some_and(|h| h.gid == entry.id)
                || self
                    .game
                    .companions
                    .mercenary
                    .as_ref()
                    .is_some_and(|m| m.gid == entry.id);
            let is_party_member = self
                .game
                .party
                .as_ref()
                .is_some_and(|p| p.members.iter().any(|m| m.aid == entry.id));
            if !is_companion && !is_party_member {
                continue;
            }
            let Some(entity) = self.game.world.entities.get(entry.id) else {
                continue;
            };
            if entity.effect_state & ragnarok_game::sprite_path::OPTION_HIDE != 0 {
                continue;
            }
            let Some(ratio) = self.entity_hp_ratio(entry.id) else {
                continue;
            };
            let (_x, y) = render_hp_bar(entry, ratio, entity.entity_type, calls);
            if let Some(sp_ratio) = self.entity_sp_ratio(entry.id) {
                render_bar(
                    entry.screen_anchor[0],
                    y + HP_BAR_HEIGHT,
                    sp_ratio,
                    SP_BAR_COLOR,
                    calls,
                );
            }
        }
    }

    fn build_cast_bars(&self, render_list: &[RenderEntry], calls: &mut Vec<UiDrawCall>) {
        use models::enums::skill_enums::SkillEnum;
        use ragnarok_game::effect::casting_skill;
        if let Some(bar) = &self.game.session.progress_bar
            && let Some(entry) = render_list
                .iter()
                .find(|e| self.game.world.entities.is_player(e.id))
        {
            render_bar(
                entry.screen_anchor[0],
                entry.screen_anchor[1] - entry.head_offset - HP_BAR_HEIGHT - 2.0,
                bar.fraction(),
                CAST_BAR_COLOR,
                calls,
            );
        }
        for entry in render_list {
            if (self.config.display.show_other_cast_bars
                || self.game.world.entities.is_player(entry.id))
                && let Some(entity) = self.game.world.entities.get(entry.id)
                && entity.state == EntityState::Casting
                && entity.cast_total_duration > 0.0
                && !entity
                    .active_skill_id
                    .is_some_and(|id| casting_skill(SkillEnum::from_id(id as u32)).hide_cast_bar)
            {
                let progress = 1.0 - (entity.state_timer / entity.cast_total_duration);
                let cast_bar_y = entry.screen_anchor[1] - entry.head_offset - HP_BAR_HEIGHT - 2.0;
                render_bar(
                    entry.screen_anchor[0],
                    cast_bar_y,
                    progress,
                    CAST_BAR_COLOR,
                    calls,
                );
            }
        }
    }

    fn name_hidden(&self, entity_type: EntityType) -> bool {
        let display = &self.config.display;
        match entity_type {
            EntityType::Player => {
                display.hide_name_player || self.game.session.map_properties.is_siege()
            }
            EntityType::Monster => display.hide_name_monster,
            EntityType::Npc => display.hide_name_npc,
            EntityType::Homunculus | EntityType::Mercenary => false,
        }
    }

    fn build_chat_bubbles(&self, render_list: &[RenderEntry], calls: &mut Vec<UiDrawCall>) {
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };
        for entry in render_list {
            let entity = match self.game.world.entities.get(entry.id) {
                Some(e) => e,
                None => continue,
            };
            let bubble = match &entity.chat_bubble {
                Some(b) => b,
                None => continue,
            };

            let padding = 4.0;
            let lines = ragnarok_ui::draw::word_wrap(
                &bubble.message,
                150.0,
                |t| renderer.font_atlas.measure_text(t),
                false,
            );

            let line_h = renderer.font_atlas.line_height;
            let total_h = line_h * lines.len() as f32 + padding * 2.0;
            let widest = lines
                .iter()
                .map(|l| renderer.font_atlas.measure_text(l))
                .fold(0.0_f32, f32::max);
            let box_w = widest + padding * 2.0;
            let box_x = entry.screen_anchor[0] - box_w / 2.0;
            let box_y = entry.screen_anchor[1] - entry.head_offset - 5.0 - total_h;

            let (bg_verts, bg_idx) = ragnarok_ui::draw::quad_vertices(
                box_x,
                box_y,
                box_w,
                total_h,
                [0.0, 0.0, 0.0, 0.8],
            );
            calls.push(UiDrawCall {
                vertices: bg_verts.to_vec(),
                indices: bg_idx.to_vec(),
                texture: UiTextureRef::White,
            });

            for (i, line) in lines.iter().enumerate() {
                let line_w = renderer.font_atlas.measure_text(line);
                let lx = entry.screen_anchor[0] - line_w / 2.0;
                let ly = box_y + padding + line_h / 2.0 + line_h * i as f32;
                let (verts, indices) = ragnarok_ui::draw::text_vertices(
                    line,
                    lx,
                    ly,
                    [1.0, 1.0, 1.0, 1.0],
                    &renderer.font_atlas,
                );
                if !verts.is_empty() {
                    calls.push(UiDrawCall {
                        vertices: verts,
                        indices,
                        texture: UiTextureRef::FontAtlas,
                    });
                }
            }
        }
    }

    fn build_vending_boards(&self, render_list: &[RenderEntry], calls: &mut Vec<UiDrawCall>) {
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };
        for entry in render_list {
            let entity = match self.game.world.entities.get(entry.id) {
                Some(e) => e,
                None => continue,
            };
            let board = match &entity.vending_board {
                Some(b) => b,
                None => continue,
            };

            let has_grf = renderer
                .texture_cache
                .texture_size(vending_board::VENDING_ICON_TEX)
                .is_some();
            let mut container = DialogContainer::new();
            container.has_grf_textures = has_grf;
            container.set_texture_sizes(&|name| renderer.texture_cache.texture_size(name));

            vending_board::draw_board(
                calls,
                &container,
                &renderer.font_atlas,
                entry.screen_anchor[0],
                entry.screen_anchor[1],
                entry.head_offset,
                board,
            );
        }
    }

    fn build_chat_room_boards(&self, render_list: &[RenderEntry], calls: &mut Vec<UiDrawCall>) {
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };
        for room in self.game.chat_rooms.iter() {
            let entry = match render_list.iter().find(|e| e.id == room.owner_aid) {
                Some(e) => e,
                None => continue,
            };

            let has_grf = renderer
                .texture_cache
                .texture_size(chat_room_board::CHAT_OPEN_TEX)
                .is_some();
            let mut container = DialogContainer::new();
            container.has_grf_textures = has_grf;
            container.set_texture_sizes(&|name| renderer.texture_cache.texture_size(name));

            chat_room_board::draw_board(
                calls,
                &container,
                &renderer.font_atlas,
                entry.screen_anchor[0],
                entry.screen_anchor[1],
                entry.head_offset,
                room.atype,
                &room.title,
                room.cur_count,
                room.max_count,
            );
        }
    }

    fn build_floor_item_tooltip(
        &self,
        hovered_floor_item_id: Option<u32>,
        floor_item_render_list: &[RenderEntry],
        calls: &mut Vec<UiDrawCall>,
    ) {
        let fi_id = match hovered_floor_item_id {
            Some(id) => id,
            None => return,
        };
        let floor_item = match self.game.world.floor_items.get(&fi_id) {
            Some(fi) => fi,
            None => return,
        };
        let fi_entry = match floor_item_render_list.iter().find(|e| e.id == fi_id) {
            Some(e) => e,
            None => return,
        };
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };

        let tooltip = if floor_item.count > 1 {
            format!("{} : {} ea.", floor_item.name, floor_item.count)
        } else {
            floor_item.name.clone()
        };
        let text_w = renderer.font_atlas.measure_text(&tooltip);
        let text_x = fi_entry.screen_anchor[0] - text_w / 2.0;
        let text_y = fi_entry.pick_bounds[1] - 5.0;
        let padding = 3.0;

        let (bg_v, bg_i) = ragnarok_ui::draw::quad_vertices(
            text_x - padding,
            text_y - padding - 12.0,
            text_w + padding * 2.0,
            12.0 + padding * 2.0,
            [0.0, 0.0, 0.0, 0.85],
        );
        calls.push(UiDrawCall {
            vertices: bg_v.to_vec(),
            indices: bg_i.to_vec(),
            texture: UiTextureRef::White,
        });

        let (verts, indices) = ragnarok_ui::draw::text_vertices(
            &tooltip,
            text_x,
            text_y,
            [1.0, 1.0, 1.0, 1.0],
            &renderer.font_atlas,
        );
        if !verts.is_empty() {
            calls.push(UiDrawCall {
                vertices: verts,
                indices,
                texture: UiTextureRef::FontAtlas,
            });
        }
    }

    fn build_debug_pick_bounds(
        &self,
        render_list: &[RenderEntry],
        floor_item_render_list: &[RenderEntry],
        calls: &mut Vec<UiDrawCall>,
    ) {
        if !self.game.debug_show_pick_bounds {
            return;
        }
        let debug_color = [1.0, 0.0, 0.0, 0.7];
        let line_thickness = 1.0;
        for entry in render_list.iter().chain(floor_item_render_list.iter()) {
            let [left, top, right, bottom] = entry.pick_bounds;
            let w = right - left;
            let h = bottom - top;
            for (x, y, bw, bh) in [
                (left, top, w, line_thickness),
                (left, bottom - line_thickness, w, line_thickness),
                (left, top, line_thickness, h),
                (right - line_thickness, top, line_thickness, h),
            ] {
                let (v, i) = ragnarok_ui::draw::quad_vertices(x, y, bw, bh, debug_color);
                calls.push(UiDrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: UiTextureRef::White,
                });
            }
            let dot = 3.0;
            let (v, i) = ragnarok_ui::draw::quad_vertices(
                entry.screen_anchor[0] - dot,
                entry.screen_anchor[1] - dot,
                dot * 2.0,
                dot * 2.0,
                debug_color,
            );
            calls.push(UiDrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: UiTextureRef::White,
            });
        }
    }

    pub(crate) fn build_skill_overlay(&self) -> Vec<UiDrawCall> {
        let (pending, renderer) = match (
            &self.game.pending_casts.pending_skill_target,
            &self.renderer,
        ) {
            (Some(p), Some(r)) => (p, r),
            _ => return Vec::new(),
        };

        let mut calls = Vec::new();
        let (mx, my) = self.input.mouse_position;
        let text = format!("Lv {}", pending.level());
        let text_x = mx as f32 + 20.0;
        let text_y = my as f32 + 2.0;
        build_outlined_text(
            &text,
            text_x,
            text_y,
            [1.0, 1.0, 1.0, 1.0],
            &renderer.font_atlas,
            &mut calls,
        );

        let skill_id = pending.skill_id();
        if let Some(skill_data) = self.game.character.skills.get_skill(skill_id) {
            let display_name = self
                .game
                .data_table
                .skill_name
                .as_ref()
                .map(|t| t.get_display_name_or_internal(&skill_data.name))
                .unwrap_or_else(|| skill_data.name.clone());
            let level = pending.level();
            let banner_text = if level > 0 {
                format!("{}(Lv {})", display_name, level)
            } else {
                display_name
            };
            let padding = 4.0;
            let line_h = renderer.font_atlas.line_height;
            let text_w = renderer.font_atlas.measure_text(&banner_text);
            let box_w = text_w + padding * 2.0;
            let box_h = line_h + padding * 2.0;
            let screen_w = renderer.device.surface_config.width as f32 / renderer.dpi_scale;
            let box_x = ((screen_w - box_w) / 2.0).floor();
            let box_y = 80.0;

            let (bg_verts, bg_idx) =
                ragnarok_ui::draw::quad_vertices(box_x, box_y, box_w, box_h, [0.0, 0.0, 0.0, 0.8]);
            calls.push(UiDrawCall {
                vertices: bg_verts.to_vec(),
                indices: bg_idx.to_vec(),
                texture: UiTextureRef::White,
            });

            let tx = box_x + padding;
            let ty = box_y + padding + line_h / 2.0;
            let (verts, indices) = ragnarok_ui::draw::text_vertices(
                &banner_text,
                tx,
                ty,
                [0.0, 1.0, 0.0, 1.0],
                &renderer.font_atlas,
            );
            if !verts.is_empty() {
                calls.push(UiDrawCall {
                    vertices: verts,
                    indices,
                    texture: UiTextureRef::FontAtlas,
                });
            }
        }

        calls
    }
}

fn push_emblem(
    guild_id: u32,
    version: i32,
    x: f32,
    y: f32,
    size: f32,
    renderer: &ragnarok_renderer::Renderer,
    calls: &mut Vec<UiDrawCall>,
) {
    let key = ragnarok_game::guild::emblem_texture_key(guild_id, version);
    if renderer.texture_cache.texture_size(&key).is_none() {
        return;
    }
    let (verts, indices) = ragnarok_ui::draw::quad_vertices(x, y, size, size, [1.0; 4]);
    calls.push(UiDrawCall {
        vertices: verts.to_vec(),
        indices: indices.to_vec(),
        texture: UiTextureRef::Named(key),
    });
}

/// Text drawn under an entity's name: the guild tag, or the monster info the server
/// sends when `show_mob_info` is on. A monster never has both.
fn second_plate_line(entity: &Entity) -> Option<(String, [f32; 4])> {
    if let Some(guild_name) = &entity.guild_name {
        let text = match &entity.position_name {
            Some(pos) if !pos.is_empty() => format!("<{guild_name}> [{pos}]"),
            _ => format!("<{guild_name}>"),
        };
        let color = if entity.is_gm {
            GM_TEXT_COLOR
        } else {
            GUILD_NAME_COLOR
        };
        return Some((text, color));
    }
    entity
        .mob_info
        .as_ref()
        .map(|info| (info.label(), MOB_INFO_COLOR))
}

fn entity_name_color(entity: &Entity) -> [f32; 4] {
    if let Some(color) = pk_name_color(entity.effect_state) {
        return color;
    }
    if entity.is_gm {
        return GM_TEXT_COLOR;
    }
    match entity.entity_type {
        EntityType::Player | EntityType::Homunculus | EntityType::Mercenary => [1.0, 1.0, 1.0, 1.0],
        EntityType::Monster => [1.0, 0.776, 0.776, 1.0],
        EntityType::Npc => [0.39, 0.54, 0.76, 1.0],
    }
}

fn hp_bar_color(ratio: f32, entity_type: EntityType) -> [f32; 4] {
    match entity_type {
        EntityType::Monster => {
            if ratio >= 0.25 {
                [1.0, 0.0, 0.906, 1.0]
            } else {
                [1.0, 1.0, 0.0, 1.0]
            }
        }
        _ => {
            if ratio >= 0.25 {
                [0.063, 0.937, 0.129, 1.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            }
        }
    }
}

fn render_bar(
    center_x: f32,
    y: f32,
    ratio: f32,
    fill_color: [f32; 4],
    draw_calls: &mut Vec<UiDrawCall>,
) {
    let border_x = center_x - HP_BAR_WIDTH / 2.0;
    let (border_verts, border_idx) = ragnarok_ui::draw::quad_vertices(
        border_x,
        y,
        HP_BAR_WIDTH,
        HP_BAR_HEIGHT,
        [0.063, 0.094, 0.612, 1.0],
    );
    draw_calls.push(UiDrawCall {
        vertices: border_verts.to_vec(),
        indices: border_idx.to_vec(),
        texture: UiTextureRef::White,
    });
    let (bg_verts, bg_idx) = ragnarok_ui::draw::quad_vertices(
        border_x + 1.0,
        y + 1.0,
        HP_BAR_WIDTH - 2.0,
        HP_BAR_HEIGHT - 2.0,
        [0.259, 0.259, 0.259, 1.0],
    );
    draw_calls.push(UiDrawCall {
        vertices: bg_verts.to_vec(),
        indices: bg_idx.to_vec(),
        texture: UiTextureRef::White,
    });
    let fill_ratio = ratio.clamp(0.0, 1.0);
    let fill_w = (HP_BAR_WIDTH - 2.0) * fill_ratio;
    let (fill_verts, fill_idx) = ragnarok_ui::draw::quad_vertices(
        border_x + 1.0,
        y + 1.0,
        fill_w,
        HP_BAR_HEIGHT - 2.0,
        fill_color,
    );
    draw_calls.push(UiDrawCall {
        vertices: fill_verts.to_vec(),
        indices: fill_idx.to_vec(),
        texture: UiTextureRef::White,
    });
}

fn render_hp_bar(
    entry: &RenderEntry,
    ratio: f32,
    entity_type: EntityType,
    draw_calls: &mut Vec<UiDrawCall>,
) -> (f32, f32) {
    let center_x = entry.screen_anchor[0];
    let y = entry.pick_bounds[3];
    render_bar(
        center_x,
        y,
        ratio,
        hp_bar_color(ratio, entity_type),
        draw_calls,
    );
    (center_x, y)
}

fn build_outlined_text(
    text: &str,
    x: f32,
    y: f32,
    color: [f32; 4],
    font_atlas: &ragnarok_renderer::FontAtlas,
    calls: &mut Vec<UiDrawCall>,
) {
    let outline_color = [0.0, 0.0, 0.0, 1.0];
    for &(dx, dy) in &[(-1.0_f32, 0.0_f32), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        let (verts, indices) =
            ragnarok_ui::draw::text_vertices(text, x + dx, y + dy, outline_color, font_atlas);
        if !verts.is_empty() {
            calls.push(UiDrawCall {
                vertices: verts,
                indices,
                texture: UiTextureRef::FontAtlas,
            });
        }
    }
    let (verts, indices) = ragnarok_ui::draw::text_vertices(text, x, y, color, font_atlas);
    if !verts.is_empty() {
        calls.push(UiDrawCall {
            vertices: verts,
            indices,
            texture: UiTextureRef::FontAtlas,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::targeting::EFFECT_STATE_RED_NAME;

    fn player() -> Entity {
        Entity::new(
            1,
            EntityType::Player,
            0,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            150,
        )
    }

    #[test]
    fn gm_name_is_yellow_but_pk_state_wins() {
        let mut e = player();
        assert_eq!(entity_name_color(&e), [1.0, 1.0, 1.0, 1.0]);
        e.is_gm = true;
        assert_eq!(entity_name_color(&e), GM_TEXT_COLOR);
        e.effect_state = EFFECT_STATE_RED_NAME;
        assert_eq!(entity_name_color(&e), [1.0, 0.0, 0.0, 1.0]);
    }
}
