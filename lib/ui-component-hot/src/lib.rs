// Force system allocator to match the host binary's allocator.
// Both sides must use the same heap for cross-FFI Vec/String operations.
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

use ragnarok_game::event::{CharacterInfo, ServerInfo};
use ragnarok_game::inventory::InventoryData;
use ragnarok_game::item::Item;
use ragnarok_game::item_resource_table::ItemResourceTable;
use ragnarok_game::npc_shop::{NpcShopMode, ShopBuyItem, ShopSellItem};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui_component::char_select_window::CharSelectWindow;
use ragnarok_ui_component::chat_window::ChatWindow;
use ragnarok_ui_component::confirm_dialog::ConfirmDialog;
use ragnarok_ui_component::equipment_window::EquipmentWindow;
use ragnarok_ui_component::inventory_window::InventoryWindow;
use ragnarok_ui_component::login_window::LoginWindow;
use ragnarok_ui_component::npc_dialog::NpcDialog;
use ragnarok_ui_component::npc_shop::NpcShop;
use ragnarok_ui_component::server_list_window::ServerListWindow;
use ragnarok_ui_component::system_menu::SystemMenu;

const GAME_COMPONENTS: &[&str] = &[
    "inventory",
    "npc_shop",
    "npc_dialog",
    "equipment",
    "system_menu",
    "confirm_dialog",
    "chat",
];
const ACCOUNT_COMPONENTS: &[&str] = &["login", "server_list", "char_select"];

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
        open: bool,
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
    CharSelect {
        win: CharSelectWindow,
    },
    Category {
        components: Vec<State>,
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

fn create_single(name: &str) -> State {
    match name {
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
        "login" => State::Login {
            login: LoginWindow::new(),
        },
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
            npc.dialog.open_text(
                100,
                "Hello adventurer!\nWelcome to Prontera.\nHow can I help you today?",
            );
            npc.dialog.wait_for_next(100);
            State::NpcDialog { npc }
        }
        "confirm_dialog" => State::ConfirmDialog {
            dialog: ConfirmDialog::new("Are you sure you want to quit?"),
            open: false,
        },
        "server_list" => State::ServerList {
            win: ServerListWindow::new(vec![
                ServerInfo {
                    ip: 0x0100007F,
                    port: 6121,
                    name: "Loki".into(),
                    user_count: 342,
                },
                ServerInfo {
                    ip: 0x0100007F,
                    port: 6122,
                    name: "Iris".into(),
                    user_count: 128,
                },
                ServerInfo {
                    ip: 0x0100007F,
                    port: 6123,
                    name: "Fenrir".into(),
                    user_count: 57,
                },
                ServerInfo {
                    ip: 0x0100007F,
                    port: 6124,
                    name: "Chaos".into(),
                    user_count: 891,
                },
            ]),
        },
        "equipment" => {
            let mut equip = EquipmentWindow::new();
            equip.open = true;
            let mut inventory = InventoryData::new();
            let items = vec![
                Item {
                    index: 0,
                    item_id: 1101,
                    item_type: 5,
                    count: 1,
                    is_identified: true,
                    is_damaged: false,
                    refining_level: 0,
                    slot: [0; 4],
                    location: 0,
                    wear_state: 2,
                    name: "Sword".into(),
                    resource_name: None,
                },
                Item {
                    index: 1,
                    item_id: 2101,
                    item_type: 5,
                    count: 1,
                    is_identified: true,
                    is_damaged: false,
                    refining_level: 0,
                    slot: [0; 4],
                    location: 0,
                    wear_state: 32,
                    name: "Guard".into(),
                    resource_name: None,
                },
                Item {
                    index: 2,
                    item_id: 2301,
                    item_type: 5,
                    count: 1,
                    is_identified: true,
                    is_damaged: false,
                    refining_level: 0,
                    slot: [0; 4],
                    location: 0,
                    wear_state: 16,
                    name: "Chain Mail".into(),
                    resource_name: None,
                },
                Item {
                    index: 3,
                    item_id: 2401,
                    item_type: 5,
                    count: 1,
                    is_identified: true,
                    is_damaged: false,
                    refining_level: 0,
                    slot: [0; 4],
                    location: 0,
                    wear_state: 64,
                    name: "Sandals".into(),
                    resource_name: None,
                },
                Item {
                    index: 4,
                    item_id: 2501,
                    item_type: 5,
                    count: 1,
                    is_identified: true,
                    is_damaged: false,
                    refining_level: 0,
                    slot: [0; 4],
                    location: 0,
                    wear_state: 4,
                    name: "Hood".into(),
                    resource_name: None,
                },
            ];
            for item in items {
                inventory.add_item(item);
            }
            State::Equipment { equip, inventory }
        }
        "system_menu" => {
            let mut menu = SystemMenu::new();
            menu.open = false;
            State::SystemMenu { menu }
        }
        "char_select" => {
            let characters = vec![
                CharacterInfo {
                    gid: 1,
                    name: "Knight".into(),
                    class: 7,
                    base_level: 50,
                    job_level: 42,
                    map: "prontera".into(),
                    slot: 0,
                    head: 1,
                    hair_color: 0,
                    weapon: 2,
                    head_top: 0,
                    head_mid: 0,
                    head_bottom: 0,
                    shield: 0,
                    sex: 1,
                    hp: 3000,
                    max_hp: 3500,
                    sp: 100,
                    max_sp: 150,
                    str: 50,
                    agi: 30,
                    vit: 40,
                    int: 10,
                    dex: 20,
                    luk: 10,
                },
                CharacterInfo {
                    gid: 2,
                    name: "Wizard".into(),
                    class: 9,
                    base_level: 45,
                    job_level: 38,
                    map: "geffen".into(),
                    slot: 1,
                    head: 2,
                    hair_color: 1,
                    weapon: 10,
                    head_top: 0,
                    head_mid: 0,
                    head_bottom: 0,
                    shield: 0,
                    sex: 0,
                    hp: 1500,
                    max_hp: 1800,
                    sp: 400,
                    max_sp: 500,
                    str: 10,
                    agi: 15,
                    vit: 15,
                    int: 60,
                    dex: 40,
                    luk: 5,
                },
                CharacterInfo {
                    gid: 3,
                    name: "Hunter".into(),
                    class: 11,
                    base_level: 60,
                    job_level: 50,
                    map: "payon".into(),
                    slot: 2,
                    head: 3,
                    hair_color: 2,
                    weapon: 11,
                    head_top: 0,
                    head_mid: 0,
                    head_bottom: 0,
                    shield: 0,
                    sex: 1,
                    hp: 2200,
                    max_hp: 2500,
                    sp: 150,
                    max_sp: 200,
                    str: 20,
                    agi: 60,
                    vit: 20,
                    int: 15,
                    dex: 55,
                    luk: 30,
                },
            ];
            State::CharSelect {
                win: CharSelectWindow::new(characters),
            }
        }
        _ => panic!("Unknown example: {name}"),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_create(name_ptr: *const u8, name_len: usize) -> *mut () {
    let name =
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len)) };
    let state = match name {
        "game" => State::Category {
            components: GAME_COMPONENTS.iter().map(|n| create_single(n)).collect(),
        },
        "account" => State::Category {
            components: ACCOUNT_COMPONENTS
                .iter()
                .map(|n| create_single(n))
                .collect(),
        },
        _ => create_single(name),
    };
    Box::into_raw(Box::new(state)) as *mut ()
}

fn grf_init_single(
    state: &mut State,
    size_fn: &impl Fn(&str) -> Option<(u32, u32)>,
    table: Option<&ItemResourceTable>,
) {
    match state {
        State::Inventory { inv } => {
            if let Some(table) = table {
                inv.inventory.resolve_resource_names(table);
            }
            inv.has_grf_textures = true;
            inv.set_texture_sizes(size_fn);
        }
        State::NpcShop { shop, buy_items, sell_items, .. } => {
            if let Some(table) = table {
                shop.shop.resolve_resource_names(table);
                for item in buy_items.iter_mut() {
                    item.item.resolve_resource_name(table);
                }
                for item in sell_items.iter_mut() {
                    item.item.resolve_resource_name(table);
                }
            }
            shop.has_grf_textures = true;
            shop.set_texture_sizes(size_fn);
        }
        State::Login { login } => {
            login.has_grf_textures = true;
            login.set_texture_sizes(size_fn);
        }
        State::Chat { chat } => {
            chat.has_grf_textures = true;
        }
        State::NpcDialog { npc } => {
            npc.has_grf_textures = true;
            npc.set_texture_sizes(size_fn);
        }
        State::ConfirmDialog { dialog, .. } => {
            dialog.has_grf_textures = true;
            dialog.set_texture_sizes(size_fn);
        }
        State::ServerList { win } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::Equipment { equip, inventory } => {
            if let Some(table) = table {
                inventory.resolve_resource_names(table);
            }
            equip.has_grf_textures = true;
            equip.set_texture_sizes(size_fn);
        }
        State::SystemMenu { menu } => {
            menu.has_grf_textures = true;
            menu.set_texture_sizes(size_fn);
        }
        State::CharSelect { win } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::Category { components } => {
            for component in components.iter_mut() {
                grf_init_single(component, size_fn, table);
            }
        }
    }
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
    grf_init_single(state, &size_fn, table);
}

fn z_order_id(state: &State) -> Option<WidgetId> {
    match state {
        State::Chat { .. } => Some(WidgetId(300)),
        State::Inventory { .. } => Some(WidgetId(800)),
        State::Equipment { .. } => Some(WidgetId(900)),
        _ => None,
    }
}

fn build_single(state: &mut State, ui: &mut UiFrame) {
    match state {
        State::Inventory { inv } => {
            inv.build(ui);
        }
        State::NpcShop {
            shop,
            buy_items,
            sell_items,
            is_sell,
        } => {
            add_button(ui, "Toggle shop", WidgetId(799), 200.0, 10.0, |_ui| {
                *is_sell = !*is_sell;
            });
            let desired = if *is_sell { NpcShopMode::Sell } else { NpcShopMode::Buy };
            if shop.shop.mode != Some(desired) {
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
        State::ConfirmDialog { dialog, open } => {
            if *open {
                let result = dialog.build(ui);
                if result != ragnarok_ui_component::confirm_dialog::ConfirmResult::None {
                    *open = false;
                }
            } else {
            }
        }
        State::ServerList { win } => {
            win.build(ui);
        }
        State::Equipment { equip, inventory } => {
            equip.build(ui, inventory, None, None);
        }
        State::SystemMenu { menu } => {
            add_button(ui, "Open system Dialog", WidgetId(599), 10.0, 10.0, |_ui| {
                menu.open = true;
            });
            menu.build(ui, true);
        }
        State::CharSelect { win } => {
            win.build(ui);
        }
        State::Category { components } => {

            // Build z-orderable windows in persisted order (back-to-front)
            let z_order = ui.get_z_order();
            ui.compute_hovered_window(&z_order);
            for &win_id in &z_order {
                if let Some(comp) = components
                    .iter_mut()
                    .find(|c| z_order_id(c) == Some(win_id))
                {
                    build_single(comp, ui);
                }
            }
            // Build z-orderable windows not yet in z-order
            for comp in components.iter_mut() {
                if let Some(id) = z_order_id(comp) {
                    if !z_order.contains(&id) {
                        build_single(comp, ui);
                    }
                }
            }
            // Build non-z-orderable windows (always on top)
            for comp in components.iter_mut() {
                if z_order_id(comp).is_none() {
                    build_single(comp, ui);
                }
            }
        }
    }
}

fn add_button<F>(
    ui: &mut UiFrame,
    label: &str,
    widget_id: WidgetId,
    x: f32,
    y: f32,
    mut on_click: F,
) where
    F: FnMut(&mut UiFrame),
{
    let btn_rect = Rect::new(x, y, 160.0, 28.0);
    let resp = ui.interact(widget_id, btn_rect);
    let color = if resp.hovered() {
        [0.3, 0.3, 0.5, 1.0]
    } else {
        [0.2, 0.2, 0.35, 1.0]
    };
    let (v, i) =
        ragnarok_ui::draw::quad_vertices(btn_rect.x, btn_rect.y, btn_rect.w, btn_rect.h, color);
    ui.draw_calls.push(ragnarok_ui::draw::DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: ragnarok_ui::draw::TextureRef::White,
    });
    ui.text(
        btn_rect.x + 8.0,
        btn_rect.y + 20.0,
        label,
        [1.0, 1.0, 1.0, 1.0],
    );
    if resp.clicked() {
        on_click(ui);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_build(state_ptr: *mut (), ui_ptr: *mut UiFrame) {
    let state = unsafe { &mut *(state_ptr as *mut State) };
    let ui = unsafe { &mut *ui_ptr };
    build_single(state, ui);
    let _ = ui.draw_drag_icon();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_destroy(state_ptr: *mut ()) {
    if !state_ptr.is_null() {
        drop(unsafe { Box::from_raw(state_ptr as *mut State) });
    }
}

fn make_test_item(index: u16, item_id: u16, item_type: u8, count: i16, name: &str) -> Item {
    Item {
        index,
        item_id,
        item_type,
        count,
        is_identified: true,
        is_damaged: false,
        refining_level: 0,
        slot: [0; 4],
        location: 0,
        wear_state: 0,
        name: name.into(),
        resource_name: None,
    }
}

fn inventory_test_items() -> Vec<Item> {
    let mut items = vec![
        make_test_item(0, 501, 0, 25, "Red Potion"),
        make_test_item(1, 502, 0, 110, "Orange Potion"),
        make_test_item(2, 503, 0, 110, "White Potion"),
        make_test_item(3, 504, 0, 110, "Blue Potion"),
        make_test_item(4, 606, 0, 110, "Aloevera"),
        make_test_item(5, 605, 0, 110, "Anodyne"),
        make_test_item(6, 696, 0, 110, "Level 1 Fire Ball"),
        make_test_item(7, 698, 0, 110, "Level 1 Fire Wall"),
        make_test_item(8, 700, 0, 110, "Level 1 Frost driver"),
        make_test_item(9, 686, 0, 110, "Level 3 Frost driver"),
        make_test_item(10, 601, 0, 5, "Fly Wing"),
        make_test_item(11, 1101, 1, 1, "Sword"),
        make_test_item(12, 2101, 4, 1, "Guard"),
        make_test_item(13, 910, 3, 50, "Jellopy"),
        make_test_item(14, 911, 3, 30, "Shell"),
        make_test_item(15, 610, 0, 30, "Yggdrasil Leaf"),
        make_test_item(16, 611, 0, 30, "Magnifier"),
        make_test_item(17, 531, 0, 30, "Apple juice"),
        make_test_item(18, 513, 0, 30, "Banana"),
        make_test_item(19, 510, 0, 30, "Blue Herb"),
        make_test_item(20, 1201, 1, 1, "Stiletto"),
        make_test_item(21, 2301, 4, 1, "Chain Mail"),
        make_test_item(22, 2402, 4, 1, "Shoes"),
    ];
    items[11].refining_level = 7;
    items[12].refining_level = 5;
    // Unidentified equipment
    items[20].is_identified = false;
    items[20].name = "Unknown Weapon".into();
    items[21].is_identified = false;
    items[21].name = "Unknown Armor".into();
    items[22].is_identified = false;
    items[22].name = "Unknown Shoes".into();
    items
}

fn shop_buy_test_items() -> Vec<ShopBuyItem> {
    vec![
        ShopBuyItem {
            item: make_test_item(0, 501, 0, 1, "Red Potion"),
            price: 50,
            discount_price: 50,
        },
        ShopBuyItem {
            item: make_test_item(0, 502, 0, 1, "Orange Potion"),
            price: 200,
            discount_price: 200,
        },
        ShopBuyItem {
            item: make_test_item(0, 503, 0, 1, "Yellow Potion"),
            price: 550,
            discount_price: 550,
        },
        ShopBuyItem {
            item: make_test_item(0, 504, 0, 1, "White Potion"),
            price: 1200,
            discount_price: 1200,
        },
        ShopBuyItem {
            item: make_test_item(0, 505, 0, 1, "Blue Potion"),
            price: 5000,
            discount_price: 5000,
        },
        ShopBuyItem {
            item: make_test_item(0, 506, 0, 1, "Green Potion"),
            price: 10,
            discount_price: 10,
        },
        ShopBuyItem {
            item: make_test_item(0, 601, 0, 1, "Fly Wing"),
            price: 40,
            discount_price: 40,
        },
        ShopBuyItem {
            item: make_test_item(0, 602, 0, 1, "Butterfly Wing"),
            price: 175,
            discount_price: 175,
        },
    ]
}

fn shop_sell_test_items() -> Vec<ShopSellItem> {
    vec![
        ShopSellItem {
            item: make_test_item(0, 501, 0, 30, "Red Potion"),
            price: 25,
            overcharge_price: 25,
        },
        ShopSellItem {
            item: make_test_item(1, 502, 0, 12, "Orange Potion"),
            price: 100,
            overcharge_price: 110,
        },
        ShopSellItem {
            item: make_test_item(2, 503, 0, 5, "Yellow Potion"),
            price: 275,
            overcharge_price: 300,
        },
        ShopSellItem {
            item: make_test_item(3, 1201, 1, 1, "Stiletto"),
            price: 5000,
            overcharge_price: 5500,
        },
        ShopSellItem {
            item: make_test_item(4, 601, 0, 47, "Fly Wing"),
            price: 20,
            overcharge_price: 22,
        },
        ShopSellItem {
            item: make_test_item(5, 602, 0, 8, "Butterfly Wing"),
            price: 87,
            overcharge_price: 95,
        },
        ShopSellItem {
            item: {
                let mut item = make_test_item(6, 2402, 4, 1, "Unknown Shoes");
                item.is_identified = false;
                item
            },
            price: 2500,
            overcharge_price: 2750,
        },
    ]
}
