use crate::App;
use crate::config::WindowStateEntry;
use ragnarok_game::app_state::AppState;
use ragnarok_game::entity::{EntityState, EntityType};
use ragnarok_network::build_action_request_packet;
use ragnarok_ui_component::game::context_menu::{ContextMenuAction, ContextMenuItem};
use std::collections::HashMap;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, Modifiers, MouseButton, MouseScrollDelta};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

impl App {
    pub(crate) fn handle_close_requested(&mut self, event_loop: &ActiveEventLoop) {
        let positions = self.ui_state_cache.extract_window_positions();
        let open_collapsed = self.game.extract_window_state(&self.ui_state_cache);
        let mut window_state = HashMap::new();
        for (id, pos) in &positions {
            let (open, collapsed) = open_collapsed.get(id).copied().unwrap_or((false, false));
            window_state.insert(
                *id,
                WindowStateEntry {
                    position: *pos,
                    open,
                    collapsed,
                },
            );
        }
        self.config.window_state = window_state;
        self.config.hotkey_visible_rows = self.game.character.hotkeys.visible_rows();
        self.config.battle_mode = self.game.character.hotkeys.battle_mode();
        self.config.save("config.json");
        event_loop.exit();
    }

    pub(crate) fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(size.width, size.height);
        }
    }

    pub(crate) fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if self.game.app_state == AppState::InGame {
            match button {
                MouseButton::Right => {
                    self.input.right_mouse_down = state == ElementState::Pressed;
                    if self.input.right_mouse_down {
                        self.input.right_dragged = false;
                        // Picking clears the hovered ids while the button is held (rotate cursor),
                        // so capture the target now, before it's lost.
                        self.input.right_press_entity = self.game.hovered_player_id;
                        self.input.right_press_target = self.game.hovered_entity_id;
                        self.game.pending_skill_target = None;
                        self.game.pending_skill_id = None;
                        self.game.pending_skill_level = None;
                    } else {
                        self.input.last_mouse_pos = None;
                        if !self.input.right_dragged && !self.input.ui_hovered {
                            if self.input.alt_pressed && self.has_companion() {
                                self.issue_owner_command();
                            } else {
                                self.open_entity_context_menu(self.input.right_press_entity);
                            }
                        }
                        self.input.right_press_entity = None;
                        self.input.right_press_target = None;
                    }
                }
                MouseButton::Left => {
                    let pressed = state == ElementState::Pressed;
                    self.input.left_mouse_down = pressed;
                    if pressed {
                        if self.input.ui_hovered {
                            self.input.ui_dragging = true;
                        } else {
                            self.handle_left_click();
                            self.input.walk_packet_cooldown = 0.5;
                            self.input.walk_server_acked = false;
                        }
                    } else {
                        self.input.ui_dragging = false;
                    }
                }
                _ => {}
            }
        }
    }

    fn open_entity_context_menu(&mut self, entity_id: Option<u32>) {
        let Some(entity_id) = entity_id else {
            return;
        };
        if Some(entity_id) == self.game.entities.player_id() {
            return;
        }
        let Some(entity) = self.game.entities.get(entity_id) else {
            return;
        };
        if entity.entity_type != EntityType::Player {
            return;
        }
        let (mx, my) = self.input.mouse_position;
        // For players the on-screen unit id equals the account id, which is the party invite key.
        let items = vec![ContextMenuItem {
            label: "Invite to Party".to_string(),
            action: ContextMenuAction::InviteToParty {
                target_aid: entity_id,
            },
        }];
        self.game
            .context_menu
            .open_at(mx as f32, my as f32, items);
    }

    pub(crate) fn handle_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let dpi = self.renderer.as_ref().map_or(1.0, |r| r.dpi_scale) as f64;
        let logical_pos = (position.x / dpi, position.y / dpi);
        self.input.mouse_position = logical_pos;
        if self.game.app_state == AppState::InGame && self.input.right_mouse_down {
            if let Some((lx, ly)) = self.input.last_mouse_pos {
                let dx = (logical_pos.0 - lx) as f32;
                let dy = (logical_pos.1 - ly) as f32;
                if dx.abs() > 1.0 || dy.abs() > 1.0 {
                    self.input.right_dragged = true;
                }
                if let Some(renderer) = &mut self.renderer {
                    super::handle_camera_drag(
                        &mut renderer.camera,
                        dx,
                        dy,
                        self.config.free_camera,
                    );
                }
            }
            self.input.last_mouse_pos = Some(logical_pos);
        }
    }

    pub(crate) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if self.game.app_state == AppState::InGame && !self.input.ui_hovered {
            let scroll = match delta {
                MouseScrollDelta::LineDelta(_, y) => y,
                MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
            };
            if let Some(renderer) = &mut self.renderer {
                super::handle_camera_zoom(&mut renderer.camera, scroll);
            }
        }
    }

    pub(crate) fn handle_keyboard_input(&mut self, event: KeyEvent) {
        if event.state == ElementState::Pressed
            && self.game.app_state == AppState::InGame
            && event.physical_key == PhysicalKey::Code(KeyCode::Tab)
            && self.input.ctrl_pressed
        {
            self.game.minimap_window.cycle_visibility();
            return;
        }

        if event.state == ElementState::Pressed
            && self.game.app_state == AppState::InGame
            && !self.game.chat_window.is_active()
            && !self.game.system_menu.open
        {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::F11) => {
                    if let Some(renderer) = &mut self.renderer
                        && let Some(grid) = &mut renderer.grid_selector
                    {
                        grid.show_grid = !grid.show_grid;
                    }
                    self.game.debug_show_pick_bounds = !self.game.debug_show_pick_bounds;
                }
                PhysicalKey::Code(KeyCode::F10) => {
                    self.game.debug_overlay = !self.game.debug_overlay;
                }
                PhysicalKey::Code(KeyCode::Insert) => {
                    if let Some(entity) = self.game.entities.player() {
                        let action = if entity.state == EntityState::Sitting {
                            3u8
                        } else {
                            2u8
                        };
                        self.channel.send_packet(build_action_request_packet(
                            0,
                            action,
                            self.config.packetver,
                        ));
                    }
                }
                PhysicalKey::Code(KeyCode::KeyE) if self.input.alt_pressed => {
                    self.game.character.inventory.toggle();
                }
                PhysicalKey::Code(KeyCode::KeyQ) if self.input.alt_pressed => {
                    self.game.equipment_window.toggle();
                }
                PhysicalKey::Code(KeyCode::KeyS) if self.input.alt_pressed => {
                    self.game.character.skills.toggle();
                }
                PhysicalKey::Code(KeyCode::KeyA) if self.input.alt_pressed => {
                    self.game.status_window.toggle();
                }
                PhysicalKey::Code(KeyCode::KeyW) if self.input.alt_pressed => {
                    let has_cart = self
                        .game
                        .entities
                        .player()
                        .is_some_and(|p| p.cart_type.is_some());
                    if has_cart {
                        self.game.character.cart.toggle();
                    }
                }
                PhysicalKey::Code(KeyCode::KeyH) if self.input.alt_pressed => {
                    if self.game.homunculus.is_some() {
                        self.game.homunculus_window.toggle();
                    }
                }
                PhysicalKey::Code(KeyCode::KeyR) if self.input.alt_pressed => {
                    if self.game.mercenary.is_some() {
                        self.game.mercenary_window.toggle();
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn handle_modifiers_changed(&mut self, modifiers: Modifiers) {
        self.input.alt_pressed = modifiers.state().alt_key();
        self.input.shift_pressed = modifiers.state().shift_key();
        self.input.ctrl_pressed = modifiers.state().control_key();
    }
}
