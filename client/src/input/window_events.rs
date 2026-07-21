use crate::App;
use crate::config::WindowStateEntry;
use ragnarok_game::app_state::AppState;
use ragnarok_game::entity::{EntityState, EntityType};
use ragnarok_game::event::GameEvent;
use ragnarok_game::keybinding::{HotkeyAction, KeyChord};
use ragnarok_network::build_action_request_packet;
use ragnarok_ui_component::game::context_menu::{ContextMenuAction, ContextMenuItem};
use std::collections::HashMap;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, Modifiers, MouseButton, MouseScrollDelta};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

impl App {
    pub(crate) fn capture_window_state(&mut self) {
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
    }

    pub(crate) fn handle_close_requested(&mut self, event_loop: &ActiveEventLoop) {
        self.capture_window_state();
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
                        self.input.right_press_entity = self.game.hover.hovered_player_id;
                        self.input.right_press_target = self.game.hover.hovered_entity_id;
                        self.game.pending_casts.pending_skill_target = None;
                        self.game.pending_casts.pending_skill_id = None;
                        self.game.pending_casts.pending_skill_level = None;
                        self.game.capture_targeting = false;
                        self.game.pet_roulette = None;
                    } else {
                        self.input.last_mouse_pos = None;
                        if !self.input.right_dragged && !self.input.ui_hovered {
                            if self.input.alt_pressed
                                && (self.has_homunculus() || self.has_mercenary())
                            {
                                self.issue_owner_command(
                                    self.has_mercenary(),
                                    self.input.right_press_target,
                                );
                            } else if !self.open_companion_context_menu(self.input.right_press_target)
                                && !self.open_pet_context_menu(self.input.right_press_target)
                            {
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
                        } else if self.input.alt_pressed
                            && (self.has_homunculus() || self.has_mercenary())
                        {
                            self.issue_owner_command(
                                self.has_mercenary(),
                                self.game.hover.hovered_entity_id,
                            );
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

    fn open_companion_context_menu(&mut self, entity_id: Option<u32>) -> bool {
        let Some(entity_id) = entity_id else {
            return false;
        };
        let is_homun = self
            .game
            .homunculus
            .as_ref()
            .is_some_and(|h| !h.vaporized && h.gid == entity_id);
        let is_merc = self
            .game
            .mercenary
            .as_ref()
            .is_some_and(|m| m.gid == entity_id);
        if !is_homun && !is_merc {
            return false;
        }
        let (mx, my) = self.input.mouse_position;
        let items = if is_homun {
            vec![
                ContextMenuItem {
                    label: "Homunculus Info".to_string(),
                    action: ContextMenuAction::CompanionShowInfo { is_mercenary: false },
                },
                ContextMenuItem {
                    label: "Feed".to_string(),
                    action: ContextMenuAction::CompanionFeed,
                },
                ContextMenuItem {
                    label: "Standby".to_string(),
                    action: ContextMenuAction::CompanionStandby { is_mercenary: false },
                },
                ContextMenuItem {
                    label: "AI Settings".to_string(),
                    action: ContextMenuAction::CompanionAiConfig,
                },
            ]
        } else {
            vec![
                ContextMenuItem {
                    label: "Mercenary Info".to_string(),
                    action: ContextMenuAction::CompanionShowInfo { is_mercenary: true },
                },
                ContextMenuItem {
                    label: "Standby".to_string(),
                    action: ContextMenuAction::CompanionStandby { is_mercenary: true },
                },
                ContextMenuItem {
                    label: "AI Settings".to_string(),
                    action: ContextMenuAction::CompanionAiConfig,
                },
            ]
        };
        self.game.context_menu.open_at(mx as f32, my as f32, items);
        true
    }

    fn open_pet_context_menu(&mut self, entity_id: Option<u32>) -> bool {
        let Some(entity_id) = entity_id else {
            return false;
        };
        if self.game.pet.gid != Some(entity_id) {
            return false;
        }
        if !self
            .game
            .entities
            .get(entity_id)
            .is_some_and(|e| e.is_pet)
        {
            return false;
        }
        let (mx, my) = self.input.mouse_position;
        let items = vec![
            ContextMenuItem {
                label: "Pet Information".to_string(),
                action: ContextMenuAction::PetShowInfo,
            },
            ContextMenuItem {
                label: "Feed".to_string(),
                action: ContextMenuAction::PetFeed,
            },
            ContextMenuItem {
                label: "Performance".to_string(),
                action: ContextMenuAction::PetCommand { csub: 2 },
            },
            ContextMenuItem {
                label: "Take off Accessory".to_string(),
                action: ContextMenuAction::PetCommand { csub: 4 },
            },
            ContextMenuItem {
                label: "Return to Egg".to_string(),
                action: ContextMenuAction::PetCommand { csub: 3 },
            },
        ];
        self.game.context_menu.open_at(mx as f32, my as f32, items);
        true
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
        // For players the on-screen unit id equals the account id, which is the invite key.
        let mut items = vec![
            ContextMenuItem {
                label: "Deal".to_string(),
                action: ContextMenuAction::RequestTrade {
                    target_aid: entity_id,
                },
            },
            ContextMenuItem {
                label: "Invite to Party".to_string(),
                action: ContextMenuAction::InviteToParty {
                    target_aid: entity_id,
                },
            },
        ];
        if matches!(entity.job, 0..=6 | 23) {
            items.push(ContextMenuItem {
                label: "Adopt as Baby".to_string(),
                action: ContextMenuAction::AdoptBaby {
                    target_aid: entity_id,
                },
            });
        }
        if let Some(g) = &self.game.guild {
            let local_gid = self
                .game
                .login_session
                .as_ref()
                .map(|s| s.account_id)
                .unwrap_or(0);
            let rights = g.my_rights(local_gid);
            if rights.can_invite {
                items.push(ContextMenuItem {
                    label: "Invite to Guild".to_string(),
                    action: ContextMenuAction::GuildInvite {
                        target_aid: entity_id,
                    },
                });
            }
            if g.is_master(local_gid) {
                items.push(ContextMenuItem {
                    label: "Request Alliance".to_string(),
                    action: ContextMenuAction::GuildAlly {
                        target_aid: entity_id,
                    },
                });
                items.push(ContextMenuItem {
                    label: "Declare Hostility".to_string(),
                    action: ContextMenuAction::GuildHostile {
                        target_aid: entity_id,
                    },
                });
            }
        }
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
                    renderer.camera.apply_drag(
                        dx,
                        dy,
                        self.config.free_camera,
                        self.game.camera_locked,
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
                renderer.camera.apply_zoom(scroll, self.game.camera_locked);
            }
        }
    }

    fn trigger_shortcut(&mut self, slot: usize) {
        if let Some(cmd) = self.config.shortcut_commands.get(slot).cloned()
            && !cmd.is_empty()
        {
            self.run_chat_command(&cmd);
        }
    }

    pub(crate) fn handle_keyboard_input(&mut self, event: KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        let code = match event.physical_key {
            PhysicalKey::Code(c) => Some(c),
            _ => None,
        };

        if pressed && self.game.hotkey_config_window.is_capturing() {
            if let Some(code) = code {
                self.capture_hotkey(code);
            }
            if let Some(ctx) = &mut self.ui_context {
                ctx.key_escape = false;
                ctx.typed_chars.clear();
            }
            return;
        }

        if !pressed || self.game.app_state != AppState::InGame {
            return;
        }
        let Some(code) = code else {
            return;
        };
        let chord = KeyChord::new(
            format!("{code:?}"),
            self.input.alt_pressed,
            self.input.ctrl_pressed,
            self.input.shift_pressed,
        );
        let action = self.config.keybindings.action_for(&chord);

        // Minimap keeps its pre-gate slot: it cycles even while chatting or with
        // the system menu open.
        if action == Some(HotkeyAction::CycleMinimap) {
            self.game.minimap_window.cycle_visibility();
            return;
        }

        if self.game.chat_window.is_active() || self.game.system_menu.open {
            return;
        }

        match code {
            KeyCode::F11 => {
                if let Some(renderer) = &mut self.renderer
                    && let Some(grid) = &mut renderer.grid_selector
                {
                    grid.show_grid = !grid.show_grid;
                }
                self.game.debug_show_pick_bounds = !self.game.debug_show_pick_bounds;
            }
            KeyCode::F10 => {
                self.game.debug_overlay = !self.game.debug_overlay;
            }
            KeyCode::Digit1 if self.input.alt_pressed => self.trigger_shortcut(0),
            KeyCode::Digit2 if self.input.alt_pressed => self.trigger_shortcut(1),
            KeyCode::Digit3 if self.input.alt_pressed => self.trigger_shortcut(2),
            KeyCode::Digit4 if self.input.alt_pressed => self.trigger_shortcut(3),
            KeyCode::Digit5 if self.input.alt_pressed => self.trigger_shortcut(4),
            KeyCode::Digit6 if self.input.alt_pressed => self.trigger_shortcut(5),
            KeyCode::Digit7 if self.input.alt_pressed => self.trigger_shortcut(6),
            KeyCode::Digit8 if self.input.alt_pressed => self.trigger_shortcut(7),
            KeyCode::Digit9 if self.input.alt_pressed => self.trigger_shortcut(8),
            KeyCode::Digit0 if self.input.alt_pressed => self.trigger_shortcut(9),
            _ => {
                if let Some(action) = action {
                    self.dispatch_action(action);
                } else if let Some(emote_type) = self.config.emotion_keys.emote_for(&chord) {
                    self.pending_events
                        .push(GameEvent::RequestEmotion { emote_type });
                }
            }
        }
    }

    fn dispatch_action(&mut self, action: HotkeyAction) {
        match action {
            HotkeyAction::ToggleInventory => self.game.character.inventory.toggle(),
            HotkeyAction::ToggleEquipment => self.game.equipment_window.toggle(),
            HotkeyAction::ToggleSkillTree => self.game.character.skills.toggle(),
            HotkeyAction::ToggleStatus => self.game.status_window.toggle(),
            HotkeyAction::ToggleShortcutList => {
                if !self.game.shortcut_list_window.is_open() {
                    self.game
                        .shortcut_list_window
                        .set_bindings(&self.config.shortcut_commands);
                }
                self.game.shortcut_list_window.toggle();
            }
            HotkeyAction::ToggleEmotion => self.game.emotion_window.toggle(),
            HotkeyAction::ToggleQuest => self.game.quest_window.toggle(),
            HotkeyAction::ToggleCart => {
                let has_cart = self
                    .game
                    .entities
                    .player()
                    .is_some_and(|p| p.cart_type.is_some());
                if has_cart {
                    self.game.character.cart.toggle();
                }
            }
            HotkeyAction::ToggleGuild => {
                if self.game.guild.is_some() {
                    self.game.guild_window.toggle();
                } else {
                    self.game
                        .chat_window
                        .add_system("You are not in a guild.".to_string());
                }
            }
            HotkeyAction::ToggleChatRoomCreate => self.game.chat_room_create_window.toggle(),
            HotkeyAction::ToggleBasicInfo => self.game.basic_info_window.toggle(),
            HotkeyAction::ToggleParty => self.game.party_friends_window.open_party_tab(),
            HotkeyAction::ToggleFriends => self.game.party_friends_window.open_friend_tab(),
            HotkeyAction::TogglePet => {
                if self.game.pet.gid.is_some() {
                    self.game.pet_window.toggle();
                }
            }
            HotkeyAction::ToggleSoundOptions => self.open_sound_options(),
            HotkeyAction::ToggleGraphicOptions => self.open_graphic_options(),
            HotkeyAction::ToggleHomunculus => {
                if self.game.homunculus.is_some() {
                    self.game.homunculus_window.toggle();
                }
            }
            HotkeyAction::ToggleMercenary => {
                if self.game.mercenary.is_some() {
                    self.game.mercenary_window.toggle();
                }
            }
            HotkeyAction::SitStand => {
                if self.player_hidden() {
                    return;
                }
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
            HotkeyAction::CycleMinimap => self.game.minimap_window.cycle_visibility(),
            HotkeyAction::MercenaryFollow => {
                if self.has_mercenary() {
                    self.push_owner_command_to(
                        true,
                        ragnarok_game::companion::OwnerCommand::follow(),
                        false,
                    );
                }
            }
        }
    }

    fn capture_hotkey(&mut self, code: KeyCode) {
        if code == KeyCode::Escape {
            self.game.hotkey_config_window.cancel_capture();
            return;
        }
        let name = format!("{code:?}");
        if ragnarok_game::keybinding::is_modifier_key(&name) {
            return;
        }
        let chord = KeyChord::new(
            name,
            self.input.alt_pressed,
            self.input.ctrl_pressed,
            self.input.shift_pressed,
        );
        self.game.hotkey_config_window.capture_key(chord);
    }

    pub(crate) fn handle_modifiers_changed(&mut self, modifiers: Modifiers) {
        self.input.alt_pressed = modifiers.state().alt_key();
        self.input.shift_pressed = modifiers.state().shift_key();
        self.input.ctrl_pressed = modifiers.state().control_key();
    }
}
