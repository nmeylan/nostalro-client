use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use ragnarok_formats::act::ActFile;
use ragnarok_formats::gat::GatFile;
use ragnarok_game::accessory_table::AccessoryTable;
use ragnarok_game::app_state::AppState;
use ragnarok_game::cursor::CursorAnimationState;
use ragnarok_game::entity_collection::EntityCollection;
use ragnarok_game::floor_item::FloorItem;
use ragnarok_game::map_coordinates::MapCoordinates;
use ragnarok_game::card_name_table::CardNameTable;
use ragnarok_game::item_name_table::ItemNameTable;
use ragnarok_game::item_resource_table::ItemResourceTable;
use ragnarok_game::item_slot_count_table::ItemSlotCountTable;
use ragnarok_game::name_table::NameTable;
use ragnarok_game::event::{CharacterInfo, GameEvent};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_game::server_time::ServerTimeClock;
use ragnarok_network::session::Session;
use ragnarok_renderer::{EntitySprite, SpriteTextures};
use ragnarok_ui_component::chat_window::{self, ChatWindow};
use ragnarok_ui_component::drop_quantity_dialog::{DropQuantityDialog, DropQuantityResult};
use ragnarok_ui_component::equipment_window::{EquipmentWindow, EQ_WINDOW_ID};
use ragnarok_ui_component::inventory_window::{InventoryWindow, INV_WINDOW_ID};
use ragnarok_ui_component::item_pickup_notification::ItemPickupNotification;
use ragnarok_ui_component::npc_dialog::NpcDialog;
use ragnarok_ui_component::npc_shop::NpcShop;
use ragnarok_ui_component::system_menu::SystemMenu;

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
    pub name_table: Option<NameTable>,
    pub accessory_table: Option<AccessoryTable>,
    pub cursor_textures: Option<SpriteTextures>,
    pub cursor_act: Option<ActFile>,
    pub cursor_animation: CursorAnimationState,
    pub emotion_textures: Option<SpriteTextures>,
    pub emotion_act: Option<ActFile>,
    pub item_name_table: Option<ItemNameTable>,
    pub item_resource_table: Option<ItemResourceTable>,
    pub item_slot_count_table: Option<ItemSlotCountTable>,
    pub card_name_table: Option<CardNameTable>,
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
    pub drop_quantity_dialog: Option<DropQuantityDialog>,
    pub pending_pickup_item_id: Option<u32>,
    pub item_pickup_notification: ItemPickupNotification,
    pub debug_show_pick_bounds: bool,
}

const Z_ORDERABLE_WINDOWS: &[WidgetId] = &[
    chat_window::CHAT_WINDOW_ID,
    INV_WINDOW_ID,
    EQ_WINDOW_ID,
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

        // Always-on-top windows (not z-orderable)
        let npc_dialog_open = self.npc_dialog.dialog.is_open();
        events.extend(self.npc_dialog.build(ui));
        let shop_open = self.npc_shop.shop.is_open();
        events.extend(self.npc_shop.build(ui));
        let inv_open = self.inventory_window.inventory.is_open();
        let allow_escape = !chat_was_active && !npc_dialog_open && !shop_open && !inv_open;
        events.extend(self.system_menu.build(ui, allow_escape));
        self.item_pickup_notification.build(ui);

        // Drag-cancel handling
        if let Some(cancelled) = ui.draw_drag_icon() {
            if cancelled.source_id == INV_WINDOW_ID {
                if self.waiting_item_throw_ack {
                    // Already waiting for server ack, ignore
                } else if self.equipment_window.is_visible() {
                    self.chat_window
                        .add_system("Please close the Equipment window.".to_string());
                } else if let Some(item) = self
                    .inventory_window
                    .inventory
                    .get_item(cancelled.item_index as u16)
                {
                    if item.count > 1 {
                        let mut dialog = DropQuantityDialog::new(item.index, item.count);
                        dialog.has_grf_textures = self.inventory_window.has_grf_textures;
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
            match dialog.build(ui) {
                DropQuantityResult::Ok(qty) => {
                    let index = dialog.item_index;
                    events.push(GameEvent::RequestDropItem { index, count: qty });
                    self.waiting_item_throw_ack = true;
                    self.drop_quantity_dialog = None;
                }
                DropQuantityResult::Cancel => {
                    self.drop_quantity_dialog = None;
                }
                DropQuantityResult::None => {}
            }
        }

        events
    }

    fn build_window(&mut self, win_id: WidgetId, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        match win_id {
            chat_window::CHAT_WINDOW_ID => events.extend(self.chat_window.build(ui)),
            INV_WINDOW_ID => events.extend(self.inventory_window.build(ui)),
            EQ_WINDOW_ID => {
                let Self {
                    equipment_window,
                    inventory_window,
                    item_slot_count_table,
                    card_name_table,
                    ..
                } = self;
                events.extend(equipment_window.build(
                    ui,
                    &inventory_window.inventory,
                    item_slot_count_table.as_ref(),
                    card_name_table.as_ref(),
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
            name_table: None,
            accessory_table: None,
            cursor_textures: None,
            cursor_act: None,
            cursor_animation: CursorAnimationState::new(),
            emotion_textures: None,
            emotion_act: None,
            item_name_table: None,
            item_resource_table: None,
            item_slot_count_table: None,
            card_name_table: None,
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
            drop_quantity_dialog: None,
            pending_pickup_item_id: None,
            item_pickup_notification: ItemPickupNotification::new(),
            debug_show_pick_bounds: false,
        }
    }
}
