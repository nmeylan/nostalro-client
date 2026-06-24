use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::config::WindowStateEntry;
use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::ailment::AilmentOverlay;
use ragnarok_game::app_state::AppState;
use ragnarok_game::character::Character;
use ragnarok_game::chat_room::ChatRoomRegistry;
use ragnarok_game::cursor::{CursorAnimationState, PendingSkillTarget, RenderEntry};
use ragnarok_game::damage_number::DamageNumberManager;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::effects::AmbientEffectScheduler;
use ragnarok_game::arrow::ArrowProjectile;
use ragnarok_game::entity_collection::EntityCollection;
use ragnarok_game::event::{CharacterInfo, GameEvent};
use ragnarok_game::floor_item::FloorItem;
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_game::server_time::ServerTimeClock;
use ragnarok_game::targeting::MapProperties;
use ragnarok_network::session::Session;
use ragnarok_renderer::{EntitySprite, SpriteTextures};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::state::StateCache;
use ragnarok_game::entity::EntityType;
use ragnarok_game::sprite_path::JT_WARPNPC;
use ragnarok_ui_component::game::basic_info_window::{BASIC_INFO_WINDOW_ID, BasicInfoWindow};
use ragnarok_ui_component::game::card_insert_dialog::CardInsertDialog;
use ragnarok_ui_component::game::chat_room_window::{ChatRoomPlacement, ChatRoomWindow};
use ragnarok_ui_component::game::chat_window::{self, ChatWindow};
use ragnarok_ui_component::game::drop_quantity_dialog::DropQuantityDialog;
use ragnarok_ui_component::game::equipment_window::{EQ_WINDOW_ID, EquipmentWindow};
use ragnarok_ui_component::game::hotkey_bar::{HOTKEY_BAR_WINDOW_ID, HotkeyBarWindow};
use ragnarok_ui_component::game::inventory_window::{INV_WINDOW_ID, InventoryWindow};
use ragnarok_ui_component::game::item_info_window::ItemInfoWindow;
use ragnarok_ui_component::game::item_pickup_notification::ItemPickupNotification;
use ragnarok_ui_component::game::minimap_window::{MarkerType, MinimapMarker, MinimapWindow};
use ragnarok_ui_component::game::npc_dialog::NpcDialog;
use ragnarok_ui_component::game::warp_list_window::WarpListWindow;
use ragnarok_ui_component::game::npc_shop::NpcShop;
use ragnarok_ui_component::game::skill_tree_window::{SKILL_WINDOW_ID, SkillTreeWindow};
use ragnarok_ui_component::game::status_window::{StatusWindow, STATUS_WINDOW_ID};
use ragnarok_ui_component::game::system_menu::SystemMenu;
use ragnarok_ui_component::{InGameWindow, Window};
use ragnarok_ui_component::game::confirm_dialog::ConfirmDialog;

pub struct GameState {
    pub app_state: AppState,
    pub login_session: Option<Session>,
    pub selected_character: Option<CharacterInfo>,
    pub current_map: Option<String>,
    pub map_properties: MapProperties,
    pub map_coords: Option<MapCoordinates>,
    pub gat: Option<GatFile>,
    pub entities: EntityCollection,
    pub sprites: HashMap<u32, Rc<EntitySprite>>,
    pub sprite_cache: HashMap<String, Rc<EntitySprite>>,
    pub character: Character,
    pub data_table: DataTable,
    pub cursor_textures: Option<SpriteTextures>,
    pub cursor_act: Option<ActFile>,
    pub cursor_animation: CursorAnimationState,
    pub lock_cursor_animation: CursorAnimationState,
    pub emotion_textures: Option<SpriteTextures>,
    pub emotion_act: Option<ActFile>,
    /// Persistent status-ailment overlay sprites (stun/sleep/curse/angelus),
    /// billboarded above an afflicted actor's head; loaded once at startup.
    pub status_overlay_sprites: HashMap<AilmentOverlay, (SpriteTextures, ActFile)>,
    pub chat_window: ChatWindow,
    pub equipment_window: EquipmentWindow,
    pub inventory_window: InventoryWindow,
    pub npc_dialog: NpcDialog,
    pub warp_list_window: WarpListWindow,
    pub confirm_dialog: ConfirmDialog,
    pub npc_shop: NpcShop,
    pub chat_rooms: ChatRoomRegistry,
    pub chat_room_window: ChatRoomWindow,
    pub system_menu: SystemMenu,
    pub hovered_entity_id: Option<u32>,
    pub hovered_floor_item_id: Option<u32>,
    pub failed_sprite_loads: HashSet<u32>,
    pub server_time: ServerTimeClock,
    pub attack_range: i16,
    pub floor_items: HashMap<u32, FloorItem>,
    pub floor_item_sprites: HashMap<u32, (Rc<SpriteTextures>, ActFile)>,
    pub arrows: Vec<ArrowProjectile>,
    pub waiting_item_throw_ack: bool,
    pub drop_dialog_has_grf_textures: bool,
    pub drop_quantity_dialog: Option<DropQuantityDialog>,
    pub card_insert_dialog: Option<CardInsertDialog>,
    pub card_insert_dialog_has_grf_textures: bool,
    pub pending_skill_target: Option<PendingSkillTarget>,
    pub pending_skill_id: Option<u16>,
    pub pending_skill_level: Option<i16>,
    pub pending_card_composition_index: Option<u16>,
    pub pending_pickup_item_id: Option<u32>,
    pub attack_target_id: Option<u32>,
    pub attack_request_cooldown: f32,
    pub noshift_mode: bool,
    pub noctrl_mode: bool,
    pub attack_is_locked: bool,
    pub item_info_window: ItemInfoWindow,
    pub item_pickup_notification: ItemPickupNotification,
    pub skill_tree_window: SkillTreeWindow,
    pub basic_info_window: BasicInfoWindow,
    pub status_window: StatusWindow,
    pub hotkey_bar: HotkeyBarWindow,
    pub minimap_window: MinimapWindow,
    pub damage_numbers: DamageNumberManager,
    pub damage_number_textures: Option<SpriteTextures>,
    pub damage_number_act: Option<ragnarok_formats::act::ActFile>,
    pub damage_msg_textures: Option<SpriteTextures>,
    pub damage_msg_act: Option<ragnarok_formats::act::ActFile>,
    pub debug_show_pick_bounds: bool,
    /// Toggles the network sync overlay (RTT/offset). Seeded from config, toggled with F10.
    pub debug_overlay: bool,
    pub ambient_effects: AmbientEffectScheduler,
    /// Live EFST status-buff effects, keyed by `(gid, efst)` → the synthetic
    /// owner key its effect was spawned with, so a status-off packet despawns
    /// exactly that buff without touching the entity's other buffs.
    pub status_buff_keys: HashMap<(u32, i16), u32>,
    /// Monotonic source for buff owner keys. The high bit is set so these never
    /// collide with the real `gid`/`aid` values other keyed effects use (Blind,
    /// ground units).
    pub next_status_buff_key: u32,
    /// Live level-99 auras, keyed by the holder gid → the owner key its ring was
    /// spawned with, so it can be despawned when the actor hides or leaves view.
    pub level_aura_keys: HashMap<u32, u32>,
    /// Live boss auras (green level-99 reskin on MVP/boss monsters), keyed by the
    /// holder gid → the owner key, so they despawn when the monster leaves view
    /// or dies.
    pub boss_aura_keys: HashMap<u32, u32>,
    /// Live warp-portal effects, keyed by the warp NPC's gid → the owner key, so
    /// the portal despawns when the NPC leaves view.
    pub warp_portal_keys: HashMap<u32, u32>,
    /// Set to true when disconnect dialog is shown.
    pub disconnect_dialog_shown: bool,
    /// Set to true after disconnect dialog is confirmed/cancelled.
    pub pending_disconnect_exit: bool,
}

const Z_ORDERABLE_WINDOWS: &[WidgetId] = &[
    BASIC_INFO_WINDOW_ID,
    chat_window::CHAT_WINDOW_ID,
    INV_WINDOW_ID,
    EQ_WINDOW_ID,
    SKILL_WINDOW_ID,
    STATUS_WINDOW_ID,
];

impl GameState {
    pub fn build_in_game_ui(
        &mut self,
        ui: &mut UiFrame,
        texture_size_fn: &dyn Fn(&str) -> Option<(u32, u32)>,
        render_list: &[RenderEntry],
    ) -> Vec<GameEvent> {
        let chat_was_active = self.chat_window.is_active();
        let mut events = Vec::new();

        // Modal windows block interaction with z-ordered windows behind them
        self.npc_shop.setup_modal(ui);

        // Build z-orderable windows in persisted order (back-to-front).
        let z_order = ui.get_z_order();
        ui.compute_hovered_window(&z_order);
        for &win_id in &z_order {
            self.build_window(win_id, ui, &mut events);
        }
        // Build windows not yet in z-order (first appearance)
        for &win_id in Z_ORDERABLE_WINDOWS {
            if !z_order.contains(&win_id) {
                self.build_window(win_id, ui, &mut events);
            }
        }

        // Hotkey bar (always visible, not z-orderable)
        self.hotkey_bar.chat_is_active = self.chat_window.is_active();
        events.extend(
            self.hotkey_bar
                .build(ui, &mut self.character, &self.data_table),
        );

        // Minimap (always visible, not z-orderable)
        if let Some(player) = self.entities.player() {
            self.minimap_window.player_position = Some(player.movement.position());
            self.minimap_window.player_direction = player.direction;
        }
        if let Some(coords) = &self.map_coords {
            self.minimap_window.map_width = coords.gat_width();
            self.minimap_window.map_height = coords.gat_height();
        }
        self.minimap_window.map_name = self.current_map.clone();
        self.minimap_window.entity_markers.clear();
        for entity in self.entities.iter() {
            if Some(entity.id) == self.entities.player_id() {
                continue;
            }
            if entity.entity_type == EntityType::Npc {
                let (ex, ey) = entity.movement.position();
                let marker_type = if entity.job == JT_WARPNPC {
                    MarkerType::WarpPortal
                } else {
                    MarkerType::Npc
                };
                self.minimap_window
                    .entity_markers
                    .push(MinimapMarker { x: ex, y: ey, marker_type });
            }
        }
        events.extend(
            self.minimap_window
                .build(ui, &mut self.character, &self.data_table),
        );

        // Chat-room boxes (world-anchored, non-draggable, one per active room).
        // Position follows the owner entity, so placements are rebuilt each frame
        // from the registry joined with the world render list.
        self.chat_room_window.placements = self
            .chat_rooms
            .iter()
            .filter_map(|room| {
                let entry = render_list.iter().find(|e| e.id == room.owner_aid)?;
                Some(ChatRoomPlacement {
                    room_id: room.room_id,
                    atype: room.atype,
                    title: room.title.clone(),
                    cur_count: room.cur_count,
                    max_count: room.max_count,
                    anchor_x: entry.screen_anchor[0],
                    anchor_y: entry.screen_anchor[1],
                    head_offset: entry.head_offset,
                })
            })
            .collect();
        events.extend(
            self.chat_room_window
                .build(ui, &mut self.character, &self.data_table),
        );

        let npc_dialog_open = self.npc_dialog.dialog.is_open();
        events.extend(
            self.npc_dialog
                .build(ui, &mut self.character, &self.data_table),
        );
        let shop_open = self.npc_shop.shop.is_open();
        events.extend(
            self.npc_shop
                .build(ui, &mut self.character, &self.data_table),
        );
        let warp_list_open = self.warp_list_window.is_open();
        events.extend(self.warp_list_window.build(ui));
        let mut allow_escape =
            !chat_was_active && !npc_dialog_open && !shop_open && !warp_list_open;
        if allow_escape && ui.ctx.key_escape && self.pending_skill_target.is_some() {
            self.pending_skill_target = None;
            allow_escape = false;
        }
        self.system_menu.allow_escape_toggle = allow_escape;
        events.extend(
            self.system_menu
                .build(ui, &mut self.character, &self.data_table),
        );
        events.extend(
            self.item_info_window
                .build(ui, &mut self.character, &self.data_table),
        );
        events.extend(InGameWindow::build(
            &mut self.item_pickup_notification,
            ui,
            &mut self.character,
            &self.data_table,
        ));

        // Always-on-top: confirm dialog (for disconnect notification)
        let had_disconnect_dialog = self.disconnect_dialog_shown
            && self.confirm_dialog.state.is_some();
        self.confirm_dialog.build(ui);
        if had_disconnect_dialog && self.confirm_dialog.state.is_none() {
            // Disconnect dialog was just closed
            self.pending_disconnect_exit = true;
            self.disconnect_dialog_shown = false;
        }

        ui.flush_tooltips();

        // Drag-cancel handling
        if let Some(cancelled) = ui.draw_drag_icon() {
            if cancelled.source_id == HOTKEY_BAR_WINDOW_ID {
                if self.character.hotkeys.get_slot(cancelled.item_index)
                    != ragnarok_game::hotkey::HotkeySlotContent::Empty
                {
                    self.character.hotkeys.clear_slot(cancelled.item_index);
                    events.push(GameEvent::RequestHotkeyChange {
                        index: cancelled.item_index as u16,
                        is_skill: false,
                        id: 0,
                        count: 0,
                    });
                }
            } else if cancelled.source_id == INV_WINDOW_ID {
                if self.waiting_item_throw_ack {
                    // Already waiting for server ack, ignore
                } else if self.equipment_window.is_visible() {
                    self.chat_window
                        .add_system("Please close the Equipment window.".to_string());
                } else if let Some(item) = self
                    .character
                    .inventory
                    .get_item(cancelled.item_index as u16)
                {
                    if item.count > 1 {
                        let mut dialog = DropQuantityDialog::new(item.index, item.count);
                        dialog.has_grf_textures = self.drop_dialog_has_grf_textures;
                        if dialog.has_grf_textures {
                            dialog.set_texture_sizes(texture_size_fn);
                        }
                        self.drop_quantity_dialog = Some(dialog);
                    } else {
                        events.push(GameEvent::RequestDropItem {
                            index: item.index,
                            count: 1,
                        });
                        self.waiting_item_throw_ack = true;
                    }
                }
            }
        }

        if let Some(dialog) = &mut self.drop_quantity_dialog {
            let dialog_events =
                InGameWindow::build(dialog, ui, &mut self.character, &self.data_table);
            let closed = dialog_events.iter().any(|e| {
                matches!(
                    e,
                    GameEvent::DialogClosed | GameEvent::RequestDropItem { .. }
                )
            });
            if closed {
                if dialog_events
                    .iter()
                    .any(|e| matches!(e, GameEvent::RequestDropItem { .. }))
                {
                    self.waiting_item_throw_ack = true;
                }
                self.drop_quantity_dialog = None;
            }
            events.extend(
                dialog_events
                    .into_iter()
                    .filter(|e| !matches!(e, GameEvent::DialogClosed)),
            );
        }

        if let Some(dialog) = &mut self.card_insert_dialog {
            let dialog_events =
                InGameWindow::build(dialog, ui, &mut self.character, &self.data_table);
            let closed = dialog_events.iter().any(|e| {
                matches!(
                    e,
                    GameEvent::DialogClosed | GameEvent::RequestCardInsert { .. }
                )
            });
            if closed {
                self.card_insert_dialog = None;
            }
            events.extend(
                dialog_events
                    .into_iter()
                    .filter(|e| !matches!(e, GameEvent::DialogClosed)),
            );
        }

        events
    }

    fn build_window(&mut self, win_id: WidgetId, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        match win_id {
            BASIC_INFO_WINDOW_ID => events.extend(self.basic_info_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            chat_window::CHAT_WINDOW_ID => events.extend(self.chat_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            INV_WINDOW_ID => events.extend(self.inventory_window.build(
                ui,
                &mut self.character,
                &self.data_table,
            )),
            EQ_WINDOW_ID => {
                events.extend(self.equipment_window.build(
                    ui,
                    &mut self.character,
                    &self.data_table,
                ));
            }
            SKILL_WINDOW_ID => {
                events.extend(self.skill_tree_window.build(
                    ui,
                    &mut self.character,
                    &self.data_table,
                ));
            }
            STATUS_WINDOW_ID => {
                events.extend(self.status_window.build(
                    ui,
                    &mut self.character,
                    &self.data_table,
                ));
            }
            _ => {}
        }
    }

    pub fn new() -> Self {
        Self {
            app_state: AppState::Login,
            login_session: None,
            selected_character: None,
            current_map: None,
            map_properties: MapProperties::default(),
            map_coords: None,
            gat: None,
            entities: EntityCollection::new(),
            sprites: HashMap::new(),
            sprite_cache: HashMap::new(),
            character: Character::new(),
            data_table: DataTable::new(),
            cursor_textures: None,
            cursor_act: None,
            cursor_animation: CursorAnimationState::new(),
            lock_cursor_animation: CursorAnimationState::new(),
            emotion_textures: None,
            emotion_act: None,
            status_overlay_sprites: HashMap::new(),
            chat_window: ChatWindow::new(),
            equipment_window: EquipmentWindow::new(),
            inventory_window: InventoryWindow::new(),
            npc_dialog: NpcDialog::new(),
            warp_list_window: WarpListWindow::new(),
            confirm_dialog: ConfirmDialog::new(),
            npc_shop: NpcShop::new(),
            chat_rooms: ChatRoomRegistry::new(),
            chat_room_window: ChatRoomWindow::new(),
            system_menu: SystemMenu::new(),
            hovered_entity_id: None,
            hovered_floor_item_id: None,
            failed_sprite_loads: HashSet::new(),
            server_time: ServerTimeClock::new(),
            attack_range: 1,
            floor_items: HashMap::new(),
            floor_item_sprites: HashMap::new(),
            arrows: Vec::new(),
            waiting_item_throw_ack: false,
            drop_dialog_has_grf_textures: false,
            drop_quantity_dialog: None,
            card_insert_dialog: None,
            card_insert_dialog_has_grf_textures: false,
            pending_skill_target: None,
            pending_skill_id: None,
            pending_skill_level: None,
            pending_card_composition_index: None,
            pending_pickup_item_id: None,
            attack_target_id: None,
            attack_request_cooldown: 0.0,
            noshift_mode: false,
            noctrl_mode: true,
            attack_is_locked: false,
            basic_info_window: BasicInfoWindow::new(),
            status_window: StatusWindow::new(),
            item_info_window: ItemInfoWindow::new(),
            item_pickup_notification: ItemPickupNotification::new(),
            skill_tree_window: SkillTreeWindow::new(),
            hotkey_bar: HotkeyBarWindow::new(),
            minimap_window: MinimapWindow::new(),
            disconnect_dialog_shown: false,
            pending_disconnect_exit: false,
            damage_numbers: DamageNumberManager::new(),
            damage_number_textures: None,
            damage_number_act: None,
            damage_msg_textures: None,
            damage_msg_act: None,
            debug_show_pick_bounds: false,
            debug_overlay: false,
            ambient_effects: AmbientEffectScheduler::empty(),
            status_buff_keys: HashMap::new(),
            next_status_buff_key: 0,
            level_aura_keys: HashMap::new(),
            boss_aura_keys: HashMap::new(),
            warp_portal_keys: HashMap::new(),
        }
    }

    pub fn apply_window_state(&mut self, window_state: &HashMap<u32, WindowStateEntry>) {
        if let Some(entry) = window_state.get(&INV_WINDOW_ID.0) {
            if entry.open {
                self.character.inventory.open();
            }
            self.inventory_window.set_minimized(entry.collapsed);
        }
        if let Some(entry) = window_state.get(&EQ_WINDOW_ID.0) {
            self.equipment_window.open = entry.open;
            self.equipment_window.set_minimized(entry.collapsed);
        }
        if let Some(entry) = window_state.get(&SKILL_WINDOW_ID.0)
            && entry.open
        {
            self.character.skills.open();
        }
        if let Some(entry) = window_state.get(&chat_window::CHAT_WINDOW_ID.0) {
            let size_index = if !entry.open {
                0
            } else if entry.collapsed {
                1
            } else {
                5
            };
            self.chat_window.set_initial_size_index(size_index);
        }
    }

    pub fn extract_window_state(&self, state_cache: &StateCache) -> HashMap<u32, (bool, bool)> {
        let mut result = HashMap::new();
        result.insert(
            INV_WINDOW_ID.0,
            (
                self.character.inventory.is_open(),
                self.inventory_window.is_minimized(),
            ),
        );
        result.insert(
            EQ_WINDOW_ID.0,
            (
                self.equipment_window.is_open(),
                self.equipment_window.is_minimized(),
            ),
        );
        result.insert(SKILL_WINDOW_ID.0, (self.character.skills.is_open(), false));
        let size_index = self.chat_window.get_size_index(state_cache);
        result.insert(
            chat_window::CHAT_WINDOW_ID.0,
            (size_index > 0, size_index == 1),
        );
        result
    }
}
