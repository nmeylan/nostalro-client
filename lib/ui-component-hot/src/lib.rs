// Force system allocator to match the host binary's allocator.
// Both sides must use the same heap for cross-FFI Vec/String operations.
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

use models::enums::EnumWithNumberValue;
use models::enums::item::ItemType;
use ragnarok_game::card_illustration_table::CardIllustrationTable;
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::{CharacterInfo, GameEvent, ServerInfo};
use ragnarok_game::item::Item;
use ragnarok_game::item_description_table::ItemDescriptionTable;
use ragnarok_game::item_name_table::ItemNameTable;
use ragnarok_game::item_resource_table::ItemResourceTable;
use ragnarok_game::item_slot_count_table::ItemSlotCountTable;
use ragnarok_game::npc_shop::{NpcShopMode, ShopBuyItem, ShopSellItem};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui_component::account::char_select_window::CharSelectWindow;
use ragnarok_ui_component::account::login_window::LoginWindow;
use ragnarok_ui_component::account::server_list_window::ServerListWindow;
use ragnarok_ui_component::game::basic_info_window::BasicInfoWindow;
use ragnarok_ui_component::game::card_insert_dialog::{CardInsertDialog, EligibleItem};
use ragnarok_ui_component::game::chat_window::ChatWindow;
use ragnarok_ui_component::game::confirm_dialog::ConfirmDialog;
use ragnarok_ui_component::game::equipment_window::EquipmentWindow;
use ragnarok_ui_component::game::hotkey_bar::HotkeyBarWindow;
use ragnarok_ui_component::game::inventory_window::InventoryWindow;
use ragnarok_ui_component::game::item_info_window::ItemInfoWindow;
use ragnarok_ui_component::game::item_pickup_notification::ItemPickupNotification;
use ragnarok_ui_component::game::npc_dialog::NpcDialog;
use ragnarok_ui_component::game::npc_shop::NpcShop;
use ragnarok_ui_component::game::number_input::{NumberInputConfig, NumberInputDialog};
use ragnarok_ui_component::game::skill_tree_window::SkillTreeWindow;
use ragnarok_ui_component::game::basic_info_window::BasicInfoWindow;
use ragnarok_ui_component::game::status_window::{StatusWindow, STATUS_WINDOW_ID};
use ragnarok_ui_component::game::system_menu::SystemMenu;
use ragnarok_ui_component::{InGameWindow, Window};
use std::collections::HashMap;

const GAME_COMPONENTS: &[&str] = &[
    "inventory",
    "npc_shop",
    "npc_dialog",
    "equipment",
    "system_menu",
    "confirm_dialog",
    "number_input",
    "chat",
    "dialog_container",
    "item_info",
    "skill_tree",
    "card_insert",
    "hotkey_bar",
    "basic_info",
    "status",
];
const ACCOUNT_COMPONENTS: &[&str] = &["login", "server_list", "char_select"];

enum State {
    Inventory {
        inv: InventoryWindow,
        character: Character,
        data: DataTable,
    },
    NpcShop {
        shop: NpcShop,
        buy_items: Vec<ShopBuyItem>,
        sell_items: Vec<ShopSellItem>,
        is_sell: bool,
        character: Character,
        data: DataTable,
    },
    Login {
        login: LoginWindow,
    },
    Chat {
        chat: ChatWindow,
        character: Character,
        data: DataTable,
    },
    NpcDialog {
        npc: NpcDialog,
        character: Character,
        data: DataTable,
    },
    ConfirmDialog {
        dialog: ConfirmDialog,
        open: bool,
    },
    NumberInput {
        dialog: NumberInputDialog,
    },
    ServerList {
        win: ServerListWindow,
    },
    Equipment {
        equip: EquipmentWindow,
        character: Character,
        data: DataTable,
    },
    SystemMenu {
        menu: SystemMenu,
        character: Character,
        data: DataTable,
    },
    CharSelect {
        win: CharSelectWindow,
    },
    ItemInfo {
        win: ItemInfoWindow,
        character: Character,
        data: DataTable,
        item: Item,
    },
    SkillTree {
        win: SkillTreeWindow,
        character: Character,
        data: DataTable,
    },
    CardInsert {
        dialog: CardInsertDialog,
        character: Character,
        data: DataTable,
    },
    DialogContainerDemo {
        notification: ItemPickupNotification,
    },
    HotkeyBarDemo {
        hotkey_win: HotkeyBarWindow,
        character: Character,
        data: DataTable,
    },
    BasicInfoDemo {
        win: BasicInfoWindow,
        character: Character,
        data: DataTable,
    },
    StatusDemo {
        win: StatusWindow,
        character: Character,
        data: DataTable,
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
            let inv = InventoryWindow::new();
            let mut character = Character::new();
            character.inventory.toggle();
            for item in inventory_test_items() {
                character.inventory.add_item(item);
            }
            State::Inventory {
                inv,
                character,
                data: DataTable::new(),
            }
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
                character: Character::new(),
                data: DataTable::new(),
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
            State::Chat {
                chat,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "npc_dialog" => {
            let mut npc = NpcDialog::new();
            npc.dialog.open_text(
                100,
                "Hello adventurer!\nWelcome to Prontera.\nHow can I help you today?",
            );
            npc.dialog.wait_for_next(100);
            State::NpcDialog {
                npc,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "confirm_dialog" => State::ConfirmDialog {
            dialog: ConfirmDialog::new("Are you sure you want to quit?"),
            open: false,
        },
        "number_input" => State::NumberInput {
            dialog: NumberInputDialog::new(
                NumberInputConfig {
                    label: Some("How many (max 99)?".to_string()),
                    show_cancel: false,
                    escape_cancels: true,
                    default_value: "99".to_string(),
                    max_len: 6,
                },
                WidgetId(950),
            ),
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
            let mut character = Character::new();
            let items = vec![
                Item {
                    index: 0,
                    item_id: 1101,
                    item_type: ItemType::Weapon,
                    count: 1,
                    is_identified: true,
                    is_damaged: false,
                    refining_level: 0,
                    slot: [0; 4],
                    location: 0,
                    wear_state: 2,
                    name: "Quadruple liberation two-handed Sword".into(),
                    resource_name: None,
                },
                Item {
                    index: 1,
                    item_id: 2101,
                    item_type: ItemType::Weapon,
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
                    item_type: ItemType::Armor,
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
                    item_type: ItemType::Armor,
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
                    item_type: ItemType::Armor,
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
                character.inventory.add_item(item);
            }
            State::Equipment {
                equip,
                character,
                data: DataTable::new(),
            }
        }
        "system_menu" => {
            let mut menu = SystemMenu::new();
            menu.open = false;
            State::SystemMenu {
                menu,
                character: Character::new(),
                data: DataTable::new(),
            }
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
        "item_info" => {
            let mut slot_entries = HashMap::new();
            slot_entries.insert(1701u16, 3u8);
            let mut desc_entries = HashMap::new();
            desc_entries.insert(
                1701u16,
                vec![
                    "A bow with 3 card slots.".to_string(),
                    "Class:^0000FF Weapon^000000".to_string(),
                    "Attack:^777777 15^000000".to_string(),
                    "Weight:^777777 50^000000".to_string(),
                    "Weapon Level:^777777 1^000000".to_string(),
                    "Required Level:^777777 4^000000".to_string(),
                    "Applicable Job:^777777 Archer class^000000".to_string(),
                ],
            );
            desc_entries.insert(
                4025u16,
                vec![
                    "A card with a picture".to_string(),
                    "of a Goblin on it.".to_string(),
                    "ATK+10, CRIT+5".to_string(),
                    "Class:^0000FF Card^000000".to_string(),
                    "Compound on:^777777 Weapon^000000".to_string(),
                    "Weight:^777777 1^000000".to_string(),
                ],
            );
            let mut name_identified = HashMap::new();
            name_identified.insert(1701u16, "Bow".to_string());
            name_identified.insert(4025u16, "Goblin Card".to_string());
            let mut illust_entries = HashMap::new();
            illust_entries.insert(4025u16, "고블린카드".to_string());
            let data = DataTable {
                item_slot_count: Some(ItemSlotCountTable::from_entries(slot_entries)),
                item_description: Some(ItemDescriptionTable::from_entries(
                    desc_entries,
                    HashMap::new(),
                )),
                item_resource: None,
                item_name: Some(ItemNameTable::from_entries(name_identified, HashMap::new())),
                card_illustration: Some(CardIllustrationTable::from_entries(illust_entries)),
                ..DataTable::new()
            };
            // resource_name: None — resolved from GRF's ItemResourceTable in grf_init_single
            let bow = Item {
                index: 0,
                item_id: 1701,
                item_type: ItemType::Armor,
                count: 1,
                is_identified: true,
                is_damaged: false,
                refining_level: 0,
                slot: [4025, 4025, 0xFFFF, 0],
                location: 0,
                wear_state: 0,
                name: "Bow".into(),
                resource_name: None,
            };
            let mut win = ItemInfoWindow::new();
            win.show(&bow, &data);
            State::ItemInfo {
                win,
                character: Character::new(),
                data,
                item: bow,
            }
        }
        "skill_tree" => {
            use ragnarok_game::skill::{SkillData, SkillTargetType};
            use ragnarok_game::skill_name_table::SkillNameTable;
            use ragnarok_game::skill_tree_table::{SkillTreeEntry, SkillTreeTable};

            let mut character = Character::new();
            character.skill_point = 5;
            character.skills.open();
            character.skills.set_skills(vec![
                SkillData {
                    id: 1,
                    name: "SM_SWORD".into(),
                    level: 10,
                    selected_level: 10,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: false,
                    skill_target_type: SkillTargetType::Passive,
                },
                SkillData {
                    id: 2,
                    name: "SM_RECOVERY".into(),
                    level: 5,
                    selected_level: 5,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Passive,
                },
                SkillData {
                    id: 3,
                    name: "SM_BASH".into(),
                    level: 5,
                    selected_level: 5,
                    sp_cost: 8,
                    attack_range: 1,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Target,
                },
                SkillData {
                    id: 4,
                    name: "SM_PROVOKE".into(),
                    level: 3,
                    selected_level: 3,
                    sp_cost: 4,
                    attack_range: 9,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Target,
                },
                SkillData {
                    id: 5,
                    name: "SM_ENDURE".into(),
                    level: 0,
                    selected_level: 0,
                    sp_cost: 10,
                    attack_range: 0,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Ground,
                },
            ]);

            let mut skill_names = HashMap::new();
            skill_names.insert("SM_SWORD".into(), "Sword Mastery".into());
            skill_names.insert("SM_RECOVERY".into(), "HP Recovery".into());
            skill_names.insert("SM_BASH".into(), "Bash".into());
            skill_names.insert("SM_PROVOKE".into(), "Provoke".into());
            skill_names.insert("SM_AUTOBERSERK".into(), "Auto Berserk".into());
            skill_names.insert("SM_MOVINGRECOVERY".into(), "Moving Recovery".into());
            skill_names.insert("SM_TWOHAND".into(), "Two-Hand Mastery".into());
            skill_names.insert("SM_MAGNUM".into(), "Magnum Break".into());
            skill_names.insert("SM_ENDURE".into(), "Endure".into());
            skill_names.insert("SM_FATALBLOW".into(), "Fatal Blow".into());
            skill_names.insert("NV_BASIC".into(), "Basic Skill".into());

            let mut trees = HashMap::new();
            trees.insert(
                0,
                vec![SkillTreeEntry {
                    skill_name: "NV_BASIC".into(),
                    position: 0,
                    max_level: 9,
                    prerequisite_positions: vec![],
                }],
            );
            trees.insert(
                1,
                vec![
                    SkillTreeEntry {
                        skill_name: "SM_SWORD".into(),
                        position: 1,
                        max_level: 10,
                        prerequisite_positions: vec![],
                    },
                    SkillTreeEntry {
                        skill_name: "SM_RECOVERY".into(),
                        position: 2,
                        max_level: 10,
                        prerequisite_positions: vec![],
                    },
                    SkillTreeEntry {
                        skill_name: "SM_BASH".into(),
                        position: 3,
                        max_level: 10,
                        prerequisite_positions: vec![],
                    },
                    SkillTreeEntry {
                        skill_name: "SM_PROVOKE".into(),
                        position: 4,
                        max_level: 10,
                        prerequisite_positions: vec![],
                    },
                    SkillTreeEntry {
                        skill_name: "SM_AUTOBERSERK".into(),
                        position: 5,
                        max_level: 1,
                        prerequisite_positions: vec![],
                    },
                    SkillTreeEntry {
                        skill_name: "SM_MOVINGRECOVERY".into(),
                        position: 6,
                        max_level: 1,
                        prerequisite_positions: vec![],
                    },
                    SkillTreeEntry {
                        skill_name: "SM_TWOHAND".into(),
                        position: 8,
                        max_level: 10,
                        prerequisite_positions: vec![1],
                    },
                    SkillTreeEntry {
                        skill_name: "SM_MAGNUM".into(),
                        position: 10,
                        max_level: 10,
                        prerequisite_positions: vec![3],
                    },
                    SkillTreeEntry {
                        skill_name: "SM_ENDURE".into(),
                        position: 11,
                        max_level: 10,
                        prerequisite_positions: vec![4],
                    },
                    SkillTreeEntry {
                        skill_name: "SM_FATALBLOW".into(),
                        position: 12,
                        max_level: 1,
                        prerequisite_positions: vec![],
                    },
                ],
            );

            let data = DataTable {
                skill_name: Some(SkillNameTable::from_entries(skill_names)),
                skill_tree: Some(SkillTreeTable::from_entries(trees)),
                ..DataTable::new()
            };

            let win = SkillTreeWindow::new();

            State::SkillTree {
                win,
                character,
                data,
            }
        }
        "card_insert" => {
            let eligible = vec![
                EligibleItem {
                    inventory_index: 1,
                    display_name: "+7 Sword [3]".into(),
                    icon_path: None,
                },
                EligibleItem {
                    inventory_index: 2,
                    display_name: "Bow [2]".into(),
                    icon_path: None,
                },
                EligibleItem {
                    inventory_index: 3,
                    display_name: "+5 Guard [1]".into(),
                    icon_path: None,
                },
                EligibleItem {
                    inventory_index: 4,
                    display_name: "Chain Mail [1]".into(),
                    icon_path: None,
                },
                EligibleItem {
                    inventory_index: 5,
                    display_name: "Sandals [1]".into(),
                    icon_path: None,
                },
                EligibleItem {
                    inventory_index: 6,
                    display_name: "Hood [1]".into(),
                    icon_path: None,
                },
                EligibleItem {
                    inventory_index: 7,
                    display_name: "Muffler [1]".into(),
                    icon_path: None,
                },
            ];
            let mut dialog = CardInsertDialog::new();
            dialog.open(10, "Poring Card".into(), eligible);
            State::CardInsert {
                dialog,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "dialog_container" => {
            let mut notification = ItemPickupNotification::new();
            notification.show("Sticky Mucus".to_string(), 1, None);
            State::DialogContainerDemo { notification }
        }
        "hotkey_bar" => {
            use ragnarok_game::hotkey::HotkeySlotContent;
            use ragnarok_game::skill::{SkillData, SkillTargetType};
            use ragnarok_game::skill_name_table::SkillNameTable;
            use ragnarok_game::skill_tree_table::{SkillTreeEntry, SkillTreeTable};

            let mut character = Character::new();

            character.inventory.toggle();
            for item in inventory_test_items() {
                character.inventory.add_item(item);
            }

            character.skill_point = 5;
            character.skills.open();
            character.skills.set_skills(vec![
                SkillData {
                    id: 43,
                    name: "AC_OWL".into(),
                    level: 10,
                    selected_level: 10,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: false,
                    skill_target_type: SkillTargetType::Passive,
                },
                SkillData {
                    id: 44,
                    name: "AC_VULTURE".into(),
                    level: 10,
                    selected_level: 10,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: false,
                    skill_target_type: SkillTargetType::Passive,
                },
                SkillData {
                    id: 45,
                    name: "AC_CONCENTRATION".into(),
                    level: 10,
                    selected_level: 10,
                    sp_cost: 15,
                    attack_range: 0,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Ground,
                },
                SkillData {
                    id: 46,
                    name: "AC_DOUBLE".into(),
                    level: 10,
                    selected_level: 10,
                    sp_cost: 12,
                    attack_range: 9,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Target,
                },
                SkillData {
                    id: 47,
                    name: "AC_SHOWER".into(),
                    level: 10,
                    selected_level: 10,
                    sp_cost: 15,
                    attack_range: 9,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Target,
                },
            ]);

            character.hotkeys.set_slot(
                0,
                HotkeySlotContent::Skill {
                    skill_id: 46,
                    level: 10,
                },
            );
            character.hotkeys.set_slot(
                1,
                HotkeySlotContent::Item {
                    item_id: 501,
                    inventory_index: 0,
                },
            );

            let mut skill_names = HashMap::new();
            skill_names.insert("AC_OWL".into(), "Owl's Eye".into());
            skill_names.insert("AC_VULTURE".into(), "Vulture's Eye".into());
            skill_names.insert("AC_CONCENTRATION".into(), "Improve Concentration".into());
            skill_names.insert("AC_DOUBLE".into(), "Double Strafe".into());
            skill_names.insert("AC_SHOWER".into(), "Arrow Shower".into());
            skill_names.insert("NV_BASIC".into(), "Basic Skill".into());

            let mut trees = HashMap::new();
            trees.insert(
                0,
                vec![SkillTreeEntry {
                    skill_name: "NV_BASIC".into(),
                    position: 0,
                    max_level: 9,
                    prerequisite_positions: vec![],
                }],
            );
            trees.insert(
                3,
                vec![
                    SkillTreeEntry {
                        skill_name: "AC_OWL".into(),
                        position: 1,
                        max_level: 10,
                        prerequisite_positions: vec![],
                    },
                    SkillTreeEntry {
                        skill_name: "AC_VULTURE".into(),
                        position: 2,
                        max_level: 10,
                        prerequisite_positions: vec![],
                    },
                    SkillTreeEntry {
                        skill_name: "AC_CONCENTRATION".into(),
                        position: 3,
                        max_level: 10,
                        prerequisite_positions: vec![],
                    },
                    SkillTreeEntry {
                        skill_name: "AC_DOUBLE".into(),
                        position: 4,
                        max_level: 10,
                        prerequisite_positions: vec![2],
                    },
                    SkillTreeEntry {
                        skill_name: "AC_SHOWER".into(),
                        position: 5,
                        max_level: 10,
                        prerequisite_positions: vec![4],
                    },
                ],
            );

            let data = DataTable {
                skill_name: Some(SkillNameTable::from_entries(skill_names)),
                skill_tree: Some(SkillTreeTable::from_entries(trees)),
                ..DataTable::new()
            };

            State::HotkeyBarDemo {
                hotkey_win: HotkeyBarWindow::new(),
                character,
                data,
            }
        }
        "basic_info" => {
            let mut character = Character::new();
            character.name = "Swordsman".into();
            character.class = 1;
            character.base_level = 42;
            character.job_level = 30;
            character.hp = 2350;
            character.max_hp = 3200;
            character.sp = 85;
            character.max_sp = 120;
            character.base_exp = 185000;
            character.next_base_exp = 300000;
            character.job_exp = 42000;
            character.next_job_exp = 80000;
            character.inventory.weight = 1250;
            character.inventory.max_weight = 3000;
            character.inventory.zeny = 1234567;
            State::BasicInfoDemo {
                win: BasicInfoWindow::new(),
                character,
                data: DataTable::new(),
            }
        }
        "status" => {
            let mut character = Character::new();
            character.name = "Swordsman".into();
            character.class = 1;
            character.base_level = 42;
            character.job_level = 30;
            character.status_point = 12;
            character.skill_point = 3;
            character.str = 35; character.str_bonus = 5; character.str_cost = 6;
            character.agi = 22; character.agi_bonus = 0; character.agi_cost = 4;
            character.vit = 28; character.vit_bonus = 2; character.vit_cost = 5;
            character.int = 10; character.int_bonus = 0; character.int_cost = 3;
            character.dex = 18; character.dex_bonus = -1; character.dex_cost = 4;
            character.luk = 5;  character.luk_bonus = 0; character.luk_cost = 2;
            character.atk1 = 250; character.atk2 = 30;
            character.matk1 = 35; character.matk2 = 40;
            character.def1 = 18; character.def2 = 5;
            character.mdef1 = 4; character.mdef2 = 2;
            character.hit = 122;
            character.flee1 = 95; character.flee2 = 10;
            character.critical = 8;
            character.aspd = 1500;
            let mut win = StatusWindow::new();
            win.toggle();
            State::StatusDemo {
                win,
                character,
                data: DataTable::new(),
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
        State::Inventory { inv, character, .. } => {
            if let Some(table) = table {
                character.inventory.resolve_resource_names(table);
            }
            inv.has_grf_textures = true;
            inv.set_texture_sizes(size_fn);
        }
        State::NpcShop {
            shop,
            buy_items,
            sell_items,
            ..
        } => {
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
        State::Chat { chat, .. } => {
            chat.has_grf_textures = true;
        }
        State::NpcDialog { npc, .. } => {
            npc.has_grf_textures = true;
            npc.set_texture_sizes(size_fn);
        }
        State::ConfirmDialog { dialog, .. } => {
            dialog.has_grf_textures = true;
            dialog.set_texture_sizes(size_fn);
        }
        State::NumberInput { dialog } => {
            dialog.has_grf_textures = true;
            dialog.set_texture_sizes(size_fn);
        }
        State::ServerList { win } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::Equipment {
            equip, character, ..
        } => {
            if let Some(table) = table {
                character.inventory.resolve_resource_names(table);
            }
            equip.has_grf_textures = true;
            equip.set_texture_sizes(size_fn);
        }
        State::SystemMenu { menu, .. } => {
            menu.has_grf_textures = true;
            menu.set_texture_sizes(size_fn);
        }
        State::CharSelect { win } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::ItemInfo {
            win, data, item, ..
        } => {
            if let Some(table) = table {
                item.resolve_resource_name(table);
                // Card icon paths also need the resource table
                if data.item_resource.is_none() {
                    let mut entries = HashMap::new();
                    for &card_id in &item.slot {
                        if card_id != 0
                            && card_id != 0xFFFF
                            && let Some(name) = table.get_resource_name(card_id)
                        {
                            entries.insert(card_id, name.to_string());
                        }
                    }
                    data.item_resource =
                        Some(ItemResourceTable::from_entries(entries, HashMap::new()));
                }
                win.close();
                win.show(item, data);
            }
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::SkillTree { win, .. } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::CardInsert { dialog, .. } => {
            dialog.has_grf_textures = true;
            dialog.set_texture_sizes(size_fn);
        }
        State::DialogContainerDemo { notification } => {
            notification.container.has_grf_textures = true;
            notification.set_texture_sizes(size_fn);
        }
        State::HotkeyBarDemo {
            hotkey_win,
            character,
            data,
        } => {
            if let Some(table) = table {
                character.inventory.resolve_resource_names(table);
                if data.item_resource.is_none() {
                    let mut entries = HashMap::new();
                    for item in character.inventory.all_items() {
                        if let Some(name) = table.get_resource_name(item.item_id) {
                            entries.insert(item.item_id, name.to_string());
                        }
                    }
                    data.item_resource =
                        Some(ItemResourceTable::from_entries(entries, HashMap::new()));
                }
            }
            hotkey_win.has_grf_textures = true;
            hotkey_win.set_texture_sizes(size_fn);
        }
        State::BasicInfoDemo { win, .. } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::StatusDemo { win, .. } => {
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
        State::SkillTree { .. } => Some(WidgetId(1000)),
        State::StatusDemo { .. } => Some(STATUS_WINDOW_ID),
        _ => None,
    }
}

fn build_single(state: &mut State, ui: &mut UiFrame) {
    match state {
        State::Inventory {
            inv,
            character,
            data,
        } => {
            inv.build(ui, character, data);
        }
        State::NpcShop {
            shop,
            buy_items,
            sell_items,
            is_sell,
            character,
            data,
        } => {
            add_button(ui, "Toggle shop", WidgetId(799), 200.0, 10.0, |_ui| {
                *is_sell = !*is_sell;
            });
            let desired = if *is_sell {
                NpcShopMode::Sell
            } else {
                NpcShopMode::Buy
            };
            if shop.shop.mode != Some(desired) {
                if *is_sell {
                    shop.shop.open_sell(100, sell_items.clone());
                } else {
                    shop.shop.open_buy(100, buy_items.clone());
                }
            }
            shop.build(ui, character, data);
        }
        State::Login { login } => {
            login.build(ui);
        }
        State::Chat {
            chat,
            character,
            data,
        } => {
            chat.build(ui, character, data);
        }
        State::NpcDialog {
            npc,
            character,
            data,
        } => {
            npc.build(ui, character, data);
        }
        State::ConfirmDialog { dialog, open } => {
            if *open {
                let result = dialog.build(ui);
                if result != ragnarok_ui_component::game::confirm_dialog::ConfirmResult::None {
                    *open = false;
                }
            }
        }
        State::NumberInput { dialog } => {
            dialog.build(ui);
        }
        State::ServerList { win } => {
            win.build(ui);
        }
        State::Equipment {
            equip,
            character,
            data,
        } => {
            equip.build(ui, character, data);
        }
        State::SystemMenu {
            menu,
            character,
            data,
        } => {
            add_button(ui, "Open system Dialog", WidgetId(599), 10.0, 10.0, |_ui| {
                menu.open = true;
            });
            menu.allow_escape_toggle = true;
            menu.build(ui, character, data);
        }
        State::CharSelect { win } => {
            win.build(ui);
        }
        State::ItemInfo {
            win,
            character,
            data,
            ..
        } => {
            let events = win.build(ui, character, data);
            for event in events {
                match event {
                    GameEvent::ShowCardInfo { item_id } => {
                        win.show_card(item_id, data);
                    }
                    GameEvent::ShowCardIllustration { item_id } => {
                        let name = data
                            .item_name
                            .as_ref()
                            .map(|t| t.get_name_or_id(item_id))
                            .unwrap_or_else(|| format!("Item #{item_id}"));
                        let illust_path = data
                            .card_illustration
                            .as_ref()
                            .and_then(|t| t.illustration_path(item_id));
                        if let Some(path) = illust_path {
                            win.show_illustration(item_id, name, path);
                        }
                    }
                    _ => {}
                }
            }
        }
        State::SkillTree {
            win,
            character,
            data,
        } => {
            win.build(ui, character, data);
        }
        State::CardInsert {
            dialog,
            character,
            data,
        } => {
            dialog.build(ui, character, data);
        }
        State::DialogContainerDemo { notification } => {
            let mut character = Character::new();
            notification.build(ui, &mut character, &DataTable::default());
        }
        State::HotkeyBarDemo {
            hotkey_win,
            character,
            data,
        } => {
            hotkey_win.build(ui, character, data);
        }
        State::BasicInfoDemo {
            win,
            character,
            data,
        } => {
            win.build(ui, character, data);
        }
        State::StatusDemo { win, character, data } => {
            win.build(ui, character, data);
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
                if let Some(id) = z_order_id(comp)
                    && !z_order.contains(&id)
                {
                    build_single(comp, ui);
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
    ui.flush_tooltips();
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
        item_type: ItemType::from_value(item_type as usize),
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
