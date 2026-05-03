use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::config::WindowStateEntry;
use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::app_state::AppState;
use ragnarok_game::character::Character;
use ragnarok_game::cursor::{CursorAnimationState, PendingSkillTarget};
use ragnarok_game::damage_number::DamageNumberManager;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::effects::EffectManager;
use ragnarok_game::entity_collection::EntityCollection;
use ragnarok_game::event::{CharacterInfo, GameEvent};
use ragnarok_game::floor_item::FloorItem;
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_game::server_time::ServerTimeClock;
use ragnarok_network::session::Session;
use ragnarok_renderer::{EntitySprite, SpriteTextures};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::state::StateCache;
use ragnarok_ui_component::game::card_insert_dialog::CardInsertDialog;
use ragnarok_ui_component::game::chat_window::{self, ChatWindow};
use ragnarok_ui_component::game::drop_quantity_dialog::DropQuantityDialog;
use ragnarok_ui_component::game::equipment_window::{EquipmentWindow, EQ_WINDOW_ID};
use ragnarok_ui_component::game::hotkey_bar::{HotkeyBarWindow, HOTKEY_BAR_WINDOW_ID};
use ragnarok_ui_component::game::inventory_window::{InventoryWindow, INV_WINDOW_ID};
use ragnarok_ui_component::game::item_info_window::ItemInfoWindow;
use ragnarok_ui_component::game::item_pickup_notification::ItemPickupNotification;
use ragnarok_ui_component::game::npc_dialog::NpcDialog;
use ragnarok_ui_component::game::npc_shop::NpcShop;
use ragnarok_ui_component::game::skill_tree_window::{SkillTreeWindow, SKILL_WINDOW_ID};
use ragnarok_ui_component::game::system_menu::SystemMenu;
use ragnarok_ui_component::{InGameWindow, Window};

pub struct GameState {
    pub app_state: AppState,
    pub login_session: Option<Session>,
    pub selected_character: Option<CharacterInfo>,
    pub current_map: Option<String>,
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
    pub chat_window: ChatWindow,
    pub equipment_window: EquipmentWindow,
    pub inventory_window: InventoryWindow,
    pub npc_dialog: NpcDialog,
    pub npc_shop: NpcShop,
    pub system_menu: SystemMenu,
    pub hovered_entity_id: Option<u32>,
    pub hovered_floor_item_id: Option<u32>,
    pub failed_sprite_loads: HashSet<u32>,
    pub server_time: ServerTimeClock,
    pub attack_range: i16,
    pub floor_items: HashMap<u32, FloorItem>,
    pub floor_item_sprites: HashMap<u32, (Rc<SpriteTextures>, ActFile)>,
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
    pub hotkey_bar: HotkeyBarWindow,
    pub damage_numbers: DamageNumberManager,
    pub damage_number_textures: Option<SpriteTextures>,
    pub damage_number_act: Option<ragnarok_formats::act::ActFile>,
    pub damage_msg_textures: Option<SpriteTextures>,
    pub damage_msg_act: Option<ragnarok_formats::act::ActFile>,
    pub debug_show_pick_bounds: bool,
    pub effects: EffectManager,
}

const Z_ORDERABLE_WINDOWS: &[WidgetId] = &[
    chat_window::CHAT_WINDOW_ID,
    INV_WINDOW_ID,
    EQ_WINDOW_ID,
    SKILL_WINDOW_ID,
];

impl GameState {
    pub fn build_in_game_ui(
        &mut self,
        ui: &mut UiFrame,
        texture_size_fn: &dyn Fn(&str) -> Option<(u32, u32)>,
    ) -> Vec<GameEvent> {
        let chat_was_active = self.chat_window.is_active();
        let mut events = Vec::new();

        // Modal windows block interaction with z-ordered windows behind them
        self.npc_shop.setup_modal(ui);

        // Build z-orderable windows in persisted order (back-to-front)
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

        // Always-on-top windows (not z-orderable)
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
        let mut allow_escape = !chat_was_active && !npc_dialog_open && !shop_open;
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
            _ => {}
        }
    }

    pub fn new() -> Self {
        Self {
            app_state: AppState::Login,
            login_session: None,
            selected_character: None,
            current_map: None,
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
            chat_window: ChatWindow::new(),
            equipment_window: EquipmentWindow::new(),
            inventory_window: InventoryWindow::new(),
            npc_dialog: NpcDialog::new(),
            npc_shop: NpcShop::new(),
            system_menu: SystemMenu::new(),
            hovered_entity_id: None,
            hovered_floor_item_id: None,
            failed_sprite_loads: HashSet::new(),
            server_time: ServerTimeClock::new(),
            attack_range: 1,
            floor_items: HashMap::new(),
            floor_item_sprites: HashMap::new(),
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
            item_info_window: ItemInfoWindow::new(),
            item_pickup_notification: ItemPickupNotification::new(),
            skill_tree_window: SkillTreeWindow::new(),
            hotkey_bar: HotkeyBarWindow::new(),
            damage_numbers: DamageNumberManager::new(),
            damage_number_textures: None,
            damage_number_act: None,
            damage_msg_textures: None,
            damage_msg_act: None,
            debug_show_pick_bounds: false,
            effects: EffectManager::empty(),
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
            && entry.open {
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
