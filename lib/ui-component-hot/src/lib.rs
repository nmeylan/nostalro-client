// Force system allocator to match the host binary's allocator.
// Both sides must use the same heap for cross-FFI Vec/String operations.
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

use ragnarok_game::event::ServerInfo;
use ragnarok_game::inventory::{InventoryData, InventoryItem};
use ragnarok_game::item_resource_table::ItemResourceTable;
use ragnarok_game::npc_shop::{ShopBuyItem, ShopSellItem};
use ragnarok_ui::frame::UiFrame;
use ragnarok_ui_component::chat_window::ChatWindow;
use ragnarok_ui_component::confirm_dialog::ConfirmDialog;
use ragnarok_ui_component::equipment_window::EquipmentWindow;
use ragnarok_ui_component::inventory_window::InventoryWindow;
use ragnarok_ui_component::login_window::LoginWindow;
use ragnarok_ui_component::npc_dialog::NpcDialog;
use ragnarok_ui_component::npc_shop::NpcShop;
use ragnarok_ui_component::server_list_window::ServerListWindow;
use ragnarok_ui_component::system_menu::SystemMenu;

enum State {
    Inventory {
        inv: InventoryWindow,
    },
    NpcShop {
        shop: NpcShop,
        buy_items: Vec<ShopBuyItem>,
        sell_items: Vec<ShopSellItem>,
        is_sell: bool,
    },
    Login {
        login: LoginWindow,
    },
    Chat {
        chat: ChatWindow,
    },
    NpcDialog {
        npc: NpcDialog,
    },
    ConfirmDialog {
        dialog: ConfirmDialog,
    },
    ServerList {
        win: ServerListWindow,
    },
    Equipment {
        equip: EquipmentWindow,
        inventory: InventoryData,
    },
    SystemMenu {
        menu: SystemMenu,
    },
}

type TextureSizeFn = unsafe extern "C" fn(*const u8, usize, *mut u32, *mut u32) -> bool;

fn wrap_texture_size_fn(f: TextureSizeFn) -> impl Fn(&str) -> Option<(u32, u32)> {
    move |name: &str| {
        let mut w = 0u32;
        let mut h = 0u32;
        if unsafe { f(name.as_ptr(), name.len(), &mut w, &mut h) } {
            Some((w, h))
        } else {
            None
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_create(name_ptr: *const u8, name_len: usize) -> *mut () {
    let name = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len)) };
    let state = match name {
        "inventory" => {
            let mut inv = InventoryWindow::new();
            inv.inventory.toggle();
            for item in inventory_test_items() {
                inv.inventory.add_item(item);
            }
            State::Inventory { inv }
        }
        "npc_shop" => {
            let buy_items = shop_buy_test_items();
            let mut shop = NpcShop::new();
            shop.shop.open_buy(100, buy_items.clone());
            State::NpcShop {
                shop,
                buy_items,
                sell_items: shop_sell_test_items(),
                is_sell: false,
            }
        }
        "login" => State::Login { login: LoginWindow::new() },
        "chat" => {
            let mut chat = ChatWindow::new();
            chat.active = true;
            chat.add_chat("Welcome to Ragnarok Online!".into());
            chat.add_chat("Type /help for a list of commands.".into());
            chat.add_chat("[Swordsman]: Anyone want to party for Payon Dungeon?".into());
            chat.add_chat("[Merchant]: Selling Red Potions 50z each!".into());
            chat.add_chat("[Acolyte]: LFG Byalan Island".into());
            chat.add_chat("[Archer]: WTB Composite Bow +5".into());
            chat.add_chat("^FF0000[System]: Server maintenance in 30 minutes.".into());
            chat.add_chat("[Mage]: Trading Fire Bolt 10 for Cold Bolt 10".into());
            State::Chat { chat }
        }
        "npc_dialog" => {
            let mut npc = NpcDialog::new();
            npc.dialog.open_text(100, "Hello adventurer!\nWelcome to Prontera.\nHow can I help you today?");
            npc.dialog.wait_for_next(100);
            State::NpcDialog { npc }
        }
        "confirm_dialog" => State::ConfirmDialog {
            dialog: ConfirmDialog::new("Are you sure you want to quit?"),
        },
        "server_list" => State::ServerList {
            win: ServerListWindow::new(vec![
                ServerInfo { ip: 0x0100007F, port: 6121, name: "Loki".into(), user_count: 342 },
                ServerInfo { ip: 0x0100007F, port: 6122, name: "Iris".into(), user_count: 128 },
                ServerInfo { ip: 0x0100007F, port: 6123, name: "Fenrir".into(), user_count: 57 },
                ServerInfo { ip: 0x0100007F, port: 6124, name: "Chaos".into(), user_count: 891 },
            ]),
        },
        "equipment" => {
            let mut equip = EquipmentWindow::new();
            equip.open = true;
            let mut inventory = InventoryData::new();
            let items = vec![
                InventoryItem { index: 0, item_id: 1101, item_type: 5, count: 1, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 2, name: "Sword".into(), resource_name: None },
                InventoryItem { index: 1, item_id: 2101, item_type: 5, count: 1, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 32, name: "Guard".into(), resource_name: None },
                InventoryItem { index: 2, item_id: 2301, item_type: 5, count: 1, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 16, name: "Chain Mail".into(), resource_name: None },
                InventoryItem { index: 3, item_id: 2401, item_type: 5, count: 1, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 64, name: "Sandals".into(), resource_name: None },
                InventoryItem { index: 4, item_id: 2501, item_type: 5, count: 1, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 4, name: "Hood".into(), resource_name: None },
            ];
            for item in items {
                inventory.add_item(item);
            }
            State::Equipment { equip, inventory }
        }
        "system_menu" => {
            let mut menu = SystemMenu::new();
            menu.open = true;
            State::SystemMenu { menu }
        }
        _ => panic!("Unknown example: {name}"),
    };
    Box::into_raw(Box::new(state)) as *mut ()
}

/// Called once after GRF textures are loaded, and again after each reload.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_grf_init(
    state_ptr: *mut (),
    texture_size_fn: TextureSizeFn,
    item_resource_table: *const ItemResourceTable,
) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    let size_fn = wrap_texture_size_fn(texture_size_fn);
    let table = if item_resource_table.is_null() {
        None
    } else {
        Some(unsafe { &*item_resource_table })
    };

    match state {
        State::Inventory { inv } => {
            if let Some(table) = table {
                inv.inventory.resolve_resource_names(table);
            }
            inv.has_grf_textures = true;
            inv.set_texture_sizes(&size_fn);
        }
        State::NpcShop { shop, .. } => {
            if let Some(table) = table {
                shop.shop.resolve_resource_names(table);
            }
            shop.has_grf_textures = true;
            shop.set_texture_sizes(&size_fn);
        }
        State::Login { login } => {
            login.has_grf_textures = true;
            login.set_texture_sizes(&size_fn);
        }
        State::Chat { chat } => {
            chat.has_grf_textures = true;
        }
        State::NpcDialog { npc } => {
            npc.has_grf_textures = true;
            npc.set_texture_sizes(&size_fn);
        }
        State::ConfirmDialog { dialog } => {
            dialog.has_grf_textures = true;
            dialog.set_texture_sizes(&size_fn);
        }
        State::ServerList { win } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(&size_fn);
        }
        State::Equipment { equip, inventory } => {
            if let Some(table) = table {
                inventory.resolve_resource_names(table);
            }
            equip.has_grf_textures = true;
            equip.set_texture_sizes(&size_fn);
        }
        State::SystemMenu { menu } => {
            menu.has_grf_textures = true;
            menu.set_texture_sizes(&size_fn);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_build(state_ptr: *mut (), ui_ptr: *mut UiFrame) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    let ui = unsafe { &mut *ui_ptr };

    match state {
        State::Inventory { inv } => {
            inv.build(ui);
        }
        State::NpcShop { shop, buy_items, sell_items, is_sell } => {
            if ui.ctx.key_tab {
                *is_sell = !*is_sell;
                if *is_sell {
                    shop.shop.open_sell(100, sell_items.clone());
                } else {
                    shop.shop.open_buy(100, buy_items.clone());
                }
            }
            shop.build(ui);
        }
        State::Login { login } => {
            login.build(ui);
        }
        State::Chat { chat } => {
            chat.build(ui);
        }
        State::NpcDialog { npc } => {
            npc.build(ui);
        }
        State::ConfirmDialog { dialog } => {
            dialog.build(ui);
        }
        State::ServerList { win } => {
            win.build(ui);
        }
        State::Equipment { equip, inventory } => {
            equip.build(ui, inventory, None, None);
        }
        State::SystemMenu { menu } => {
            menu.build(ui, true);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_destroy(state_ptr: *mut ()) {
    if !state_ptr.is_null() {
        drop(unsafe { Box::from_raw(state_ptr as *mut State) });
    }
}

fn inventory_test_items() -> Vec<InventoryItem> {
    vec![
        InventoryItem { index: 0, item_id: 501, item_type: 0, count: 25, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Red Potion".into(), resource_name: None },
        InventoryItem { index: 1, item_id: 502, item_type: 0, count: 10, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Orange Potion".into(), resource_name: None },
        InventoryItem { index: 2, item_id: 601, item_type: 0, count: 5, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Fly Wing".into(), resource_name: None },
        InventoryItem { index: 3, item_id: 1101, item_type: 1, count: 1, is_identified: true, is_damaged: false, refining_level: 7, slot: [0; 4], location: 0, wear_state: 0, name: "Sword".into(), resource_name: None },
        InventoryItem { index: 4, item_id: 2101, item_type: 4, count: 1, is_identified: true, is_damaged: false, refining_level: 5, slot: [0; 4], location: 0, wear_state: 0, name: "Guard".into(), resource_name: None },
        InventoryItem { index: 5, item_id: 910, item_type: 3, count: 50, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Jellopy".into(), resource_name: None },
        InventoryItem { index: 6, item_id: 911, item_type: 3, count: 30, is_identified: true, is_damaged: false, refining_level: 0, slot: [0; 4], location: 0, wear_state: 0, name: "Shell".into(), resource_name: None },
    ]
}

fn shop_buy_test_items() -> Vec<ShopBuyItem> {
    vec![
        ShopBuyItem { item_id: 501, price: 50, discount_price: 50, item_type: 0, name: "Red Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 502, price: 200, discount_price: 200, item_type: 0, name: "Orange Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 503, price: 550, discount_price: 550, item_type: 0, name: "Yellow Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 504, price: 1200, discount_price: 1200, item_type: 0, name: "White Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 505, price: 5000, discount_price: 5000, item_type: 0, name: "Blue Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 506, price: 10, discount_price: 10, item_type: 0, name: "Green Potion".into(), resource_name: None },
        ShopBuyItem { item_id: 601, price: 40, discount_price: 40, item_type: 0, name: "Fly Wing".into(), resource_name: None },
        ShopBuyItem { item_id: 602, price: 175, discount_price: 175, item_type: 0, name: "Butterfly Wing".into(), resource_name: None },
    ]
}

fn shop_sell_test_items() -> Vec<ShopSellItem> {
    vec![
        ShopSellItem { index: 0, price: 25, overcharge_price: 25, name: "Red Potion".into(), resource_name: Some("빨간포션".into()), count: 30 },
        ShopSellItem { index: 1, price: 100, overcharge_price: 110, name: "Orange Potion".into(), resource_name: Some("주황포션".into()), count: 12 },
        ShopSellItem { index: 2, price: 275, overcharge_price: 300, name: "Yellow Potion".into(), resource_name: Some("노란포션".into()), count: 5 },
        ShopSellItem { index: 3, price: 5000, overcharge_price: 5500, name: "Stiletto".into(), resource_name: Some("스틸레토".into()), count: 1 },
        ShopSellItem { index: 4, price: 20, overcharge_price: 22, name: "Fly Wing".into(), resource_name: Some("파리의날개".into()), count: 47 },
        ShopSellItem { index: 5, price: 87, overcharge_price: 95, name: "Butterfly Wing".into(), resource_name: Some("나비의날개".into()), count: 8 },
    ]
}
