use crate::App;
use crate::game_state::HoverState;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::{
    PendingSkillTarget, RenderEntry, cursor_type_for_cell, hovered_entity_cursor_type,
};
use ragnarok_game::targeting::{TargetClass, skill_target_class};

impl App {
    pub(crate) fn update_grid_hover(&mut self) -> Option<(i32, i32)> {
        let hovered = if self.game.session.app_state == AppState::InGame {
            self.hovered_cell()
        } else {
            None
        };

        let hover_corners = hovered.and_then(|(cx, cy)| {
            let coords = self.game.session.map_coords.as_ref()?;
            let gat = self.game.session.gat.as_ref()?;
            Some(coords.cell_corners_world(gat, cx, cy))
        });

        if let Some(renderer) = &mut self.renderer
            && let Some(grid) = &mut renderer.grid_selector
        {
            if let Some(corners) = hover_corners {
                grid.update_hover(&renderer.device.queue, corners);
                grid.set_hover_visible(true);
            } else {
                grid.set_hover_visible(false);
            }
        }

        hovered
    }

    fn hovered_vending_board(&self, render_list: &[RenderEntry]) -> Option<u32> {
        let (mx, my) = self.input.mouse_position;
        let (mx, my) = (mx as f32, my as f32);
        for entry in render_list {
            let is_vendor = self
                .game
                .world
                .entities
                .get(entry.id)
                .is_some_and(|e| e.vending_board.is_some());
            if !is_vendor {
                continue;
            }
            let r = crate::overlay::vending_board_rect(entry);
            if mx >= r[0] && mx <= r[2] && my >= r[1] && my <= r[3] {
                return Some(entry.id);
            }
        }
        None
    }

    fn hovered_chat_room(&self, render_list: &[RenderEntry]) -> Option<u32> {
        let (mx, my) = self.input.mouse_position;
        let (mx, my) = (mx as f32, my as f32);
        for room in self.game.chat_rooms.iter() {
            let entry = match render_list.iter().find(|e| e.id == room.owner_aid) {
                Some(e) => e,
                None => continue,
            };
            let r = crate::overlay::chat_room_board_rect(entry);
            if mx >= r[0] && mx <= r[2] && my >= r[1] && my <= r[3] {
                return Some(room.room_id);
            }
        }
        None
    }

    pub(crate) fn resolve_hover(
        &self,
        hovered_cell: Option<(i32, i32)>,
        render_list: &[RenderEntry],
        floor_item_render_list: &[RenderEntry],
        ui_any_hovered: bool,
        ui_any_interactive_hovered: bool,
    ) -> HoverState {
        let mut hover = HoverState::default();
        let mouse = self.input.mouse_position;
        let entities = &self.game.world.entities;
        let map = &self.game.session.map_properties;

        hover.hovered_player_id = ragnarok_game::cursor::hovered_player(mouse, entities, render_list);

        if let Some(gat) = &self.game.session.gat {
            hover.cell_cursor = cursor_type_for_cell(gat, hovered_cell);
        }

        let companion_target_armed =
            self.game.companions.companion_attack_target.iter().any(Option::is_some);
        let suppressed = self.input.right_mouse_down
            || ui_any_interactive_hovered
            || ui_any_hovered
            || companion_target_armed;

        if !suppressed {
            if let Some(pending) = &self.game.pending_casts.pending_companion_skill {
                if !pending.is_ground {
                    hover.hovered_entity_id =
                        hovered_entity_cursor_type(mouse, entities, render_list, map, None)
                            .map(|(_, id)| id);
                }
            } else if self.game.companions.capture_targeting {
                hover.hovered_entity_id = hovered_entity_cursor_type(
                    mouse,
                    entities,
                    render_list,
                    map,
                    Some(TargetClass::Offensive),
                )
                .map(|(_, id)| id);
            } else if let Some(PendingSkillTarget::Entity { skill_id, .. }) =
                &self.game.pending_casts.pending_skill_target
            {
                let class = self
                    .game
                    .character
                    .skills
                    .get_skill(*skill_id)
                    .map(|s| skill_target_class(s.skill_target_type))
                    .unwrap_or(TargetClass::Offensive);
                hover.hovered_entity_id =
                    hovered_entity_cursor_type(mouse, entities, render_list, map, Some(class))
                        .map(|(_, id)| id);
            } else if self.game.pending_casts.pending_skill_target.is_none() {
                if let Some(room_id) = self.hovered_chat_room(render_list) {
                    hover.hovered_chat_room = Some(room_id);
                } else if let Some(vendor_id) = self.hovered_vending_board(render_list) {
                    hover.hovered_vending = Some(vendor_id);
                } else if let Some((cursor, id)) =
                    hovered_entity_cursor_type(mouse, entities, render_list, map, None)
                {
                    hover.hovered_entity_id = Some(id);
                    hover.hovered_entity_cursor = Some(cursor);
                }
            }
        }

        if hover.target_id().is_none() && !ui_any_hovered && !self.input.right_mouse_down {
            let (mx, my) = (mouse.0 as f32, mouse.1 as f32);
            hover.hovered_floor_item_id = floor_item_render_list
                .iter()
                .find(|entry| {
                    mx >= entry.pick_bounds[0]
                        && mx <= entry.pick_bounds[2]
                        && my >= entry.pick_bounds[1]
                        && my <= entry.pick_bounds[3]
                })
                .map(|entry| entry.id);
        }

        hover
    }
}
