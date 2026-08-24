// Force system allocator to match the host binary's allocator.
// Both sides must use the same heap for cross-FFI Vec/String operations.
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

use models::enums::EnumWithNumberValue;
use models::enums::class::JobName;
use models::enums::item::ItemType;
use ragnarok_ai::config::CompanionAiConfig;
use ragnarok_game::char_name::CharNameCache;
use ragnarok_game::character::Character;
use ragnarok_game::companion::{HomunculusState, MercenaryState};
use ragnarok_game::data_table::DataTable;
use ragnarok_game::data_table::card_illustration_table::CardIllustrationTable;
use ragnarok_game::data_table::item_description_table::ItemDescriptionTable;
use ragnarok_game::data_table::item_name_table::ItemNameTable;
use ragnarok_game::data_table::item_resource_table::ItemResourceTable;
use ragnarok_game::data_table::item_slot_count_table::ItemSlotCountTable;
use ragnarok_game::data_table::map_name_table::MapNameTable;
use ragnarok_game::data_table::map_position_table::MapPositionTable;
use ragnarok_game::data_table::skill_name_table::SkillNameTable;
use ragnarok_game::data_table::skill_tree_table::{SkillTreeEntry, SkillTreeTable};
use ragnarok_game::event::SkillInfo;
use ragnarok_game::event::{CharacterInfo, GameEvent, ServerInfo, VendorItem};
use ragnarok_game::friends::FriendList;
use ragnarok_game::guild::{
    Guild, GuildBanEntry, GuildMember, GuildPosition, GuildRelation, GuildSkill,
};
use ragnarok_game::item::Item;
use ragnarok_game::npc_shop::{NpcShopMode, ShopBuyItem, ShopSellItem};
use ragnarok_game::party::{Party, PartyMember};
use ragnarok_game::pet::PetState;
use ragnarok_game::quest::{Quest, QuestLog, QuestObjective};
use ragnarok_game::skill::{SkillEnum, SkillTargetType};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;
use ragnarok_ui_component::BuildCtx;
use ragnarok_ui_component::account::char_create_window::{CHAR_CREATE_WINDOW_ID, CharCreateWindow};
use ragnarok_ui_component::account::char_select_window::{CHAR_SELECT_WINDOW_ID, CharSelectWindow};
use ragnarok_ui_component::account::login_window::{LOGIN_WINDOW_ID, LoginWindow};
use ragnarok_ui_component::account::server_list_window::{SERVER_LIST_WINDOW_ID, ServerListWindow};
use ragnarok_ui_component::game::basic_info_window::{BASIC_INFO_WINDOW_ID, BasicInfoWindow};
use ragnarok_ui_component::game::book_window::{BOOK_WINDOW_ID, BookWindow};
use ragnarok_ui_component::game::card_insert_dialog::{
    CARD_INSERT_WINDOW_ID, CardInsertDialog, EligibleItem,
};
use ragnarok_ui_component::game::cart_select_window::{CART_SELECT_WINDOW_ID, CartSelectWindow};
use ragnarok_ui_component::game::cart_window::{CART_WINDOW_ID, CartWindow};
use ragnarok_ui_component::game::chat_room_board;
use ragnarok_ui_component::game::chat_room_create_window::{
    CHAT_ROOM_CREATE_WINDOW_ID, ChatRoomCreateWindow,
};
use ragnarok_ui_component::game::chat_room_member_window::{
    CHAT_ROOM_MEMBER_WINDOW_ID, ChatRoomMemberWindow,
};
use ragnarok_ui_component::game::chat_window::{CHAT_WINDOW_ID, ChatWindow};
use ragnarok_ui_component::game::companion_ai_config_window::{
    COMPANION_AI_CONFIG_WINDOW_ID, CompanionAiConfigWindow,
};
use ragnarok_ui_component::game::confirm_dialog::ConfirmDialog;
use ragnarok_ui_component::game::emotion_window::{EMOTION_WINDOW_ID, EmotionWindow};
use ragnarok_ui_component::game::equipment_window::{EQ_WINDOW_ID, EquipmentWindow};
use ragnarok_ui_component::game::graphic_options::{
    GRAPHIC_OPTIONS_WINDOW_ID, GraphicOptionsWindow,
};
use ragnarok_ui_component::game::guild_window::{GUILD_WINDOW_ID, GuildWindow};
use ragnarok_ui_component::game::homun_window::{HOMUN_WINDOW_ID, HomunWindow};
use ragnarok_ui_component::game::hotkey_bar::HotkeyBarWindow;
use ragnarok_ui_component::game::hotkey_config_window::{
    HOTKEY_CONFIG_WINDOW_ID, HotkeyConfigWindow,
};
use ragnarok_ui_component::game::input_dialog::{InputDialog, InputDialogConfig};
use ragnarok_ui_component::game::inventory_window::{INV_WINDOW_ID, InventoryWindow};
use ragnarok_ui_component::game::item_info_window::{ITEM_INFO_WINDOW_ID, ItemInfoWindow};
use ragnarok_ui_component::game::item_pickup_notification::ItemPickupNotification;
use ragnarok_ui_component::game::mailbox_window::{MAILBOX_WINDOW_ID, MailboxWindow};
use ragnarok_ui_component::game::mercenary_skill_window::{
    MERCENARY_SKILL_WINDOW_ID, MercenarySkillWindow,
};
use ragnarok_ui_component::game::mercenary_window::{MERCENARY_WINDOW_ID, MercenaryWindow};
use ragnarok_ui_component::game::minimap_window::{
    MINIMAP_WINDOW_ID, MarkerType, MinimapMarker, MinimapWindow, quest_marker_color,
};
use ragnarok_ui_component::game::monster_info_window::{MONSTER_INFO_WINDOW_ID, MonsterInfoWindow};
use ragnarok_ui_component::game::my_shop_window::{MY_SHOP_WINDOW_ID, MyShopWindow};
use ragnarok_ui_component::game::npc_dialog::{NPC_DIALOG_WINDOW_ID, NpcDialog};
use ragnarok_ui_component::game::npc_shop::NpcShop;
use ragnarok_ui_component::game::party_friends_window::{
    PARTY_FRIENDS_WINDOW_ID, PartyFriendsWindow,
};
use ragnarok_ui_component::game::pet_window::{PET_WINDOW_ID, PetWindow};
use ragnarok_ui_component::game::quest_window::{
    QUEST_DETAIL_WINDOW_ID, QUEST_WINDOW_ID, QuestDetailWindow, QuestWindow,
};
use ragnarok_ui_component::game::read_mail_window::{READ_MAIL_WINDOW_ID, ReadMailWindow};
use ragnarok_ui_component::game::shortcut_list_window::{
    SHORTCUT_LIST_WINDOW_ID, ShortcutListWindow,
};
use ragnarok_ui_component::game::skill_tree_window::{SKILL_WINDOW_ID, SkillTreeWindow};
use ragnarok_ui_component::game::status_window::{STATUS_WINDOW_ID, StatusWindow};
use ragnarok_ui_component::game::storage_password_window::{
    STORAGE_PASSWORD_WINDOW_ID, StoragePasswordMode, StoragePasswordWindow,
};
use ragnarok_ui_component::game::storage_window::{STORAGE_WINDOW_ID, StorageWindow};
use ragnarok_ui_component::game::system_menu::SystemMenu;
use ragnarok_ui_component::game::trade_window::{TRADE_WINDOW_ID, TradeWindow};
use ragnarok_ui_component::game::vending_board;
use ragnarok_ui_component::game::vending_setup_window::{
    VENDING_SETUP_WINDOW_ID, VendingSetupWindow,
};
use ragnarok_ui_component::game::vending_shop_window::{VENDING_SHOP_WINDOW_ID, VendingShopWindow};
use ragnarok_ui_component::game::world_map_window::{
    WORLD_MAP_TEX, WORLD_MAP_WINDOW_ID, WorldMapWindow,
};
use ragnarok_ui_component::helper::dialog_container::DialogContainer;
use ragnarok_ui_component::helper::head_board::BOARD_W;
use ragnarok_ui_component::{InGameWindow, Window};
use std::collections::HashMap;

const GAME_COMPONENTS: &[&str] = &[
    "inventory",
    "storage",
    "storage_password",
    "cart_select",
    "npc_shop",
    "npc_dialog",
    "equipment",
    "system_menu",
    "confirm_dialog",
    "number_input",
    "dialog_container",
    "item_info",
    "book",
    "skill_tree",
    "card_insert",
    "hotkey_bar",
    "basic_info",
    "status",
    "quest",
    "quest_detail",
    "graphic_options",
    "hotkey_config",
    "monster_info",
];
const MAP_COMPONENTS: &[&str] = &["world_map", "minimap"];
const SOCIAL_COMPONENTS: &[&str] = &[
    "inventory",
    "guild",
    "vending_board",
    "party",
    "emotion",
    "shortcut_list",
    "mailbox",
    "read_mail",
    "trade",
];
const CHAT_COMPONENTS: &[&str] = &[
    "chat",
    "chat_room_create",
    "chat_room_member",
    "chat_room_board",
];
const ACCOUNT_COMPONENTS: &[&str] = &["login", "server_list", "char_select", "char_create"];
const SHOP_COMPONENTS: &[&str] = &["cart", "vending_setup", "my_shop", "vending_buy"];
const COMPANION_COMPONENTS: &[&str] = &[
    "mercenary",
    "mercenary_skill",
    "homun",
    "companion_ai_config",
    "pet",
];

enum State {
    Inventory {
        inv: InventoryWindow,
        character: Character,
        data: DataTable,
    },
    Cart {
        win: CartWindow,
        character: Character,
        data: DataTable,
    },
    Storage {
        win: StorageWindow,
        character: Character,
        data: DataTable,
    },
    StoragePassword {
        win: StoragePasswordWindow,
        character: Character,
        data: DataTable,
    },
    Trade {
        win: TradeWindow,
        character: Character,
        data: DataTable,
    },
    Mailbox {
        win: MailboxWindow,
        character: Character,
        data: DataTable,
    },
    ReadMail {
        win: ReadMailWindow,
        character: Character,
        data: DataTable,
    },
    CartSelect {
        win: CartSelectWindow,
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
        dialog: InputDialog,
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
    CharCreate {
        win: CharCreateWindow,
    },
    ItemInfo {
        win: ItemInfoWindow,
        character: Character,
        data: DataTable,
        item: Item,
    },
    Book {
        win: BookWindow,
        character: Character,
        data: DataTable,
    },
    MonsterInfo {
        win: MonsterInfoWindow,
        character: Character,
        data: DataTable,
    },
    ChatRoomCreate {
        win: ChatRoomCreateWindow,
        character: Character,
        data: DataTable,
    },
    Emotion {
        win: EmotionWindow,
        character: Character,
        data: DataTable,
    },
    ShortcutList {
        win: ShortcutListWindow,
        character: Character,
        data: DataTable,
    },
    GraphicOptions {
        win: GraphicOptionsWindow,
        character: Character,
        data: DataTable,
    },
    HotkeyConfig {
        win: HotkeyConfigWindow,
        character: Character,
        data: DataTable,
    },
    Quest {
        win: QuestWindow,
        log: QuestLog,
        character: Character,
        data: DataTable,
    },
    QuestDetail {
        win: QuestDetailWindow,
        log: QuestLog,
        character: Character,
        data: DataTable,
    },
    ChatRoomMember {
        win: ChatRoomMemberWindow,
        character: Character,
        data: DataTable,
    },
    Minimap {
        win: MinimapWindow,
        character: Character,
        data: DataTable,
    },
    WorldMap {
        win: WorldMapWindow,
        party: Party,
        local_aid: u32,
        character: Character,
        data: DataTable,
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
    PartyDemo {
        win: PartyFriendsWindow,
        party: Party,
        local_aid: u32,
        character: Character,
        data: DataTable,
    },
    GuildDemo {
        win: GuildWindow,
        guild: Guild,
        local_gid: u32,
        character: Character,
        data: DataTable,
    },
    VendingSetup {
        win: VendingSetupWindow,
        character: Character,
        data: DataTable,
    },
    MyShop {
        win: MyShopWindow,
        src: Vec<(VendorItem, String)>,
        shop_name: String,
        character: Character,
        data: DataTable,
    },
    VendingBuy {
        win: VendingShopWindow,
        src: Vec<(VendorItem, String)>,
        character: Character,
        data: DataTable,
    },
    VendingBoard {
        container: DialogContainer,
        name: String,
    },
    ChatRoomBoard {
        container: DialogContainer,
        atype: u8,
        title: String,
        cur: i16,
        max: i16,
    },
    Mercenary {
        win: MercenaryWindow,
        merc: MercenaryState,
    },
    MercenarySkill {
        win: MercenarySkillWindow,
        merc: MercenaryState,
    },
    Pet {
        win: PetWindow,
        pet: PetState,
    },
    Homun {
        win: HomunWindow,
        homun: HomunculusState,
    },
    CompanionAiConfig {
        win: CompanionAiConfigWindow,
        config: ragnarok_ai::config::CompanionAiConfig,
    },
    FallbackGallery {
        name_field: TextInput,
    },
    Category {
        components: Vec<State>,
    },
}

fn demo_mercenary_skills() -> Vec<SkillInfo> {
    let skill = |skill, level, sp_cost| SkillInfo {
        skill,
        level,
        sp_cost,
        attack_range: 1,
        upgradable: false,
        skill_target_type: SkillTargetType::Target,
    };
    vec![
        skill(SkillEnum::SmMagnum, 5, 30),
        skill(SkillEnum::KnBrandishspear, 5, 12),
        skill(SkillEnum::MerRegain, 1, 10),
        skill(SkillEnum::SmProvoke, 5, 8),
    ]
}

fn demo_mercenary() -> MercenaryState {
    let mut merc = MercenaryState::new(150001);
    merc.name = "Alice".into();
    merc.level = 62;
    merc.hp = 10000;
    merc.max_hp = 10000;
    merc.sp = 221;
    merc.max_sp = 221;
    merc.atk = 801;
    merc.matk = 374;
    merc.hit = 312;
    merc.critical = 44;
    merc.def = 56;
    merc.mdef = 23;
    merc.flee = 126;
    merc.aspd = 0;
    merc.atk_range = 1;
    merc.faith = 0;
    merc.expire_date = 1_255_130_880;
    merc.calls = 0;
    merc.kills = 442;
    merc.skills = demo_mercenary_skills();
    merc
}

fn demo_homunculus() -> HomunculusState {
    let mut homun = HomunculusState::new(160001);
    homun.name = "Big Chungus".into();
    homun.renamed = true;
    homun.level = 168;
    homun.hp = 409973;
    homun.max_hp = 409973;
    homun.sp = 7366;
    homun.max_sp = 7366;
    homun.exp = 18087058;
    homun.max_exp = 30000000;
    homun.hunger = 28;
    homun.intimacy = 50;
    homun.atk = 300;
    homun.matk = 1452;
    homun.hit = 518;
    homun.critical = 51;
    homun.def = 644;
    homun.mdef = 420;
    homun.flee = 384;
    homun.aspd = 190;
    homun.atk_range = 1;
    homun.skill_points = 2;
    homun
}

fn demo_pet() -> PetState {
    PetState {
        gid: Some(400123),
        job: 1002,
        name: "Poring".into(),
        renamed: false,
        level: 1,
        hunger: 80,
        intimacy: 920,
        accessory: 10013,
        egg_index: None,
        capture_pending: false,
    }
}

fn demo_quest_log() -> QuestLog {
    let obj = |mob_id, name: &str, current, required| QuestObjective {
        mob_id,
        name: name.to_string(),
        current,
        required,
    };
    QuestLog {
        quests: vec![
            Quest {
                id: 1000,
                active: true,
                end_time: None,
                objectives: vec![obj(1002, "Poring", 3, 10), obj(1063, "Lunatic", 1, 5)],
            },
            Quest {
                id: 1001,
                active: true,
                end_time: Some(1_735_689_600),
                objectives: vec![obj(1007, "Fabre", 0, 1)],
            },
            Quest {
                id: 1002,
                active: false,
                end_time: None,
                objectives: vec![obj(1113, "Drops", 2, 2)],
            },
        ],
    }
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
        "cart" => {
            let mut character = Character::new();
            character.cart.open();
            character.cart.set_count_info(2850, 8000, 6, 100);
            for item in inventory_test_items() {
                character.cart.add_item(item);
            }
            State::Cart {
                win: CartWindow::new(),
                character,
                data: DataTable::new(),
            }
        }
        "storage" => {
            let mut character = Character::new();
            character.storage.open_with_pending(42, 600);
            for item in storage_test_items() {
                character.storage.add_item(item);
            }
            State::Storage {
                win: StorageWindow::new(),
                character,
                data: DataTable::new(),
            }
        }
        "storage_password" => {
            let mut win = StoragePasswordWindow::new();
            win.open_with(StoragePasswordMode::SetNew);
            State::StoragePassword {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "trade" => {
            let mut character = Character::new();
            character.name = "MyChar".into();
            character.base_level = 60;
            for item in inventory_test_items() {
                character.inventory.add_item(item);
            }
            character.trade.begin("TradePartner".into(), 2000, 55, 60);
            for item in storage_test_items().into_iter().take(3) {
                character.trade.add_other_item(item);
            }
            character.trade.set_other_zeny(12345);
            State::Trade {
                win: TradeWindow::new(),
                character,
                data: DataTable::new(),
            }
        }
        "mailbox" => {
            let mut character = Character::new();
            character.mail.window_open = true;
            character.mail.inbox = mail_test_inbox();
            for item in inventory_test_items() {
                character.inventory.add_item(item);
            }
            State::Mailbox {
                win: MailboxWindow::new(),
                character,
                data: DataTable::new(),
            }
        }
        "read_mail" => {
            use ragnarok_game::mail::{MailItem, OpenedMail};
            let mut character = Character::new();
            character.mail.read_open = true;
            character.mail.opened = Some(OpenedMail {
                mail_id: 2001,
                title: "About your order".to_string(),
                sender: "Kafra".to_string(),
                zeny: 1_250_000,
                item: Some(MailItem {
                    nameid: 501,
                    amount: 5,
                    item_type: 0,
                    identified: true,
                    damaged: false,
                    refine: 0,
                    cards: [0; 4],
                }),
                body: "Thank you for your purchase. Here is your reward, please enjoy it and come back soon.".to_string(),
            });
            State::ReadMail {
                win: ReadMailWindow::new(),
                character,
                data: DataTable::new(),
            }
        }
        "vending_setup" => {
            let mut character = Character::new();
            character.cart.open();
            character.cart.set_count_info(2850, 8000, 6, 100);
            for item in inventory_test_items() {
                character.cart.add_item(item);
            }
            let mut win = VendingSetupWindow::new();
            win.open(12);
            State::VendingSetup {
                win,
                character,
                data: DataTable::new(),
            }
        }
        "my_shop" => {
            let src = vending_test_stock();
            let shop_name = "Cheap Potions!".to_string();
            let mut win = MyShopWindow::new();
            win.open(
                shop_name.clone(),
                src.iter()
                    .map(|(it, n)| (it.clone(), n.clone(), None))
                    .collect(),
            );
            State::MyShop {
                win,
                src,
                shop_name,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "vending_buy" => {
            let src = vending_test_stock();
            let mut win = VendingShopWindow::new();
            win.open(
                2000101,
                1,
                "store02".to_string(),
                src.iter()
                    .map(|(it, n)| (it.clone(), n.clone(), None))
                    .collect(),
            );
            State::VendingBuy {
                win,
                src,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "vending_board" => State::VendingBoard {
            container: DialogContainer::new(),
            name: "+7 Gears".to_string(),
        },
        "chat_room_board" => State::ChatRoomBoard {
            container: DialogContainer::new(),
            atype: 1,
            title: "Newbies welcome!".to_string(),
            cur: 3,
            max: 20,
        },
        "cart_select" => {
            let mut character = Character::new();
            character.base_level = 99;
            character.cart_design = Some(1);
            let mut win = CartSelectWindow::new();
            win.open();
            State::CartSelect {
                win,
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
            chat.add_whisper_in("Lidia".into(), "are you online?".into());
            chat.add_whisper_out("Gandalf".into(), "on my way".into());
            chat.add_whisper_in("Zephyr".into(), "meet at Prontera fountain".into());
            for name in ["Lidia", "Gandalf", "Zephyr", "Mint", "Raki"] {
                chat.remember_whisper(name.into());
            }
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
            dialog: ConfirmDialog::new(),
            open: false,
        },
        "number_input" => State::NumberInput {
            dialog: InputDialog::new(
                InputDialogConfig {
                    label: Some("How many (max 99)?".to_string()),
                    show_cancel: false,
                    escape_cancels: true,
                    default_value: "99".to_string(),
                    max_len: 6,
                    numeric_only: true,
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
                    base_exp: 1145316,
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
                    effect_state: 0,
                    zeny: 0,
                },
                CharacterInfo {
                    gid: 2,
                    name: "Wizard".into(),
                    class: 9,
                    base_level: 45,
                    base_exp: 823400,
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
                    effect_state: 0,
                    zeny: 0,
                },
                CharacterInfo {
                    gid: 3,
                    name: "Hunter".into(),
                    class: 11,
                    base_level: 60,
                    base_exp: 1502990,
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
                    effect_state: 0,
                    zeny: 0,
                },
            ];
            State::CharSelect {
                win: CharSelectWindow::new(characters),
            }
        }
        "char_create" => {
            let mut win = CharCreateWindow::new(0, true);
            win.show_skin_toggle = true;
            State::CharCreate { win }
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
            // resource_name: None - resolved from GRF's ItemResourceTable in grf_init_single
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
            win.show(&bow, &data, &CharNameCache::default(), false);
            State::ItemInfo {
                win,
                character: Character::new(),
                data,
                item: bow,
            }
        }
        "book" => {
            let mut win = BookWindow::new();
            win.show(ragnarok_game::book::BookContent {
                bg_color: [0.96, 0.96, 0.86],
                lines: vec![
                    "^000088The Book of Ymir^000000".to_string(),
                    String::new(),
                    "Long ago, the gods forged the world from the void.".to_string(),
                    "This is a demo book used for hot-reload preview.".to_string(),
                ],
            });
            State::Book {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "monster_info" => {
            let mut win = MonsterInfoWindow::new();
            win.show(ragnarok_game::monster_info::MonsterInfo {
                name: "Poring".to_string(),
                job: 1002,
                level: 1,
                size: 0,
                hp: 50,
                def: 0,
                race: 3,
                mdef: 5,
                property: 21,
                resistances: [100, 100, 100, 100, 100, 100, 100, 100, 100],
            });
            State::MonsterInfo {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "chat_room_create" => {
            let mut win = ChatRoomCreateWindow::new();
            win.open_create();
            State::ChatRoomCreate {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "emotion" => {
            let mut win = EmotionWindow::new();
            win.toggle();
            State::Emotion {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "shortcut_list" => {
            let mut win = ShortcutListWindow::new();
            win.set_bindings(&ragnarok_game::emotion::default_shortcut_commands());
            win.toggle();
            State::ShortcutList {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "graphic_options" => {
            let mut win = GraphicOptionsWindow::new();
            win.set_values(
                100.0,
                false,
                false,
                true,
                ragnarok_game::display::DisplayOptions::default(),
                ragnarok_game::cursor::MouseSnapPrefs::default(),
                false,
                false,
            );
            win.toggle();
            State::GraphicOptions {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "hotkey_config" => {
            let mut win = HotkeyConfigWindow::new();
            win.set_bindings(
                &ragnarok_game::keybinding::KeyBindings::defaults(),
                &ragnarok_game::keybinding::EmotionKeys::default(),
            );
            win.toggle();
            State::HotkeyConfig {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "quest" => {
            let mut win = QuestWindow::new();
            win.toggle();
            State::Quest {
                win,
                log: demo_quest_log(),
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "quest_detail" => {
            let log = demo_quest_log();
            let quest_id = log.quests[0].id;
            let mut win = QuestDetailWindow::new();
            win.open(quest_id);
            State::QuestDetail {
                win,
                log,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "minimap" => {
            let mut win = MinimapWindow::new();
            win.map_name = Some("prontera".to_string());
            win.map_width = 400;
            win.map_height = 400;
            win.player_position = Some((156.0, 191.0));
            win.player_direction = 4;
            win.set_map_texture(Some(ragnarok_resources::ui::minimap::PRONTERA.to_string()));
            win.entity_markers = vec![
                MinimapMarker {
                    x: 150.0,
                    y: 200.0,
                    marker_type: MarkerType::PartyMember { leader: true },
                    name: Some("Walkiry".to_string()),
                },
                MinimapMarker {
                    x: 170.0,
                    y: 180.0,
                    marker_type: MarkerType::PartyMember { leader: false },
                    name: Some("Lidia".to_string()),
                },
                MinimapMarker {
                    x: 120.0,
                    y: 160.0,
                    marker_type: MarkerType::GuildMember,
                    name: None,
                },
                MinimapMarker {
                    x: 134.0,
                    y: 221.0,
                    marker_type: MarkerType::Mark([1.0, 0.0, 0.0]),
                    name: None,
                },
                MinimapMarker {
                    x: 200.0,
                    y: 150.0,
                    marker_type: MarkerType::Mark(quest_marker_color(2)),
                    name: None,
                },
            ];
            State::Minimap {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "world_map" => {
            let local_aid = 2000001;
            let member = |aid: u32, name: &str, map: &str, x: u16, y: u16| PartyMember {
                aid,
                name: name.to_string(),
                map: map.to_string(),
                leader: aid == local_aid,
                online: true,
                hp: None,
                max_hp: None,
                x,
                y,
                has_live_position: true,
            };
            let mut party = Party::new("Adventurers".to_string());
            party.members = vec![
                member(local_aid, "Walkiry", "prontera.gat", 156, 191),
                member(2000002, "Lidia", "prontera.gat", 60, 340),
                member(2000003, "Garm", "payon.gat", 0, 0),
                member(2000004, "Sohee", "geffen.gat", 0, 0),
            ];
            let mut data = DataTable::new();
            data.map_position = Some(MapPositionTable::parse(DEMO_MAP_POSITIONS.as_bytes()));
            data.map_name = Some(MapNameTable::parse(DEMO_MAP_NAMES.as_bytes()));
            let mut win = WorldMapWindow::new();
            win.open();
            win.current_map = Some("prontera".to_string());
            win.map_width = 400;
            win.map_height = 400;
            win.player_position = Some((156.0, 191.0));
            State::WorldMap {
                win,
                party,
                local_aid,
                character: Character::new(),
                data,
            }
        }
        "chat_room_member" => {
            use ragnarok_game::chat_room::ChatRoomMember;
            let mut win = ChatRoomMemberWindow::new();
            win.open_joined(
                5,
                "Trade Room",
                20,
                true,
                vec![
                    ChatRoomMember {
                        name: "Owner".to_string(),
                        is_owner: true,
                    },
                    ChatRoomMember {
                        name: "Guest".to_string(),
                        is_owner: false,
                    },
                ],
                "Owner",
            );
            use ragnarok_ui_component::game::chat_room_member_window::{
                OTHER_MSG_COLOR, OWN_MSG_COLOR, SYSTEM_MSG_COLOR,
            };
            win.push_message("You entered the room.".to_string(), SYSTEM_MSG_COLOR);
            win.push_message("Owner : Welcome, everyone!".to_string(), OTHER_MSG_COLOR);
            win.push_message(
                "Guest : hello, selling gear here".to_string(),
                OWN_MSG_COLOR,
            );
            State::ChatRoomMember {
                win,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "skill_tree" => {
            use ragnarok_game::skill::{SkillData, SkillEnum, SkillTargetType};

            let mut character = Character::new();
            character.skill_point = 5;
            character.skills.open();
            character.skills.set_skills(vec![
                SkillData {
                    skill: SkillEnum::SmSword,
                    level: 10,
                    selected_level: 10,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: false,
                    skill_target_type: SkillTargetType::Passive,
                },
                SkillData {
                    skill: SkillEnum::SmRecovery,
                    level: 5,
                    selected_level: 5,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Passive,
                },
                SkillData {
                    skill: SkillEnum::SmBash,
                    level: 5,
                    selected_level: 5,
                    sp_cost: 8,
                    attack_range: 1,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Target,
                },
                SkillData {
                    skill: SkillEnum::SmProvoke,
                    level: 3,
                    selected_level: 3,
                    sp_cost: 4,
                    attack_range: 9,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Target,
                },
                SkillData {
                    skill: SkillEnum::SmEndure,
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
            use ragnarok_game::skill::{SkillData, SkillEnum, SkillTargetType};

            let mut character = Character::new();

            character.inventory.toggle();
            for item in inventory_test_items() {
                character.inventory.add_item(item);
            }

            character.skill_point = 5;
            character.skills.open();
            character.skills.set_skills(vec![
                SkillData {
                    skill: SkillEnum::AcOwl,
                    level: 10,
                    selected_level: 10,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: false,
                    skill_target_type: SkillTargetType::Passive,
                },
                SkillData {
                    skill: SkillEnum::AcVulture,
                    level: 10,
                    selected_level: 10,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: false,
                    skill_target_type: SkillTargetType::Passive,
                },
                SkillData {
                    skill: SkillEnum::AcConcentration,
                    level: 10,
                    selected_level: 10,
                    sp_cost: 15,
                    attack_range: 0,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Ground,
                },
                SkillData {
                    skill: SkillEnum::AcDouble,
                    level: 10,
                    selected_level: 10,
                    sp_cost: 12,
                    attack_range: 9,
                    upgradable: true,
                    skill_target_type: SkillTargetType::Target,
                },
                SkillData {
                    skill: SkillEnum::AcShower,
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
                    skill: SkillEnum::AcDouble,
                    level: 10,
                },
            );
            character
                .hotkeys
                .set_slot(1, HotkeySlotContent::Item { item_id: 501 });

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

            let mut hotkey_win = HotkeyBarWindow::new();
            hotkey_win.top_margin = 40.0;
            State::HotkeyBarDemo {
                hotkey_win,
                character,
                data,
            }
        }
        "basic_info" => {
            let mut character = Character::new();
            character.name = "Walkiry".into();
            character.base_level = 42;
            character.job_level = 30;
            character.hp = 2350;
            character.max_hp = 3200;
            character.sp = 5;
            character.max_sp = 120;
            character.base_exp = 185000;
            character.next_base_exp = 300000;
            character.job_exp = 42000;
            character.next_job_exp = 80000;
            character.inventory.weight = 12500;
            character.inventory.max_weight = 30000;
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
            character.base_level = 42;
            character.job_level = 30;
            character.status_point = 12;
            character.skill_point = 3;
            character.str = 35;
            character.str_bonus = 5;
            character.str_cost = 6;
            character.agi = 22;
            character.agi_bonus = 0;
            character.agi_cost = 4;
            character.vit = 28;
            character.vit_bonus = 2;
            character.vit_cost = 5;
            character.int = 10;
            character.int_bonus = 0;
            character.int_cost = 3;
            character.dex = 18;
            character.dex_bonus = -1;
            character.dex_cost = 4;
            character.luk = 5;
            character.luk_bonus = 0;
            character.luk_cost = 2;
            character.atk1 = 250;
            character.atk2 = 30;
            character.matk1 = 35;
            character.matk2 = 40;
            character.def1 = 18;
            character.def2 = 5;
            character.mdef1 = 4;
            character.mdef2 = 2;
            character.hit = 122;
            character.flee1 = 95;
            character.flee2 = 10;
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
        "party" => {
            let local_aid = 2000001;
            let mut party = Party::new("Adventurers".to_string());
            party.exp_share = true;
            let member = |aid, name: &str, map: &str, leader, online, hp, max_hp| PartyMember {
                aid,
                name: name.to_string(),
                map: map.to_string(),
                leader,
                online,
                hp: Some(hp),
                max_hp: Some(max_hp),
                x: 0,
                y: 0,
                has_live_position: false,
            };
            party.members = vec![
                member(local_aid, "Walkiry", "prontera.gat", true, true, 3200, 3200),
                member(2000002, "Lidia", "prontera.gat", false, true, 800, 2400),
                member(2000003, "Garm", "payon_dun01.gat", false, true, 120, 1800),
                member(2000004, "Sohee", "geffen.gat", false, false, 0, 1500),
            ];
            let mut win = PartyFriendsWindow::new();
            win.open_party_tab();
            State::PartyDemo {
                win,
                party,
                local_aid,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "guild" => {
            let local_gid = 2000001;
            let mut guild = Guild {
                gdid: 42,
                name: "Prontera Knights".to_string(),
                level: 12,
                exp: 45_000,
                max_exp: 100_000,
                member_num: 3,
                max_member_num: 36,
                avg_level: 74,
                point: 320,
                master_name: "Walkiry".to_string(),
                manage_land: "prontera".to_string(),
                notice_subject: "Woe this Saturday".to_string(),
                notice_body: "Meet at the guild dungeon at 20:00. Bring supplies and \
                              be ready for the emperium break."
                    .to_string(),
                ..Default::default()
            };
            guild.positions = vec![
                GuildPosition {
                    id: 0,
                    name: "Master".to_string(),
                    right: 0x111,
                    ranking: 0,
                    pay_rate: 50,
                },
                GuildPosition {
                    id: 1,
                    name: "Officer".to_string(),
                    right: 0x001,
                    ranking: 1,
                    pay_rate: 10,
                },
                GuildPosition {
                    id: 2,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 0,
                },
                GuildPosition {
                    id: 3,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 10,
                },
                GuildPosition {
                    id: 4,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 10,
                },
                GuildPosition {
                    id: 5,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 10,
                },
                GuildPosition {
                    id: 6,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 10,
                },
                GuildPosition {
                    id: 7,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 10,
                },
                GuildPosition {
                    id: 8,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 20,
                },
                GuildPosition {
                    id: 9,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 10,
                },
                GuildPosition {
                    id: 10,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 10,
                },
                GuildPosition {
                    id: 11,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 20,
                },
                GuildPosition {
                    id: 12,
                    name: "Member".to_string(),
                    right: 0x000,
                    ranking: 2,
                    pay_rate: 30,
                },
            ];
            #[allow(clippy::too_many_arguments)]
            let gmember = |gid: u32,
                           name: &str,
                           job: i16,
                           level: i16,
                           position_id: i32,
                           pos_name: &str,
                           online: bool,
                           note: &str,
                           contrib: i32| GuildMember {
                aid: gid,
                gid,
                name: name.to_string(),
                job,
                level,
                position_id,
                position_name: pos_name.to_string(),
                online,
                note: note.to_string(),
                contribution_exp: contrib,
                ..Default::default()
            };
            guild.members = vec![
                gmember(local_gid, "Walkiry", 7, 99, 0, "Master", true, "GM", 1500),
                gmember(2000002, "Lidia", 4, 88, 1, "Officer", true, "2nd", 900),
                gmember(2000003, "Garm", 12, 71, 2, "Member", false, "", 300),
                gmember(2000004, "Sohee", 8, 65, 2, "Member", true, "alt", 250),
                gmember(2000005, "Poring", 1, 42, 2, "Member", false, "", 60),
                gmember(2000005, "Poring", 1, 42, 2, "Member", false, "", 60),
                gmember(2000005, "Poring", 1, 42, 2, "Member", false, "", 60),
                gmember(2000005, "Poring", 1, 42, 2, "Member", false, "", 60),
            ];
            guild.relations = vec![
                GuildRelation {
                    gdid: 7,
                    name: "Geffen Mages".to_string(),
                    relation: 0,
                },
                GuildRelation {
                    gdid: 9,
                    name: "Payon Archers".to_string(),
                    relation: 1,
                },
            ];
            guild.skill_point = 2;
            guild.skills = vec![
                GuildSkill {
                    skill: SkillEnum::GdApproval,
                    level: 1,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: false,
                    passive: true,
                },
                GuildSkill {
                    skill: SkillEnum::GdGuardup,
                    level: 0,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: true,
                    passive: false,
                },
                GuildSkill {
                    skill: SkillEnum::GdExtension,
                    level: 2,
                    sp_cost: 0,
                    attack_range: 0,
                    upgradable: true,
                    passive: true,
                },
            ];
            guild.ban_list = vec![
                GuildBanEntry {
                    char_name: "Traitor".to_string(),
                    reason: "Left mid-WoE".to_string(),
                    ..Default::default()
                },
                GuildBanEntry {
                    char_name: "Spy".to_string(),
                    reason: "Enemy alt".to_string(),
                    ..Default::default()
                },
            ];
            let mut win = GuildWindow::new();
            win.toggle();
            State::GuildDemo {
                win,
                guild,
                local_gid,
                character: Character::new(),
                data: DataTable::new(),
            }
        }
        "mercenary" => {
            let mut win = MercenaryWindow::new();
            win.set_visible(true);
            State::Mercenary {
                win,
                merc: demo_mercenary(),
            }
        }
        "mercenary_skill" => {
            let mut win = MercenarySkillWindow::new();
            win.set_visible(true);
            State::MercenarySkill {
                win,
                merc: demo_mercenary(),
            }
        }
        "homun" => {
            let mut win = HomunWindow::new();
            win.set_visible(true);
            State::Homun {
                win,
                homun: demo_homunculus(),
            }
        }
        "pet" => {
            let mut win = PetWindow::new();
            win.set_visible(true);
            State::Pet {
                win,
                pet: demo_pet(),
            }
        }
        "companion_ai_config" => {
            let mut win = CompanionAiConfigWindow::new();
            win.set_visible(true);
            State::CompanionAiConfig {
                win,
                config: ragnarok_ai::config::CompanionAiConfig::default(),
            }
        }
        "fallback" => State::FallbackGallery {
            name_field: TextInput::new(23, false),
        },
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
        "shop" => State::Category {
            components: SHOP_COMPONENTS.iter().map(|n| create_single(n)).collect(),
        },
        "map" => State::Category {
            components: MAP_COMPONENTS.iter().map(|n| create_single(n)).collect(),
        },
        "social" => State::Category {
            components: SOCIAL_COMPONENTS.iter().map(|n| create_single(n)).collect(),
        },
        "chat" => State::Category {
            components: CHAT_COMPONENTS.iter().map(|n| create_single(n)).collect(),
        },
        "companion" => State::Category {
            components: COMPANION_COMPONENTS
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
        State::Cart { win, character, .. } => {
            if let Some(table) = table {
                character.cart.resolve_resource_names(table);
            }
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::Storage { win, character, .. } => {
            if let Some(table) = table {
                character.storage.resolve_resource_names(table);
            }
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::Trade { win, character, .. } => {
            if let Some(table) = table {
                character.inventory.resolve_resource_names(table);
            }
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::Mailbox {
            win,
            character,
            data,
        } => {
            if let Some(table) = table {
                character.inventory.resolve_resource_names(table);
                if data.item_resource.is_none() {
                    data.item_resource = Some(item_resource_from_ids(
                        character.inventory.all_items().iter().map(|i| i.item_id),
                        table,
                    ));
                }
            }
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::ReadMail {
            win,
            character,
            data,
        } => {
            if let Some(table) = table
                && data.item_resource.is_none()
            {
                let nameid = character
                    .mail
                    .opened
                    .as_ref()
                    .and_then(|o| o.item.as_ref())
                    .map(|it| it.nameid);
                data.item_resource = Some(item_resource_from_ids(nameid, table));
            }
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::CartSelect { win, .. } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::StoragePassword { win, .. } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::VendingSetup { win, character, .. } => {
            if let Some(table) = table {
                character.cart.resolve_resource_names(table);
            }
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::MyShop {
            win,
            src,
            shop_name,
            ..
        } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
            if let Some(table) = table {
                win.open(shop_name.clone(), resolve_stock_icons(src, table));
            }
        }
        State::VendingBuy { win, src, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
            if let Some(table) = table {
                win.open(
                    2000101,
                    1,
                    "store02".to_string(),
                    resolve_stock_icons(src, table),
                );
            }
        }
        State::VendingBoard { container, .. } => {
            container.has_grf_textures = true;
            container.set_texture_sizes(size_fn);
        }
        State::ChatRoomBoard { container, .. } => {
            container.has_grf_textures = true;
            container.set_texture_sizes(size_fn);
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
        State::CharCreate { win } => {
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
                win.show(item, data, &CharNameCache::default(), false);
            }
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::SkillTree { win, .. } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::Book { win, .. } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::MonsterInfo { win, .. } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::ChatRoomCreate { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::Emotion { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::ShortcutList { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::GraphicOptions { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::HotkeyConfig { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::Quest { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::QuestDetail { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::ChatRoomMember { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::Minimap { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::WorldMap { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
            win.texture_loaded(WORLD_MAP_TEX, true);
            win.texture_loaded(&WorldMapWindow::minimap_texture_path("prontera"), true);
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
        State::PartyDemo { win, .. } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::GuildDemo { win, .. } => {
            win.has_grf_textures = true;
            win.set_texture_sizes(size_fn);
        }
        State::Mercenary { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::MercenarySkill { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::Pet { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::Homun { win, .. } => {
            win.set_has_grf_textures(true);
            win.set_texture_sizes(size_fn);
        }
        State::CompanionAiConfig { win, .. } => {
            win.has_grf_textures = true;
        }
        State::FallbackGallery { .. } => {}
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
        State::Login { .. } => Some(LOGIN_WINDOW_ID),
        State::ServerList { .. } => Some(SERVER_LIST_WINDOW_ID),
        State::CharSelect { .. } => Some(CHAR_SELECT_WINDOW_ID),
        State::CharCreate { .. } => Some(CHAR_CREATE_WINDOW_ID),
        State::Chat { .. } => Some(CHAT_WINDOW_ID),
        State::Inventory { .. } => Some(INV_WINDOW_ID),
        State::Cart { .. } => Some(CART_WINDOW_ID),
        State::Storage { .. } => Some(STORAGE_WINDOW_ID),
        State::Trade { .. } => Some(TRADE_WINDOW_ID),
        State::Mailbox { .. } => Some(MAILBOX_WINDOW_ID),
        State::ReadMail { .. } => Some(READ_MAIL_WINDOW_ID),
        State::CartSelect { .. } => Some(CART_SELECT_WINDOW_ID),
        State::StoragePassword { .. } => Some(STORAGE_PASSWORD_WINDOW_ID),
        State::VendingSetup { .. } => Some(VENDING_SETUP_WINDOW_ID),
        State::MyShop { .. } => Some(MY_SHOP_WINDOW_ID),
        State::VendingBuy { .. } => Some(VENDING_SHOP_WINDOW_ID),
        State::VendingBoard { .. } => None,
        State::NpcDialog { .. } => Some(NPC_DIALOG_WINDOW_ID),
        State::Equipment { .. } => Some(EQ_WINDOW_ID),
        State::SkillTree { .. } => Some(SKILL_WINDOW_ID),
        State::Book { .. } => Some(BOOK_WINDOW_ID),
        State::MonsterInfo { .. } => Some(MONSTER_INFO_WINDOW_ID),
        State::ChatRoomCreate { .. } => Some(CHAT_ROOM_CREATE_WINDOW_ID),
        State::Emotion { .. } => Some(EMOTION_WINDOW_ID),
        State::ShortcutList { .. } => Some(SHORTCUT_LIST_WINDOW_ID),
        State::GraphicOptions { .. } => Some(GRAPHIC_OPTIONS_WINDOW_ID),
        State::HotkeyConfig { .. } => Some(HOTKEY_CONFIG_WINDOW_ID),
        State::Quest { .. } => Some(QUEST_WINDOW_ID),
        State::QuestDetail { .. } => Some(QUEST_DETAIL_WINDOW_ID),
        State::WorldMap { .. } => Some(WORLD_MAP_WINDOW_ID),
        State::Minimap { .. } => Some(MINIMAP_WINDOW_ID),
        State::ChatRoomMember { .. } => Some(CHAT_ROOM_MEMBER_WINDOW_ID),
        State::StatusDemo { .. } => Some(STATUS_WINDOW_ID),
        State::PartyDemo { .. } => Some(PARTY_FRIENDS_WINDOW_ID),
        State::GuildDemo { .. } => Some(GUILD_WINDOW_ID),
        State::Mercenary { .. } => Some(MERCENARY_WINDOW_ID),
        State::MercenarySkill { .. } => Some(MERCENARY_SKILL_WINDOW_ID),
        State::Homun { .. } => Some(HOMUN_WINDOW_ID),
        State::Pet { .. } => Some(PET_WINDOW_ID),
        State::CompanionAiConfig { .. } => Some(COMPANION_AI_CONFIG_WINDOW_ID),
        _ => None,
    }
}

/// The draggable, `window_at`-based windows a component contributes to the
/// packer, each with its nominal size. Most states own a single `Window`;
/// composites (the NPC shop) return several. Fixed bars, head boards, and
/// centered modal dialogs return nothing here — they position themselves
/// (dialogs are seeded separately by `seed_modal_layout`).
fn gallery_windows(state: &State) -> Vec<(WidgetId, (f32, f32))> {
    let single: Option<(WidgetId, &dyn Window)> = match state {
        State::Login { login, .. } => Some((LOGIN_WINDOW_ID, login)),
        State::ServerList { win, .. } => Some((SERVER_LIST_WINDOW_ID, win)),
        State::CharSelect { win, .. } => Some((CHAR_SELECT_WINDOW_ID, win)),
        State::CharCreate { win, .. } => Some((CHAR_CREATE_WINDOW_ID, win)),
        State::Inventory { inv, .. } => Some((INV_WINDOW_ID, inv)),
        State::Equipment { equip, .. } => Some((EQ_WINDOW_ID, equip)),
        State::StatusDemo { win, .. } => Some((STATUS_WINDOW_ID, win)),
        State::BasicInfoDemo { win, .. } => Some((BASIC_INFO_WINDOW_ID, win)),
        State::SkillTree { win, .. } => Some((SKILL_WINDOW_ID, win)),
        State::PartyDemo { win, .. } => Some((PARTY_FRIENDS_WINDOW_ID, win)),
        State::GuildDemo { win, .. } => Some((GUILD_WINDOW_ID, win)),
        State::ItemInfo { win, .. } => Some((ITEM_INFO_WINDOW_ID, win)),
        State::Book { win, .. } => Some((BOOK_WINDOW_ID, win)),
        State::MonsterInfo { win, .. } => Some((MONSTER_INFO_WINDOW_ID, win)),
        State::ChatRoomCreate { win, .. } => Some((CHAT_ROOM_CREATE_WINDOW_ID, win)),
        State::Emotion { win, .. } => Some((EMOTION_WINDOW_ID, win)),
        State::ShortcutList { win, .. } => Some((SHORTCUT_LIST_WINDOW_ID, win)),
        State::GraphicOptions { win, .. } => Some((GRAPHIC_OPTIONS_WINDOW_ID, win)),
        State::HotkeyConfig { win, .. } => Some((HOTKEY_CONFIG_WINDOW_ID, win)),
        State::Quest { win, .. } => Some((QUEST_WINDOW_ID, win)),
        State::QuestDetail { win, .. } => Some((QUEST_DETAIL_WINDOW_ID, win)),
        State::WorldMap { win, .. } => Some((WORLD_MAP_WINDOW_ID, win)),
        State::ChatRoomMember { win, .. } => Some((CHAT_ROOM_MEMBER_WINDOW_ID, win)),
        State::Cart { win, .. } => Some((CART_WINDOW_ID, win)),
        State::Storage { win, .. } => Some((STORAGE_WINDOW_ID, win)),
        State::Trade { win, .. } => Some((TRADE_WINDOW_ID, win)),
        State::Mailbox { win, .. } => Some((MAILBOX_WINDOW_ID, win)),
        State::ReadMail { win, .. } => Some((READ_MAIL_WINDOW_ID, win)),
        State::CartSelect { win, .. } => Some((CART_SELECT_WINDOW_ID, win)),
        State::StoragePassword { win, .. } => Some((STORAGE_PASSWORD_WINDOW_ID, win)),
        State::VendingSetup { win, .. } => Some((VENDING_SETUP_WINDOW_ID, win)),
        State::MyShop { win, .. } => Some((MY_SHOP_WINDOW_ID, win)),
        State::VendingBuy { win, .. } => Some((VENDING_SHOP_WINDOW_ID, win)),
        State::Mercenary { win, .. } => Some((MERCENARY_WINDOW_ID, win)),
        State::MercenarySkill { win, .. } => Some((MERCENARY_SKILL_WINDOW_ID, win)),
        State::Homun { win, .. } => Some((HOMUN_WINDOW_ID, win)),
        State::Pet { win, .. } => Some((PET_WINDOW_ID, win)),
        State::CompanionAiConfig { win, .. } => Some((COMPANION_AI_CONFIG_WINDOW_ID, win)),
        State::NpcDialog { npc, .. } => Some((NPC_DIALOG_WINDOW_ID, npc)),
        State::NpcShop { shop, .. } => return shop.gallery_windows(),
        _ => None,
    };
    single
        .map(|(id, win)| (id, win.window_size()))
        .into_iter()
        .collect()
}

/// Centered modal dialogs a component owns, with their nominal size. The tool
/// seeds these into the bottom-right corner (staggered) so they don't pile up
/// over the packed windows; each still centers itself in-game.
fn modal_windows(state: &State) -> Vec<(WidgetId, (f32, f32))> {
    match state {
        State::NumberInput { dialog } => vec![(dialog.win_id(), dialog.window_size())],
        State::CardInsert { dialog, .. } => vec![(CARD_INSERT_WINDOW_ID, dialog.window_size())],
        _ => Vec::new(),
    }
}

/// A few rows of the real position table, enough to see the current-map
/// highlight and off-map party markers side by side.
const DEMO_MAP_POSITIONS: &str = concat!(
    "8#prontera.rsw#812#587#870#643#\n",
    "8#izlude.rsw#871#644#902#676#\n",
    "6#geffen.rsw#576#528#635#586#\n",
    "11#payon.rsw#967#746#1013#803#\n",
);
const DEMO_MAP_NAMES: &str = concat!(
    "prontera.rsw#Prontera, Capital of Rune Midgard#\n",
    "izlude.rsw#Izlude, the Satellite City#\n",
    "geffen.rsw#Geffen, the Magic City#\n",
    "payon.rsw#Payon Town#\n",
);

const GALLERY_GAP: f32 = 12.0;
const GALLERY_MARGIN: f32 = 8.0;

type FreeRect = (f32, f32, f32, f32);

fn rects_intersect(a: FreeRect, b: FreeRect) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

/// True when `outer` fully contains `inner`.
fn rect_contains(outer: FreeRect, inner: FreeRect) -> bool {
    inner.0 >= outer.0
        && inner.1 >= outer.1
        && inner.0 + inner.2 <= outer.0 + outer.2
        && inner.1 + inner.3 <= outer.1 + outer.3
}

/// Subtract `used` from free rectangle `f`, yielding the maximal sub-rectangles
/// that remain. When they do not intersect, `f` is returned unchanged.
fn split_free(f: FreeRect, used: FreeRect) -> Vec<FreeRect> {
    if !rects_intersect(f, used) {
        return vec![f];
    }
    let (fx, fy, fw, fh) = f;
    let (ux, uy, uw, uh) = used;
    let mut out = Vec::new();
    if ux > fx {
        out.push((fx, fy, ux - fx, fh));
    }
    if ux + uw < fx + fw {
        out.push((ux + uw, fy, fx + fw - (ux + uw), fh));
    }
    if uy > fy {
        out.push((fx, fy, fw, uy - fy));
    }
    if uy + uh < fy + fh {
        out.push((fx, uy + uh, fw, fy + fh - (uy + uh)));
    }
    out
}

fn prune_contained(free: &mut Vec<FreeRect>) {
    let mut i = 0;
    while i < free.len() {
        let mut j = i + 1;
        let mut removed = false;
        while j < free.len() {
            if rect_contains(free[j], free[i]) {
                free.swap_remove(i);
                removed = true;
                break;
            }
            if rect_contains(free[i], free[j]) {
                free.swap_remove(j);
            } else {
                j += 1;
            }
        }
        if !removed {
            i += 1;
        }
    }
}

/// MaxRects bin packing (best-short-side-fit). The largest windows are placed
/// first; each goes into the free rectangle that leaves the smallest leftover
/// edge, then every free rectangle overlapping it is split into its maximal
/// remaining pieces. Placed windows never overlap as long as they fit on
/// screen — no fixed columns, no windows clamped on top of each other. Returns
/// each window's `(id, x, y)`.
fn pack_gallery(
    mut items: Vec<(WidgetId, f32, f32)>,
    avail_w: f32,
    avail_h: f32,
    reserved: &[FreeRect],
) -> Vec<(WidgetId, f32, f32)> {
    items.sort_by(|a, b| (b.1 * b.2).total_cmp(&(a.1 * a.2)));

    let mut free: Vec<FreeRect> = vec![(
        GALLERY_MARGIN,
        GALLERY_MARGIN + 20.0,
        (avail_w - GALLERY_MARGIN).max(1.0),
        (avail_h - GALLERY_MARGIN).max(1.0),
    )];
    // Carve out the regions occupied by self-positioning windows first, so
    // packed windows are routed around them.
    for &zone in reserved {
        free = free.drain(..).flat_map(|f| split_free(f, zone)).collect();
        prune_contained(&mut free);
    }
    let mut placed = Vec::with_capacity(items.len());

    for (id, w, h) in items {
        let (rw, rh) = (w + GALLERY_GAP, h + GALLERY_GAP);

        let mut best: Option<(f32, f32, f32, f32)> = None; // (short_leftover, long_leftover, x, y)
        for &(fx, fy, fw, fh) in &free {
            if rw <= fw && rh <= fh {
                let (short, long) = {
                    let (a, b) = (fw - rw, fh - rh);
                    (a.min(b), a.max(b))
                };
                if best.is_none_or(|(bs, bl, ..)| short < bs || (short == bs && long < bl)) {
                    best = Some((short, long, fx, fy));
                }
            }
        }

        let (bx, by) = match best {
            Some((_, _, x, y)) => (x, y),
            None => free
                .iter()
                .max_by(|a, b| (a.2 * a.3).total_cmp(&(b.2 * b.3)))
                .map(|&(x, y, ..)| (x, y))
                .unwrap_or((GALLERY_MARGIN, GALLERY_MARGIN)),
        };
        // `window_at` clamps every window to [0, screen − size] at render time.
        // Keep the packer's own placement inside that range so a window that
        // didn't fit is never seeded off-screen and then dragged back on top of
        // an already-placed one.
        let bx = bx.min((avail_w - w).max(0.0));
        let by = by.min((avail_h - h).max(0.0));
        placed.push((id, bx, by));

        let used = (bx, by, rw, rh);
        free = free.drain(..).flat_map(|f| split_free(f, used)).collect();
        prune_contained(&mut free);
    }
    placed
}

/// Regions occupied by the self-positioning windows the packer can't move:
/// the top-centre hotkey bar and the bottom-left chat box. Packed windows are
/// routed around these. The centred modal dialogs are drawn on top and are not
/// reserved, so they never fragment the free space out from under large windows.
fn fixed_ui_zones(sw: f32, sh: f32) -> Vec<FreeRect> {
    let hotkey = (
        (sw * 0.25).max(0.0),
        0.0,
        (sw * 0.5).min(sw),
        92.0f32.min(sh),
    );
    let chat = (
        0.0,
        (sh - 200.0).max(0.0),
        360.0f32.min(sw),
        200.0f32.min(sh),
    );
    vec![hotkey, chat]
}

fn gallery_placements(components: &[State], ui: &UiFrame) -> Vec<(WidgetId, f32, f32)> {
    let items: Vec<(WidgetId, f32, f32)> = components
        .iter()
        .flat_map(|comp| {
            gallery_windows(comp)
                .into_iter()
                .map(|(id, (w, h))| (id, w, h))
        })
        .collect();
    if items.is_empty() {
        return Vec::new();
    }
    let (sw, sh) = (ui.ctx.screen_width.max(1.0), ui.ctx.screen_height.max(1.0));
    let placed = pack_gallery(items.clone(), sw, sh, &fixed_ui_zones(sw, sh));
    log_gallery_layout(sw, sh, &items, &placed);
    placed
}

/// Print the packer input/output to stderr once per screen-size change, so a
/// reported overlap can be reproduced with the exact logical dimensions used.
fn log_gallery_layout(
    sw: f32,
    sh: f32,
    items: &[(WidgetId, f32, f32)],
    placed: &[(WidgetId, f32, f32)],
) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST: AtomicU64 = AtomicU64::new(0);
    let key = ((sw as u64) << 32) | (sh as u64);
    if LAST.swap(key, Ordering::Relaxed) == key {
        return;
    }
    eprintln!("[gallery] logical screen {sw}x{sh}");
    for &(id, x, y) in placed {
        let (w, h) = items
            .iter()
            .find(|it| it.0 == id)
            .map(|it| (it.1, it.2))
            .unwrap_or((0.0, 0.0));
        eprintln!("  window {} at ({x:.0}, {y:.0}) size {w:.0}x{h:.0}", id.0);
    }
}

/// Stack the centered modal dialogs into the bottom-right corner, each shifted
/// diagonally so no two share a position and every one keeps a distinct x.
fn modal_placements(components: &[State], sw: f32, sh: f32) -> Vec<(WidgetId, f32, f32)> {
    const STAGGER: f32 = 34.0;
    let mut out = Vec::new();
    for (i, (id, (w, h))) in components.iter().flat_map(modal_windows).enumerate() {
        let shift = i as f32 * STAGGER;
        let x = (sw - w - GALLERY_MARGIN - shift).max(GALLERY_MARGIN);
        let y = (sh - h - GALLERY_MARGIN - shift).max(GALLERY_MARGIN);
        out.push((id, x, y));
    }
    out
}

fn seed_gallery_layout(components: &[State], ui: &mut UiFrame) {
    for (id, x, y) in gallery_placements(components, ui) {
        ui.seed_window_position(id, x, y);
    }
    let (sw, sh) = (ui.ctx.screen_width.max(1.0), ui.ctx.screen_height.max(1.0));
    for (id, x, y) in modal_placements(components, sw, sh) {
        ui.seed_window_position(id, x, y);
    }
}

fn repack_gallery_layout(components: &[State], ui: &mut UiFrame) {
    for (id, x, y) in gallery_placements(components, ui) {
        ui.set_window_position(id, x, y);
    }
    let (sw, sh) = (ui.ctx.screen_width.max(1.0), ui.ctx.screen_height.max(1.0));
    for (id, x, y) in modal_placements(components, sw, sh) {
        ui.set_window_position(id, x, y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_windows_report_packable_size() {
        // The packer lays out windows by their `window_size`; a (0,0) here means
        // the window collapses into a corner and renders on top of the others.
        let sizes = [
            LoginWindow::new().window_size(),
            ServerListWindow::new(Vec::new()).window_size(),
            CharSelectWindow::new(Vec::new()).window_size(),
            CharCreateWindow::new(0, true).window_size(),
        ];
        for (w, h) in sizes {
            assert!(w > 0.0 && h > 0.0, "account window size was {w}x{h}");
        }
    }

    #[test]
    fn packed_windows_never_overlap_and_stay_on_screen() {
        let sizes = [
            (WidgetId(1), 555.0, 455.0),
            (WidgetId(2), 280.0, 130.0),
            (WidgetId(3), 240.0, 178.0),
            (WidgetId(4), 384.0, 340.0),
            (WidgetId(5), 170.0, 240.0),
            (WidgetId(6), 288.0, 224.0),
            (WidgetId(7), 270.0, 195.0),
        ];
        let (screen_w, screen_h) = (1280.0, 900.0);
        let placed = pack_gallery(sizes.to_vec(), screen_w, screen_h, &[]);

        let rect = |id: WidgetId| {
            let (_, w, h) = sizes.iter().find(|s| s.0 == id).copied().unwrap();
            let (_, x, y) = placed.iter().find(|p| p.0 == id).copied().unwrap();
            (x, y, w, h)
        };
        for i in 0..sizes.len() {
            let (ax, ay, aw, ah) = rect(sizes[i].0);
            assert!(ax >= 0.0 && ay >= 0.0);
            assert!(ax + aw <= screen_w && ay + ah <= screen_h);
            for j in (i + 1)..sizes.len() {
                let (bx, by, bw, bh) = rect(sizes[j].0);
                let disjoint = ax + aw <= bx || bx + bw <= ax || ay + ah <= by || by + bh <= ay;
                assert!(disjoint, "windows {i} and {j} overlap");
            }
        }
    }
}

/// Bottom-right anchor for a head board, stacked upward by `index`. Head boards
/// draw centred above the anchor, so this keeps them clear of the packed windows
/// in the top-left of a category view.
fn board_anchor(ui: &UiFrame, index: usize) -> (f32, f32) {
    const STEP: f32 = 50.0;
    let x = ui.ctx.screen_width - GALLERY_MARGIN - BOARD_W / 2.0;
    let y = ui.ctx.screen_height - GALLERY_MARGIN - index as f32 * STEP;
    (x, y)
}

struct HotCtxDefaults {
    friends: FriendList,
    quest_log: QuestLog,
    pet: PetState,
    companion_ai: CompanionAiConfig,
}

impl HotCtxDefaults {
    fn new() -> Self {
        Self {
            friends: FriendList::default(),
            quest_log: QuestLog::default(),
            pet: PetState::default(),
            companion_ai: CompanionAiConfig::default(),
        }
    }

    fn ctx<'a>(&'a mut self, character: &'a mut Character, data: &'a DataTable) -> BuildCtx<'a> {
        BuildCtx {
            character,
            data,
            party: None,
            friends: &self.friends,
            guild: None,
            quest_log: &self.quest_log,
            homunculus: None,
            mercenary: None,
            pet: &self.pet,
            companion_ai: &mut self.companion_ai,
            job_class: JobName::Swordsman.value() as u16,
            local_aid: 0,
            local_gid: 0,
        }
    }
}

fn build_single(state: &mut State, ui: &mut UiFrame) {
    let mut d = HotCtxDefaults::new();
    match state {
        State::Inventory {
            inv,
            character,
            data,
        } => {
            inv.build(ui, &mut d.ctx(character, data));
        }
        State::Cart {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::Storage {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::Trade {
            win,
            character,
            data,
        } => {
            use ragnarok_game::trade::{CONCLUDE_ME, CONCLUDE_OTHER, TRADE_ZENY_INDEX};
            // No trade server here, so stand in for the add/conclude acks that
            // normally reflect our side and lock the panes.
            let trade_events = win.build(ui, &mut d.ctx(character, data));
            for event in trade_events {
                match event {
                    GameEvent::RequestAddExchangeItem { .. } => {
                        if let Some((idx, cnt)) = character.trade.take_pending_add() {
                            if idx == TRADE_ZENY_INDEX {
                                character.trade.add_my_zeny(cnt as i64);
                                character.inventory.zeny = (character.inventory.zeny - cnt).max(0);
                            } else if let Some(src) = character.inventory.get_item(idx) {
                                let mut item = src.clone();
                                item.count = cnt as i16;
                                character.trade.add_my_item(item);
                            }
                        }
                    }
                    GameEvent::RequestConcludeExchange => {
                        character.trade.lock(CONCLUDE_ME);
                        character.trade.lock(CONCLUDE_OTHER);
                    }
                    _ => {}
                }
            }
        }
        State::Mailbox {
            win,
            character,
            data,
        } => {
            // No mail-server here, so stand in for the attach/send acks that
            // normally drive compose state.
            let mail_events = win.build(ui, &mut d.ctx(character, data));
            for event in mail_events {
                match event {
                    GameEvent::RequestMailAddItem { .. } => {
                        if let Some(pending) = character.mail.compose.pending_item.take() {
                            character.mail.compose.item = Some(pending);
                        }
                    }
                    GameEvent::RequestMailSend { .. } => {
                        character.mail.send_pending = false;
                        character.mail.switch_to_inbox();
                    }
                    _ => {}
                }
            }
        }
        State::ReadMail {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::CartSelect {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::StoragePassword {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::VendingSetup {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
            win.build_available(ui, &mut d.ctx(character, data));
        }
        State::MyShop {
            win,
            character,
            data,
            ..
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::VendingBuy {
            win,
            character,
            data,
            ..
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::VendingBoard { container, name } => {
            let (ax, ay) = board_anchor(ui, 0);
            vending_board::draw_board(&mut ui.draw_calls, container, ui.atlas, ax, ay, 40.0, name);
        }
        State::ChatRoomBoard {
            container,
            atype,
            title,
            cur,
            max,
        } => {
            let (ax, ay) = board_anchor(ui, 1);
            chat_room_board::draw_board(
                &mut ui.draw_calls,
                container,
                ui.atlas,
                ax,
                ay,
                40.0,
                *atype,
                title,
                *cur,
                *max,
            );
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
            shop.build(ui, &mut d.ctx(character, data));
        }
        State::Login { login } => {
            login.build(ui);
        }
        State::Chat {
            chat,
            character,
            data,
        } => {
            chat.build(ui, &mut d.ctx(character, data));
        }
        State::NpcDialog {
            npc,
            character,
            data,
        } => {
            npc.build(ui, &mut d.ctx(character, data));
        }
        State::ConfirmDialog { dialog, open } => {
            if *open && dialog.state.is_some() {
                dialog.build(ui);
                if dialog.state.is_none() {
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
            equip.build(ui, &mut d.ctx(character, data));
        }
        State::SystemMenu {
            menu,
            character,
            data,
        } => {
            add_button(ui, "Open system Dialog", WidgetId(599), 10.0, 10.0, |_ui| {
                menu.open = true;
            });
            menu.build(ui, &mut d.ctx(character, data));
        }
        State::CharSelect { win } => {
            // No char-server here, so stand in for the reserve/confirm acks to make
            // the delete dialog exercisable in the viewer.
            for event in win.build(ui) {
                match event {
                    GameEvent::RequestDeleteCharacterReserve { gid } => {
                        win.open_delete_dialog(gid, 0);
                    }
                    GameEvent::RequestDeleteCharacterConfirm { gid, .. } => {
                        win.remove_character(gid);
                        win.close_delete_dialog();
                    }
                    GameEvent::RequestDeleteCharacterCancel { .. } => {
                        win.close_delete_dialog();
                    }
                    _ => {}
                }
            }
        }
        State::CharCreate { win } => {
            win.build(ui);
        }
        State::ItemInfo {
            win,
            character,
            data,
            ..
        } => {
            let events = win.build(ui, &mut d.ctx(character, data));
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
            win.build(ui, &mut d.ctx(character, data));
        }
        State::Book {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::MonsterInfo {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::ChatRoomCreate {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::Emotion {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::ShortcutList {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::GraphicOptions {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::HotkeyConfig {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::Quest {
            win,
            log,
            character,
            data,
        } => {
            let mut ctx = d.ctx(character, data);
            ctx.quest_log = &*log;
            win.build(ui, &mut ctx);
        }
        State::QuestDetail {
            win,
            log,
            character,
            data,
        } => {
            let mut ctx = d.ctx(character, data);
            ctx.quest_log = &*log;
            win.build(ui, &mut ctx);
        }
        State::Minimap {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::WorldMap {
            win,
            party,
            local_aid,
            character,
            data,
        } => {
            let mut ctx = d.ctx(character, data);
            ctx.party = Some(&*party);
            ctx.local_aid = *local_aid;
            win.build(ui, &mut ctx);
        }
        State::ChatRoomMember {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::CardInsert {
            dialog,
            character,
            data,
        } => {
            dialog.build(ui, &mut d.ctx(character, data));
        }
        State::DialogContainerDemo { notification } => {
            let mut character = Character::new();
            notification.build(ui, &mut d.ctx(&mut character, &DataTable::default()));
        }
        State::HotkeyBarDemo {
            hotkey_win,
            character,
            data,
        } => {
            hotkey_win.build(ui, &mut d.ctx(character, data));
        }
        State::BasicInfoDemo {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::StatusDemo {
            win,
            character,
            data,
        } => {
            win.build(ui, &mut d.ctx(character, data));
        }
        State::PartyDemo {
            win,
            party,
            local_aid,
            character,
            data,
        } => {
            let mut ctx = d.ctx(character, data);
            ctx.party = Some(&*party);
            ctx.local_aid = *local_aid;
            win.build(ui, &mut ctx);
        }
        State::GuildDemo {
            win,
            guild,
            local_gid,
            character,
            data,
        } => {
            character.name = "Walkiry".to_string();
            let mut ctx = d.ctx(character, data);
            ctx.guild = Some(&*guild);
            ctx.local_gid = *local_gid;
            win.build(ui, &mut ctx);
        }
        State::Mercenary { win, merc } => {
            let mut character = Character::new();
            let data = DataTable::new();
            let mut ctx = d.ctx(&mut character, &data);
            ctx.mercenary = Some(&*merc);
            win.build(ui, &mut ctx);
        }
        State::MercenarySkill { win, merc } => {
            let mut character = Character::new();
            let data = DataTable::new();
            let mut ctx = d.ctx(&mut character, &data);
            ctx.mercenary = Some(&*merc);
            win.build(ui, &mut ctx);
        }
        State::Homun { win, homun } => {
            let mut character = Character::new();
            let data = DataTable::new();
            let mut ctx = d.ctx(&mut character, &data);
            ctx.homunculus = Some(&*homun);
            win.build(ui, &mut ctx);
        }
        State::Pet { win, pet } => {
            let mut character = Character::new();
            let data = DataTable::new();
            let mut ctx = d.ctx(&mut character, &data);
            ctx.pet = &*pet;
            win.build(ui, &mut ctx);
        }
        State::CompanionAiConfig { win, config } => {
            let mut character = Character::new();
            let data = DataTable::new();
            let mut ctx = d.ctx(&mut character, &data);
            ctx.companion_ai = &mut *config;
            win.build(ui, &mut ctx);
        }
        State::FallbackGallery { name_field } => {
            build_fallback_gallery(ui, name_field);
        }
        State::Category { components } => {
            seed_gallery_layout(components, ui);
            // The re-pack button lives in the top-centre band the packer keeps
            // clear (the hotkey zone), so no window ever covers it.
            let mut do_repack = false;
            let btn_x = (ui.ctx.screen_width * 0.5 - 80.0).max(GALLERY_MARGIN);
            add_button(ui, "Re-pack layout", WidgetId(60100), btn_x, 4.0, |_| {
                do_repack = true;
            });
            if do_repack {
                repack_gallery_layout(components, ui);
            }
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

fn build_fallback_gallery(ui: &mut UiFrame, name_field: &mut TextInput) {
    use ragnarok_ui::theme::{self, FallbackPalette as P};
    use ragnarok_ui_component::helper::fallback;

    // The gallery exists to iterate the no-GRF look, so force the fallback path.
    ui.has_grf_textures = false;

    let heading = [0.72, 0.78, 0.92, 1.0];
    ui.text(24.0, 22.0, "Fallback components — edit lib/ui-core/src/theme.rs & lib/ui-component/src/helper/fallback.rs", heading);

    // Window chrome
    let (wx, wy, ww) = (24.0, 56.0, 210.0);
    ui.text(wx, wy - 8.0, "WINDOW CHROME", heading);
    fallback::titlebar(ui, wx, wy, ww, 18.0);
    ui.text(wx + 20.0, wy + 13.0, "Inventory", P::TEXT_ON_LIGHT);
    fallback::container(ui, wx, wy + 18.0, ww, 56.0);
    ui.text(wx + 10.0, wy + 40.0, "10,240 z", P::TEXT_ON_LIGHT);
    fallback::footer(ui, wx, wy + 74.0, ww, 22.0);
    theme::fallback_button(
        ui,
        Rect::new(wx + ww - 54.0, wy + 78.0, 46.0, 16.0),
        false,
        false,
        "Close",
    );

    // Buttons
    let bx = 280.0;
    ui.text(bx, 48.0, "BUTTONS  normal / hover / pressed", heading);
    theme::fallback_button(ui, Rect::new(bx, 56.0, 60.0, 21.0), false, false, "OK");
    theme::fallback_button(
        ui,
        Rect::new(bx + 70.0, 56.0, 60.0, 21.0),
        true,
        false,
        "OK",
    );
    theme::fallback_button(
        ui,
        Rect::new(bx + 140.0, 56.0, 60.0, 21.0),
        false,
        true,
        "OK",
    );
    ui.text(bx, 96.0, "live:", heading);
    let btn = ButtonTextures {
        normal: "",
        hover: "",
        pressed: "",
    };
    ui.button(
        WidgetId(60001),
        Rect::new(bx + 34.0, 88.0, 90.0, 21.0),
        &btn,
        "Hover / click",
    );

    // Text field
    ui.text(bx, 132.0, "TEXT FIELD  (click to focus)", heading);
    ui.text_input(
        WidgetId(60002),
        Rect::new(bx, 140.0, 150.0, 20.0),
        name_field,
        TextInputBg::Default,
    );

    // Slot cells + system buttons
    ui.text(wx, 176.0, "SLOT CELLS / SYS BUTTONS", heading);
    for i in 0..4 {
        fallback::slot_cell(ui, wx + i as f32 * 34.0, 188.0, 30.0, 30.0);
    }
    fallback::sys_button(ui, wx + 150.0, 188.0, 11.0, false, Some('_'));
    fallback::sys_button(ui, wx + 166.0, 188.0, 11.0, true, Some('x'));

    // Gauges
    ui.text(bx, 176.0, "GAUGES  (HP / SP)", heading);
    fallback::gauge(ui, bx, 190.0, 150.0, 9.0, 0.72, true);
    fallback::gauge(ui, bx, 204.0, 150.0, 9.0, 0.54, false);

    // Standalone panel + cells (context menu / stat cells / tabs)
    ui.text(wx, 240.0, "PANEL / CELLS  (active | inactive)", heading);
    fallback::panel(ui, wx, 252.0, 120.0, 46.0);
    ui.text(wx + 8.0, 272.0, "context menu", P::TEXT_ON_LIGHT);
    fallback::cell(ui, wx + 132.0, 252.0, 70.0, 20.0, true);
    ui.text(wx + 140.0, 266.0, "Use", P::TEXT_ON_LIGHT);
    fallback::cell(ui, wx + 132.0, 276.0, 70.0, 20.0, false);
    ui.text(wx + 140.0, 290.0, "Etc", P::TEXT_ON_LIGHT);
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

fn vendor_item(index: i16, item_id: u16, amount: i16, price: i32) -> VendorItem {
    VendorItem {
        index,
        item_id,
        amount,
        price,
        refine: 0,
        is_identified: true,
        is_damaged: false,
        item_type: 0,
    }
}

fn vending_test_stock() -> Vec<(VendorItem, String)> {
    vec![
        (vendor_item(0, 501, 20, 120), "Red Potion".into()),
        (vendor_item(1, 502, 8, 350), "Orange Potion".into()),
        (vendor_item(2, 610, 5, 4500), "Yggdrasil Leaf".into()),
        (vendor_item(3, 1101, 1, 25000), "+7 Sword".into()),
        (vendor_item(4, 1201, 1, 8000), "Stiletto".into()),
        (vendor_item(5, 4058, 1, 8000000), "Stiletto".into()),
    ]
}

fn item_resource_from_ids(
    ids: impl IntoIterator<Item = u16>,
    table: &ItemResourceTable,
) -> ItemResourceTable {
    let mut entries = HashMap::new();
    for id in ids {
        if let Some(name) = table.get_resource_name(id) {
            entries.insert(id, name.to_string());
        }
    }
    ItemResourceTable::from_entries(entries, HashMap::new())
}

fn resolve_stock_icons(
    src: &[(VendorItem, String)],
    table: &ItemResourceTable,
) -> Vec<(VendorItem, String, Option<String>)> {
    src.iter()
        .map(|(item, name)| {
            let icon = table
                .get_resource_name_for(item.item_id, item.is_identified)
                .map(|res| ragnarok_resources::ui::item::icon(res));
            (item.clone(), name.clone(), icon)
        })
        .collect()
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

fn storage_test_items() -> Vec<Item> {
    let mut items = vec![
        make_test_item(1, 501, 0, 250, "Red Potion"),
        make_test_item(2, 502, 0, 999, "Orange Potion"),
        make_test_item(3, 601, 0, 30, "Fly Wing"),
        make_test_item(4, 1201, 5, 1, "Stiletto"),
        make_test_item(5, 1101, 5, 1, "Sword"),
        make_test_item(6, 2301, 4, 1, "Chain Mail"),
        make_test_item(7, 2101, 4, 1, "Guard"),
        make_test_item(8, 1750, 10, 500, "Arrow"),
        make_test_item(9, 4001, 6, 3, "Poring Card"),
        make_test_item(10, 4035, 6, 1, "Marc Card"),
        make_test_item(11, 910, 3, 250, "Jellopy"),
        make_test_item(12, 911, 3, 120, "Shell"),
    ];
    items[3].refining_level = 7;
    items[4].refining_level = 4;
    items[5].is_damaged = true;
    items[6].is_identified = false;
    items[6].name = "Unknown Guard".into();
    items
}

fn mail_test_inbox() -> Vec<ragnarok_game::mail::MailEntry> {
    use ragnarok_game::mail::MailEntry;
    let base = 1_615_680_000u32;
    (0..9)
        .map(|i| MailEntry {
            mail_id: 1000 + i as u32,
            title: format!("Message subject number {i}"),
            read: i % 3 == 0,
            sender: format!("Sender{i}"),
            time: base + i as u32 * 86_400,
        })
        .collect()
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
