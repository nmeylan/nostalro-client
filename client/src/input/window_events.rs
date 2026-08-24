use crate::App;
use crate::config::WindowStateEntry;
use models::enums::EnumWithNumberValue;
use models::enums::class::JobName;
use models::enums::skill_enums::SkillEnum;
use ragnarok_game::ailment::OPT2_BLIND;
use ragnarok_game::app_state::AppState;
use ragnarok_game::entity::{EntityState, EntityType};
use ragnarok_game::event::GameEvent;
use ragnarok_game::keybinding::{HotkeyAction, KeyChord};
use ragnarok_network::build_action_request_packet;
use ragnarok_renderer::camera::CameraControl;
use ragnarok_ui::context::{DOUBLE_CLICK_DISTANCE, DOUBLE_CLICK_THRESHOLD_MS};
use ragnarok_ui_component::game::context_menu::{ContextMenuAction, ContextMenuItem};
use std::collections::HashMap;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, Modifiers, MouseButton, MouseScrollDelta};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

fn logical_window_size(size: PhysicalSize<u32>, scale_factor: f32) -> (u32, u32) {
    if scale_factor <= 0.0 {
        return (size.width, size.height);
    }
    (
        (size.width as f32 / scale_factor).round() as u32,
        (size.height as f32 / scale_factor).round() as u32,
    )
}

/// Folds this session's windows into the saved layout. Must merge, not replace:
/// a window never opened this session has no entry here and would otherwise lose
/// the position an earlier session recorded.
fn merge_window_state(
    saved: &mut HashMap<u32, WindowStateEntry>,
    positions: &HashMap<u32, [f32; 2]>,
    open_collapsed: &HashMap<u32, (bool, bool)>,
) {
    for (id, pos) in positions {
        let (open, collapsed) = open_collapsed.get(id).copied().unwrap_or((false, false));
        saved.insert(
            *id,
            WindowStateEntry {
                position: *pos,
                open,
                collapsed,
            },
        );
    }
}

impl App {
    pub(crate) fn capture_window_state(&mut self) {
        let positions = self.ui_state_cache.extract_window_positions();
        let open_collapsed = self
            .game
            .extract_window_state(&self.windows, &self.ui_state_cache);
        merge_window_state(&mut self.config.window_state, &positions, &open_collapsed);
        self.config.hotkey_visible_rows = self.game.character.hotkeys.visible_rows();
        self.config.battle_mode = self.game.character.hotkeys.battle_mode();
        self.capture_window_size();
    }

    fn capture_window_size(&mut self) {
        if self.config.fullscreen {
            return;
        }
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        // `with_inner_size` takes a LogicalSize, so the OS scale factor is what the
        // saved value has to round-trip through, not `dpi_scale`.
        let (width, height) = logical_window_size(size, window.scale_factor() as f32);
        if width > 0 && height > 0 {
            self.config.screen_width = width;
            self.config.screen_height = height;
        }
    }

    pub(crate) fn handle_close_requested(&mut self, event_loop: &ActiveEventLoop) {
        self.capture_window_state();
        self.config.save("config.json");
        event_loop.exit();
    }

    pub(crate) fn handle_focus_changed(&mut self, focused: bool) {
        self.window_focused = focused;
        self.apply_sound_pause();
    }

    pub(crate) fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(size.width, size.height);
        }
    }

    /// Consumes the pending right-press so a third click can't count as another
    /// double.
    fn take_right_double_click(&mut self) -> bool {
        let now = std::time::Instant::now();
        let pos = self.input.mouse_position;
        let is_double = self.input.last_right_press.is_some_and(|(at, (px, py))| {
            let dx = (pos.0 - px) as f32;
            let dy = (pos.1 - py) as f32;
            now.duration_since(at).as_millis() < DOUBLE_CLICK_THRESHOLD_MS
                && (dx * dx + dy * dy).sqrt() < DOUBLE_CLICK_DISTANCE
        });
        self.input.last_right_press = if is_double { None } else { Some((now, pos)) };
        is_double
    }

    pub(crate) fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if self.game.session.app_state == AppState::InGame {
            match button {
                MouseButton::Right => {
                    self.input.right_mouse_down = state == ElementState::Pressed;
                    if self.input.right_mouse_down {
                        if self.take_right_double_click() && !self.input.ui_hovered {
                            let control = self.camera_control();
                            if let Some(renderer) = &mut self.renderer {
                                renderer.camera.apply_reset_gesture(control);
                            }
                        }
                        self.input.right_dragged = false;
                        // Picking clears the hovered ids while the button is held (rotate cursor),
                        // so capture the target now, before it's lost.
                        self.input.right_press_entity = self.game.hover.hovered_player_id;
                        self.input.right_press_target = self.game.hover.target_id();
                        self.game.pending_casts.pending_skill_target = None;
                        self.game.pending_casts.pending_skill = None;
                        self.game.pending_casts.pending_skill_level = None;
                        self.game.pending_casts.pending_companion_patrol = None;
                        self.game.companions.capture_targeting = false;
                        self.game.companions.pet_roulette = None;
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
                            } else if !self
                                .open_companion_context_menu(self.input.right_press_target)
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
                                self.game.hover.target_id(),
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

    fn patrol_menu_label(&self, is_mercenary: bool) -> String {
        if self.companion_is_patrolling(is_mercenary) {
            "Stop Patrol".to_string()
        } else {
            "Patrol".to_string()
        }
    }

    fn open_companion_context_menu(&mut self, entity_id: Option<u32>) -> bool {
        let Some(entity_id) = entity_id else {
            return false;
        };
        let is_homun = self
            .game
            .companions
            .homunculus
            .as_ref()
            .is_some_and(|h| !h.vaporized && h.gid == entity_id);
        let is_merc = self
            .game
            .companions
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
                    action: ContextMenuAction::CompanionShowInfo {
                        is_mercenary: false,
                    },
                },
                ContextMenuItem {
                    label: "Feed".to_string(),
                    action: ContextMenuAction::CompanionFeed,
                },
                ContextMenuItem {
                    label: "Standby".to_string(),
                    action: ContextMenuAction::CompanionStandby {
                        is_mercenary: false,
                    },
                },
                ContextMenuItem {
                    label: self.patrol_menu_label(false),
                    action: ContextMenuAction::CompanionPatrol {
                        is_mercenary: false,
                    },
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
                    label: self.patrol_menu_label(true),
                    action: ContextMenuAction::CompanionPatrol { is_mercenary: true },
                },
                ContextMenuItem {
                    label: "AI Settings".to_string(),
                    action: ContextMenuAction::CompanionAiConfig,
                },
            ]
        };
        self.windows
            .context_menu
            .open_at(mx as f32, my as f32, items);
        true
    }

    fn open_pet_context_menu(&mut self, entity_id: Option<u32>) -> bool {
        let Some(entity_id) = entity_id else {
            return false;
        };
        if self.game.companions.pet.gid != Some(entity_id) {
            return false;
        }
        if !self
            .game
            .world
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
        self.windows
            .context_menu
            .open_at(mx as f32, my as f32, items);
        true
    }

    fn open_entity_context_menu(&mut self, entity_id: Option<u32>) {
        let Some(entity_id) = entity_id else {
            return;
        };
        if Some(entity_id) == self.game.world.entities.player_id() {
            return;
        }
        let Some(entity) = self.game.world.entities.get(entity_id) else {
            return;
        };
        if entity.entity_type != EntityType::Player {
            return;
        }
        let (mx, my) = self.input.mouse_position;
        let target_aid = self.game.world.entities.account_id_of(entity_id);
        let mut items = vec![
            ContextMenuItem {
                label: "Deal".to_string(),
                action: ContextMenuAction::RequestTrade { target_aid },
            },
            ContextMenuItem {
                label: "Invite to Party".to_string(),
                action: ContextMenuAction::InviteToParty { target_aid },
            },
        ];
        if matches!(entity.job, 0..=6 | 23) {
            items.push(ContextMenuItem {
                label: "Adopt as Baby".to_string(),
                action: ContextMenuAction::AdoptBaby { target_aid },
            });
        }
        if let Some(g) = &self.game.guild {
            let local_gid = self
                .game
                .session
                .login_session
                .as_ref()
                .map(|s| s.account_id)
                .unwrap_or(0);
            let rights = g.my_rights(local_gid);
            if rights.can_invite {
                items.push(ContextMenuItem {
                    label: "Invite to Guild".to_string(),
                    action: ContextMenuAction::GuildInvite { target_aid },
                });
            }
            if g.is_master(local_gid) {
                items.push(ContextMenuItem {
                    label: "Request Alliance".to_string(),
                    action: ContextMenuAction::GuildAlly { target_aid },
                });
                items.push(ContextMenuItem {
                    label: "Declare Hostility".to_string(),
                    action: ContextMenuAction::GuildHostile { target_aid },
                });
            }
        }
        if self.is_gm_account() {
            items.push(ContextMenuItem {
                label: "Give Manner Point".to_string(),
                action: ContextMenuAction::GiveMannerPoint {
                    target_aid,
                    positive: true,
                },
            });
            items.push(ContextMenuItem {
                label: "Take Manner Point".to_string(),
                action: ContextMenuAction::GiveMannerPoint {
                    target_aid,
                    positive: false,
                },
            });
            items.push(ContextMenuItem {
                label: "Account Name".to_string(),
                action: ContextMenuAction::AccountName { aid: target_aid },
            });
        }
        self.windows
            .context_menu
            .open_at(mx as f32, my as f32, items);
    }

    pub(crate) fn handle_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let dpi = self.renderer.as_ref().map_or(1.0, |r| r.dpi_scale) as f64;
        let logical_pos = (position.x / dpi, position.y / dpi);
        self.input.mouse_position = logical_pos;
        if self.game.session.app_state == AppState::InGame && self.input.right_mouse_down {
            if let Some((lx, ly)) = self.input.last_mouse_pos {
                let dx = (logical_pos.0 - lx) as f32;
                let dy = (logical_pos.1 - ly) as f32;
                if dx.abs() > 1.0 || dy.abs() > 1.0 {
                    self.input.right_dragged = true;
                }
                let control = self.camera_control();
                if let Some(renderer) = &mut self.renderer {
                    renderer.camera.apply_drag(dx, dy, control);
                }
            }
            self.input.last_mouse_pos = Some(logical_pos);
        }
    }

    pub(crate) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if self.game.session.app_state == AppState::InGame && !self.input.ui_hovered {
            let notches = match delta {
                MouseScrollDelta::LineDelta(_, y) => y,
                MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
            };
            if notches == 0.0 {
                return;
            }
            let control = self.camera_control();
            if let Some(renderer) = &mut self.renderer {
                renderer.camera.apply_wheel(notches, control);
            }
        }
    }

    fn camera_control(&self) -> CameraControl {
        let account_id = self
            .game
            .session
            .login_session
            .as_ref()
            .map_or(0, |s| s.account_id);
        CameraControl {
            indoor: self.game.session.camera_locked,
            gm: self.config.is_gm_account(account_id),
            shift: self.input.shift_pressed,
            ctrl: self.input.ctrl_pressed,
            alt: self.input.alt_pressed,
            star_gazing: self.is_star_gazing(),
            unbounded: self.config.free_camera,
        }
    }

    /// A seated Star Gladiator who has not learned Demon of the Sun, Moon and
    /// Stars may pull the camera far back to watch the sky.
    fn is_star_gazing(&self) -> bool {
        let Some(player) = self.game.world.entities.player() else {
            return false;
        };
        player.state == EntityState::Sitting
            && JobName::try_from_value(player.job as usize) == Ok(JobName::StarGladiator)
            && player.health_state & OPT2_BLIND == 0
            && self
                .game
                .character
                .skills
                .get_skill(SkillEnum::SgDevil)
                .is_none_or(|skill| skill.level <= 0)
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

        if pressed && self.windows.hotkey_config_window.is_capturing() {
            if let Some(code) = code {
                self.capture_hotkey(code);
            }
            if let Some(ctx) = &mut self.ui_context {
                ctx.key_escape = false;
                ctx.typed_chars.clear();
            }
            return;
        }

        if !pressed || self.game.session.app_state != AppState::InGame {
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
            self.windows.minimap_window.cycle_visibility();
            return;
        }

        // A focused chat box only swallows unmodified keys; Alt/Ctrl chords stay
        // live while typing.
        let typing = self.windows.chat_window.is_active()
            && !self.input.alt_pressed
            && !self.input.ctrl_pressed;
        if typing || self.windows.system_menu.open {
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
            KeyCode::KeyP if self.input.ctrl_pressed => {
                self.profiler.start();
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
            HotkeyAction::ToggleEquipment => self.windows.equipment_window.toggle(),
            HotkeyAction::ToggleSkillTree => self.game.character.skills.toggle(),
            HotkeyAction::ToggleStatus => self.windows.status_window.toggle(),
            HotkeyAction::ToggleShortcutList => {
                if !self.windows.shortcut_list_window.is_open() {
                    self.windows
                        .shortcut_list_window
                        .set_bindings(&self.config.shortcut_commands);
                }
                self.windows.shortcut_list_window.toggle();
            }
            HotkeyAction::ToggleEmotion => self.windows.emotion_window.toggle(),
            HotkeyAction::ToggleQuest => self.windows.quest_window.toggle(),
            HotkeyAction::ToggleCart => {
                let has_cart = self
                    .game
                    .world
                    .entities
                    .player()
                    .is_some_and(|p| p.cart_type.is_some());
                if has_cart {
                    self.game.character.cart.toggle();
                }
            }
            HotkeyAction::ToggleGuild => {
                if self.game.guild.is_some() {
                    self.windows.guild_window.toggle();
                } else {
                    self.windows
                        .chat_window
                        .add_system("You are not in a guild.".to_string());
                }
            }
            HotkeyAction::ToggleChatRoomCreate => self.windows.chat_room_create_window.toggle(),
            HotkeyAction::ToggleBasicInfo => self.windows.basic_info_window.toggle(),
            HotkeyAction::ToggleParty => self.windows.party_friends_window.open_party_tab(),
            HotkeyAction::ToggleFriends => self.windows.party_friends_window.open_friend_tab(),
            HotkeyAction::TogglePet => {
                if self.game.companions.pet.gid.is_some() {
                    self.windows.pet_window.toggle();
                }
            }
            HotkeyAction::ToggleSoundOptions => self.open_sound_options(),
            HotkeyAction::ToggleGraphicOptions => self.open_graphic_options(),
            HotkeyAction::ToggleHomunculus => {
                if self.game.companions.homunculus.is_some() {
                    self.windows.homunculus_window.toggle();
                }
            }
            HotkeyAction::ToggleMercenary => {
                if self.game.companions.mercenary.is_some() {
                    self.windows.mercenary_window.toggle();
                }
            }
            HotkeyAction::SitStand => {
                if self.player_hidden() {
                    return;
                }
                if let Some(entity) = self.game.world.entities.player() {
                    let action = if entity.state == EntityState::Sitting {
                        3u8
                    } else {
                        2u8
                    };
                    self.channel.send_packet(build_action_request_packet(
                        0,
                        action,
                        self.active_packetver,
                    ));
                }
            }
            HotkeyAction::CycleMinimap => self.windows.minimap_window.cycle_visibility(),
            HotkeyAction::ToggleWorldMap => self.windows.world_map_window.toggle(),
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
            self.windows.hotkey_config_window.cancel_capture();
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
        self.windows.hotkey_config_window.capture_key(chord);
    }

    pub(crate) fn handle_modifiers_changed(&mut self, modifiers: Modifiers) {
        self.input.alt_pressed = modifiers.state().alt_key();
        self.input.shift_pressed = modifiers.state().shift_key();
        self.input.ctrl_pressed = modifiers.state().control_key();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn resized_window_size_survives_a_config_round_trip() {
        let mut config = Config::default();
        let (width, height) = logical_window_size(PhysicalSize::new(2560, 1440), 2.0);
        config.screen_width = width;
        config.screen_height = height;

        let parsed: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!((parsed.screen_width, parsed.screen_height), (1280, 720));
    }

    #[test]
    fn a_window_left_closed_for_a_whole_session_still_reopens_where_it_was() {
        use ragnarok_renderer::font_atlas::FontAtlas;
        use ragnarok_ui::context::UiContext;
        use ragnarok_ui::frame::UiFrame;
        use ragnarok_ui::state::StateCache;
        use ragnarok_ui_component::game::inventory_window::INV_WINDOW_ID;
        use ragnarok_ui_component::game::storage_window::STORAGE_WINDOW_ID;

        let mut config = Config::default();

        let positions = HashMap::from([
            (STORAGE_WINDOW_ID.0, [500.0, 300.0]),
            (INV_WINDOW_ID.0, [10.0, 20.0]),
        ]);
        let open_collapsed = HashMap::from([(INV_WINDOW_ID.0, (true, false))]);
        merge_window_state(&mut config.window_state, &positions, &open_collapsed);

        let positions = HashMap::from([(INV_WINDOW_ID.0, [40.0, 60.0])]);
        merge_window_state(&mut config.window_state, &positions, &open_collapsed);

        let config: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        let saved: HashMap<u32, [f32; 2]> = config
            .window_state
            .iter()
            .map(|(&id, entry)| (id, entry.position))
            .collect();

        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let ctx = UiContext::new(1024.0, 768.0);
        let mut ui = UiFrame::new(&ctx, &atlas, &mut state, 0.0, false, None, &saved);
        let storage = ui.window_at(STORAGE_WINDOW_ID, 280.0, 300.0, 17.0, 320.0, 80.0);
        let inventory = ui.window_at(INV_WINDOW_ID, 280.0, 300.0, 17.0, 0.0, 0.0);
        assert_eq!((storage.x, storage.y), (500.0, 300.0));
        assert_eq!((inventory.x, inventory.y), (40.0, 60.0));
    }

    #[test]
    fn unknown_scale_factor_keeps_the_physical_size() {
        assert_eq!(
            logical_window_size(PhysicalSize::new(1024, 768), 0.0),
            (1024, 768)
        );
    }
}
