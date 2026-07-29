use ragnarok_ui::frame::WidgetId;
use ragnarok_ui_component::InGameWindow;
use ragnarok_ui_component::game::basic_info_window::BASIC_INFO_WINDOW_ID;
use ragnarok_ui_component::game::basic_info_window::BasicInfoWindow;
use ragnarok_ui_component::game::book_window::BOOK_WINDOW_ID;
use ragnarok_ui_component::game::book_window::BookWindow;
use ragnarok_ui_component::game::card_insert_dialog::CardInsertDialog;
use ragnarok_ui_component::game::cart_select_window::CART_SELECT_WINDOW_ID;
use ragnarok_ui_component::game::cart_select_window::CartSelectWindow;
use ragnarok_ui_component::game::cart_window::CART_WINDOW_ID;
use ragnarok_ui_component::game::cart_window::CartWindow;
use ragnarok_ui_component::game::chat_room_create_window::CHAT_ROOM_CREATE_WINDOW_ID;
use ragnarok_ui_component::game::chat_room_create_window::ChatRoomCreateWindow;
use ragnarok_ui_component::game::chat_room_member_window::CHAT_ROOM_MEMBER_WINDOW_ID;
use ragnarok_ui_component::game::chat_room_member_window::ChatRoomMemberWindow;
use ragnarok_ui_component::game::chat_window::CHAT_WINDOW_ID;
use ragnarok_ui_component::game::chat_window::ChatWindow;
use ragnarok_ui_component::game::companion_ai_config_window::COMPANION_AI_CONFIG_WINDOW_ID;
use ragnarok_ui_component::game::companion_ai_config_window::CompanionAiConfigWindow;
use ragnarok_ui_component::game::confirm_dialog::ConfirmDialog;
use ragnarok_ui_component::game::context_menu::ContextMenu;
use ragnarok_ui_component::game::drop_quantity_dialog::DropQuantityDialog;
use ragnarok_ui_component::game::emblem_picker_window::EMBLEM_PICKER_WINDOW_ID;
use ragnarok_ui_component::game::emblem_picker_window::EmblemPickerWindow;
use ragnarok_ui_component::game::emotion_window::EMOTION_WINDOW_ID;
use ragnarok_ui_component::game::emotion_window::EmotionWindow;
use ragnarok_ui_component::game::equipment_window::EQ_WINDOW_ID;
use ragnarok_ui_component::game::equipment_window::EquipmentWindow;
use ragnarok_ui_component::game::graphic_options::GRAPHIC_OPTIONS_WINDOW_ID;
use ragnarok_ui_component::game::graphic_options::GraphicOptionsWindow;
use ragnarok_ui_component::game::guild_expel_dialog::GuildExpelDialog;
use ragnarok_ui_component::game::guild_window::GUILD_WINDOW_ID;
use ragnarok_ui_component::game::guild_window::GuildWindow;
use ragnarok_ui_component::game::homun_skill_window::HOMUN_SKILL_WINDOW_ID;
use ragnarok_ui_component::game::homun_skill_window::HomunSkillWindow;
use ragnarok_ui_component::game::homun_window::HOMUN_WINDOW_ID;
use ragnarok_ui_component::game::homun_window::HomunWindow;
use ragnarok_ui_component::game::hotkey_bar::HotkeyBarWindow;
use ragnarok_ui_component::game::hotkey_config_window::HOTKEY_CONFIG_WINDOW_ID;
use ragnarok_ui_component::game::hotkey_config_window::HotkeyConfigWindow;
use ragnarok_ui_component::game::inventory_window::INV_WINDOW_ID;
use ragnarok_ui_component::game::inventory_window::InventoryWindow;
use ragnarok_ui_component::game::item_info_window::ItemInfoWindow;
use ragnarok_ui_component::game::item_list_selection_window::ItemListSelectionWindow;
use ragnarok_ui_component::game::item_pickup_notification::ItemPickupNotification;
use ragnarok_ui_component::game::levelup_notification_window::LevelUpNotificationWindow;
use ragnarok_ui_component::game::mailbox_window::MAILBOX_WINDOW_ID;
use ragnarok_ui_component::game::mailbox_window::MailboxWindow;
use ragnarok_ui_component::game::make_item_window::MAKE_ITEM_WINDOW_ID;
use ragnarok_ui_component::game::make_item_window::MakeItemWindow;
use ragnarok_ui_component::game::map_missing_window::MapMissingWindow;
use ragnarok_ui_component::game::mercenary_skill_window::MERCENARY_SKILL_WINDOW_ID;
use ragnarok_ui_component::game::mercenary_skill_window::MercenarySkillWindow;
use ragnarok_ui_component::game::mercenary_window::MERCENARY_WINDOW_ID;
use ragnarok_ui_component::game::mercenary_window::MercenaryWindow;
use ragnarok_ui_component::game::minimap_window::MinimapWindow;
use ragnarok_ui_component::game::my_shop_window::MY_SHOP_WINDOW_ID;
use ragnarok_ui_component::game::my_shop_window::MyShopWindow;
use ragnarok_ui_component::game::npc_dialog::NpcDialog;
use ragnarok_ui_component::game::npc_shop::NpcShop;
use ragnarok_ui_component::game::party_friends_window::PARTY_FRIENDS_WINDOW_ID;
use ragnarok_ui_component::game::party_friends_window::PartyFriendsWindow;
use ragnarok_ui_component::game::party_helper_window::PARTY_HELPER_WINDOW_ID;
use ragnarok_ui_component::game::party_helper_window::PartyHelperWindow;
use ragnarok_ui_component::game::pet_window::PET_WINDOW_ID;
use ragnarok_ui_component::game::pet_window::PetWindow;
use ragnarok_ui_component::game::quest_window::{QUEST_DETAIL_WINDOW_ID, QUEST_WINDOW_ID};
use ragnarok_ui_component::game::quest_window::{QuestDetailWindow, QuestWindow};
use ragnarok_ui_component::game::read_mail_window::READ_MAIL_WINDOW_ID;
use ragnarok_ui_component::game::read_mail_window::ReadMailWindow;
use ragnarok_ui_component::game::shortcut_list_window::SHORTCUT_LIST_WINDOW_ID;
use ragnarok_ui_component::game::shortcut_list_window::ShortcutListWindow;
use ragnarok_ui_component::game::skill_talkbox_dialog::SkillTalkboxDialog;
use ragnarok_ui_component::game::skill_tree_window::SKILL_WINDOW_ID;
use ragnarok_ui_component::game::skill_tree_window::SkillTreeWindow;
use ragnarok_ui_component::game::sound_options::SOUND_OPTIONS_WINDOW_ID;
use ragnarok_ui_component::game::sound_options::SoundOptionsWindow;
use ragnarok_ui_component::game::status_icon_bar::StatusIconBarWindow;
use ragnarok_ui_component::game::status_window::STATUS_WINDOW_ID;
use ragnarok_ui_component::game::status_window::StatusWindow;
use ragnarok_ui_component::game::storage_window::STORAGE_WINDOW_ID;
use ragnarok_ui_component::game::storage_window::StorageWindow;
use ragnarok_ui_component::game::system_menu::SystemMenu;
use ragnarok_ui_component::game::trade_window::TRADE_WINDOW_ID;
use ragnarok_ui_component::game::trade_window::TradeWindow;
use ragnarok_ui_component::game::vending_setup_window::VendingSetupWindow;
use ragnarok_ui_component::game::vending_setup_window::{
    VENDING_AVAILABLE_WINDOW_ID, VENDING_SETUP_WINDOW_ID,
};
use ragnarok_ui_component::game::vending_shop_window::VENDING_SHOP_WINDOW_ID;
use ragnarok_ui_component::game::vending_shop_window::VendingShopWindow;
use ragnarok_ui_component::game::warp_list_window::WarpListWindow;
use ragnarok_ui_component::game::world_map_window::{WORLD_MAP_WINDOW_ID, WorldMapWindow};

pub struct Windows {
    pub chat_window: ChatWindow,
    pub equipment_window: EquipmentWindow,
    pub inventory_window: InventoryWindow,
    pub cart_window: CartWindow,
    pub storage_window: StorageWindow,
    pub trade_window: TradeWindow,
    pub mailbox_window: MailboxWindow,
    pub read_mail_window: ReadMailWindow,
    pub cart_select_window: CartSelectWindow,
    pub npc_dialog: NpcDialog,
    pub warp_list_window: WarpListWindow,
    pub item_list_selection_window: ItemListSelectionWindow,
    pub make_item_window: MakeItemWindow,
    pub vending_shop_window: VendingShopWindow,
    pub vending_setup_window: VendingSetupWindow,
    pub my_shop_window: MyShopWindow,
    pub confirm_dialog: ConfirmDialog,
    pub npc_shop: NpcShop,
    pub chat_room_create_window: ChatRoomCreateWindow,
    pub chat_room_member_window: ChatRoomMemberWindow,
    pub emotion_window: EmotionWindow,
    pub shortcut_list_window: ShortcutListWindow,
    pub quest_window: QuestWindow,
    pub quest_detail_window: QuestDetailWindow,
    pub system_menu: SystemMenu,
    pub map_missing_window: MapMissingWindow,
    pub drop_dialog_has_grf_textures: bool,
    pub drop_quantity_dialog: Option<DropQuantityDialog>,
    pub guild_expel_dialog: Option<GuildExpelDialog>,
    pub skill_talkbox_dialog: Option<SkillTalkboxDialog>,
    pub card_insert_dialog: Option<CardInsertDialog>,
    pub card_insert_dialog_has_grf_textures: bool,
    pub item_info_window: ItemInfoWindow,
    pub book_window: BookWindow,
    pub sound_options: SoundOptionsWindow,
    pub graphic_options: GraphicOptionsWindow,
    pub hotkey_config_window: HotkeyConfigWindow,
    pub item_pickup_notification: ItemPickupNotification,
    pub skill_tree_window: SkillTreeWindow,
    pub basic_info_window: BasicInfoWindow,
    pub status_window: StatusWindow,
    pub hotkey_bar: HotkeyBarWindow,
    pub minimap_window: MinimapWindow,
    pub status_icon_bar: StatusIconBarWindow,
    pub levelup_notification: LevelUpNotificationWindow,
    pub guild_window: GuildWindow,
    pub emblem_picker_window: EmblemPickerWindow,
    pub party_friends_window: PartyFriendsWindow,
    pub party_helper_window: PartyHelperWindow,
    pub companion_ai_config_window: CompanionAiConfigWindow,
    pub homunculus_window: HomunWindow,
    pub mercenary_window: MercenaryWindow,
    pub pet_window: PetWindow,
    pub mercenary_skill_window: MercenarySkillWindow,
    pub homun_skill_window: HomunSkillWindow,
    pub world_map_window: WorldMapWindow,
    pub context_menu: ContextMenu,
}

impl Default for Windows {
    fn default() -> Self {
        Self::new()
    }
}

impl Windows {
    pub fn new() -> Self {
        Self {
            chat_window: ChatWindow::new(),
            equipment_window: EquipmentWindow::new(),
            inventory_window: InventoryWindow::new(),
            cart_window: CartWindow::new(),
            storage_window: StorageWindow::new(),
            trade_window: TradeWindow::new(),
            mailbox_window: MailboxWindow::new(),
            read_mail_window: ReadMailWindow::new(),
            cart_select_window: CartSelectWindow::new(),
            npc_dialog: NpcDialog::new(),
            warp_list_window: WarpListWindow::new(),
            item_list_selection_window: ItemListSelectionWindow::new(),
            make_item_window: MakeItemWindow::new(),
            vending_shop_window: VendingShopWindow::new(),
            vending_setup_window: VendingSetupWindow::new(),
            my_shop_window: MyShopWindow::new(),
            confirm_dialog: ConfirmDialog::new(),
            npc_shop: NpcShop::new(),
            chat_room_create_window: ChatRoomCreateWindow::new(),
            chat_room_member_window: ChatRoomMemberWindow::new(),
            emotion_window: EmotionWindow::new(),
            shortcut_list_window: ShortcutListWindow::new(),
            quest_window: QuestWindow::new(),
            quest_detail_window: QuestDetailWindow::new(),
            system_menu: SystemMenu::new(),
            map_missing_window: MapMissingWindow::new(),
            drop_dialog_has_grf_textures: false,
            drop_quantity_dialog: None,
            guild_expel_dialog: None,
            skill_talkbox_dialog: None,
            card_insert_dialog: None,
            card_insert_dialog_has_grf_textures: false,
            item_info_window: ItemInfoWindow::new(),
            book_window: BookWindow::new(),
            sound_options: SoundOptionsWindow::new(),
            graphic_options: GraphicOptionsWindow::new(),
            hotkey_config_window: HotkeyConfigWindow::new(),
            item_pickup_notification: ItemPickupNotification::new(),
            skill_tree_window: SkillTreeWindow::new(),
            basic_info_window: BasicInfoWindow::new(),
            status_window: StatusWindow::new(),
            hotkey_bar: HotkeyBarWindow::new(),
            minimap_window: MinimapWindow::new(),
            status_icon_bar: StatusIconBarWindow::new(),
            levelup_notification: LevelUpNotificationWindow::new(),
            guild_window: GuildWindow::new(),
            emblem_picker_window: EmblemPickerWindow::new(),
            party_friends_window: PartyFriendsWindow::new(),
            party_helper_window: PartyHelperWindow::new(),
            companion_ai_config_window: CompanionAiConfigWindow::new(),
            homunculus_window: HomunWindow::new(),
            mercenary_window: MercenaryWindow::new(),
            pet_window: PetWindow::new(),
            mercenary_skill_window: MercenarySkillWindow::new(),
            homun_skill_window: HomunSkillWindow::new(),
            world_map_window: WorldMapWindow::new(),
            context_menu: ContextMenu::new(),
        }
    }
}

/// How the driver reaches a window's `build` given its id. `Trait` yields a
/// short-lived `&mut dyn InGameWindow` borrow; `VendingAvailable` is the second
/// view onto `vending_setup_window` (its `build_available`), which can't also
/// appear as a `&mut dyn` accessor of the same field.
pub(crate) enum Dispatch {
    Trait(fn(&mut Windows) -> &mut dyn InGameWindow),
    VendingAvailable,
}

/// The single source for both id→window dispatch and registration (fallback
/// build) order. Registration order is back-to-front z-order for windows not
/// yet self-registered in the frame's z-order.
pub(crate) const REGISTRY: &[(WidgetId, Dispatch)] = &[
    (
        BASIC_INFO_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.basic_info_window as &mut dyn InGameWindow),
    ),
    (
        CHAT_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.chat_window as &mut dyn InGameWindow),
    ),
    (
        INV_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.inventory_window as &mut dyn InGameWindow),
    ),
    (
        CART_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.cart_window as &mut dyn InGameWindow),
    ),
    (
        STORAGE_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.storage_window as &mut dyn InGameWindow),
    ),
    (
        TRADE_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.trade_window as &mut dyn InGameWindow),
    ),
    (
        MAILBOX_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.mailbox_window as &mut dyn InGameWindow),
    ),
    (
        READ_MAIL_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.read_mail_window as &mut dyn InGameWindow),
    ),
    (
        CART_SELECT_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.cart_select_window as &mut dyn InGameWindow),
    ),
    (
        MAKE_ITEM_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.make_item_window as &mut dyn InGameWindow),
    ),
    (
        VENDING_SHOP_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.vending_shop_window as &mut dyn InGameWindow),
    ),
    (
        VENDING_SETUP_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.vending_setup_window as &mut dyn InGameWindow),
    ),
    (VENDING_AVAILABLE_WINDOW_ID, Dispatch::VendingAvailable),
    (
        MY_SHOP_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.my_shop_window as &mut dyn InGameWindow),
    ),
    (
        EQ_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.equipment_window as &mut dyn InGameWindow),
    ),
    (
        SKILL_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.skill_tree_window as &mut dyn InGameWindow),
    ),
    (
        STATUS_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.status_window as &mut dyn InGameWindow),
    ),
    (
        PARTY_FRIENDS_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.party_friends_window as &mut dyn InGameWindow),
    ),
    (
        GUILD_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.guild_window as &mut dyn InGameWindow),
    ),
    (
        EMBLEM_PICKER_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.emblem_picker_window as &mut dyn InGameWindow),
    ),
    (
        PARTY_HELPER_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.party_helper_window as &mut dyn InGameWindow),
    ),
    (
        HOMUN_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.homunculus_window as &mut dyn InGameWindow),
    ),
    (
        MERCENARY_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.mercenary_window as &mut dyn InGameWindow),
    ),
    (
        PET_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.pet_window as &mut dyn InGameWindow),
    ),
    (
        MERCENARY_SKILL_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.mercenary_skill_window as &mut dyn InGameWindow),
    ),
    (
        HOMUN_SKILL_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.homun_skill_window as &mut dyn InGameWindow),
    ),
    (
        BOOK_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.book_window as &mut dyn InGameWindow),
    ),
    (
        SOUND_OPTIONS_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.sound_options as &mut dyn InGameWindow),
    ),
    (
        GRAPHIC_OPTIONS_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.graphic_options as &mut dyn InGameWindow),
    ),
    (
        HOTKEY_CONFIG_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.hotkey_config_window as &mut dyn InGameWindow),
    ),
    (
        COMPANION_AI_CONFIG_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.companion_ai_config_window as &mut dyn InGameWindow),
    ),
    (
        CHAT_ROOM_CREATE_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.chat_room_create_window as &mut dyn InGameWindow),
    ),
    (
        CHAT_ROOM_MEMBER_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.chat_room_member_window as &mut dyn InGameWindow),
    ),
    (
        EMOTION_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.emotion_window as &mut dyn InGameWindow),
    ),
    (
        SHORTCUT_LIST_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.shortcut_list_window as &mut dyn InGameWindow),
    ),
    (
        QUEST_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.quest_window as &mut dyn InGameWindow),
    ),
    (
        QUEST_DETAIL_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.quest_detail_window as &mut dyn InGameWindow),
    ),
    (
        WORLD_MAP_WINDOW_ID,
        Dispatch::Trait(|w| &mut w.world_map_window as &mut dyn InGameWindow),
    ),
];
