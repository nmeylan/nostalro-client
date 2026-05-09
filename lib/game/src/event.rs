use crate::inventory::{EquipmentItemData, NormalItemData};
use models::enums::action::ActionType;
use models::enums::skill::SkillTargetType;
use models::enums::vanish::VanishType;
use packets::packets::{CharacterInfoNeoUnion, ServerAddr};

#[derive(Debug)]
pub enum GameEvent {
    // Network → Game (server responses)
    LoginAccepted {
        account_id: u32,
        login_id1: i32,
        login_id2: u32,
        sex: u8,
        servers: Vec<ServerInfo>,
    },
    LoginRefused {
        error_code: u8,
    },
    CharacterListReceived {
        characters: Vec<CharacterInfo>,
    },
    ZoneServerConnectInfo {
        char_id: u32,
        map_name: String,
        ip: u32,
        port: i16,
    },
    MapEntered {
        x: u16,
        y: u16,
        dir: u8,
        tick: u32,
    },
    MapChanged {
        map_name: String,
        x: i16,
        y: i16,
    },
    ServerTick {
        server_tick: u32,
        local_send_time_ms: u32,
    },
    Disconnected(String),

    // Game → Network (user-initiated requests)
    RequestLogin {
        username: String,
        password: String,
    },
    RequestSelectServer {
        index: usize,
    },
    RequestSelectCharacter {
        slot: u8,
    },
    RequestMove {
        x: u16,
        y: u16,
    },
    PlayerMoved {
        start_x: u16,
        start_y: u16,
        dest_x: u16,
        dest_y: u16,
        start_time: u32,
    },
    BackToLogin,
    BackToServerSelect,
    BackToCharacterSelect,
    RestartAck,
    QuitGame,

    // Entity lifecycle
    EntitySpawned {
        gid: u32,
        job: u16,
        speed: u16,
        sex: u8,
        head: u16,
        weapon: u16,
        shield: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        hair_color: u16,
        x: u16,
        y: u16,
        direction: u8,
        body_state: i16,
    },
    EntityMoved {
        gid: u32,
        start_x: u16,
        start_y: u16,
        dest_x: u16,
        dest_y: u16,
        start_time: u32,
    },
    EntityVanished {
        gid: u32,
        vanish_type: VanishType,
    },
    EntityStopMove {
        gid: u32,
        x: u16,
        y: u16,
    },
    EntityAction {
        gid: u32,
        target_gid: u32,
        action: ActionType,
        damage: i32,
        left_damage: i32,
        attack_mt: i32,
        attacked_mt: i32,
        start_time: u32,
        count: i16,
    },
    EntityDirectionChanged {
        gid: u32,
        head_dir: u8,
        dir: u8,
    },
    EntityNameReceived {
        gid: u32,
        name: String,
    },
    EntityHpChanged {
        gid: u32,
        hp: u32,
        max_hp: u32,
    },

    // Stats & parameters
    ParameterChanged {
        var_id: u16,
        value: i32,
    },
    StatusChanged {
        status_type: u32,
        base: i32,
        bonus: i32,
    },
    AttackRangeChanged {
        range: i16,
    },
    EntitySpriteChanged {
        gid: u32,
        sprite_type: u8,
        value: u16,
        value2: u16,
    },

    // Skills (effects & casting)
    SkillCasting {
        gid: u32,
        target_gid: u32,
        skill_id: u16,
        delay_ms: u32,
        x: i16,
        y: i16,
    },
    SkillCastCancel {
        gid: u32,
    },
    SkillFailed {
        skill_id: u16,
        cause: u8,
    },
    ActionFailure,
    SkillPostDelay {
        skill_id: u16,
        delay_ms: u32,
    },
    AfterCastDelay {
        delay_ms: u32,
    },
    SkillDamage {
        skill_id: u16,
        src_gid: u32,
        target_gid: u32,
        damage: i32,
        attack_mt: i32,
        attacked_mt: i32,
        count: i16,
        action: ActionType,
        skill_name: Option<String>,
    },
    SkillNoDamage {
        skill_id: u16,
        src_gid: u32,
        target_gid: u32,
        skill_name: Option<String>,
    },
    GroundSkill {
        skill_id: u16,
        src_gid: u32,
        level: i16,
        x: i16,
        y: i16,
    },

    // Effects
    EntityEmotion {
        gid: u32,
        emotion_type: u8,
    },

    // Chat
    ChatMessage {
        gid: u32,
        message: String,
    },
    OwnChatMessage {
        message: String,
    },
    RequestSendChat {
        message: String,
    },

    // NPC dialog (server → client)
    NpcDialogText {
        npc_id: u32,
        text: String,
    },
    NpcDialogNext {
        npc_id: u32,
    },
    NpcDialogClose {
        npc_id: u32,
    },
    NpcDialogMenu {
        npc_id: u32,
        items: Vec<String>,
    },
    NpcInputNumber {
        npc_id: u32,
    },
    NpcInputString {
        npc_id: u32,
    },
    NpcDealTypeSelect {
        npc_id: u32,
    },

    // NPC dialog (client → server)
    RequestNpcContact {
        npc_id: u32,
    },
    RequestNpcNext {
        npc_id: u32,
    },
    RequestNpcClose {
        npc_id: u32,
    },
    RequestNpcMenuSelect {
        npc_id: u32,
        choice: u8,
    },
    RequestNpcInputNumber {
        npc_id: u32,
        value: i32,
    },
    RequestNpcInputString {
        npc_id: u32,
        text: String,
    },
    RequestNpcDealType {
        npc_id: u32,
        deal_type: u8,
    },

    // NPC shop (server → client)
    NpcShopBuyList {
        npc_id: u32,
        items: Vec<(u16, i32, i32, u8)>,
    },
    NpcShopSellList {
        npc_id: u32,
        items: Vec<(i16, i32, i32)>,
    },
    NpcShopBuyResult {
        result: u8,
    },
    NpcShopSellResult {
        result: u8,
    },

    // NPC shop (client → server)
    RequestNpcShopBuy {
        items: Vec<(i16, u16)>,
    },
    RequestNpcShopSell {
        items: Vec<(i16, i16)>,
    },
    RequestNpcShopClose,

    // Inventory (server → client)
    InventoryNormalItems {
        items: Vec<NormalItemData>,
    },
    InventoryEquipmentItems {
        items: Vec<EquipmentItemData>,
    },
    InventoryItemPickup {
        index: u16,
        item_id: u16,
        count: u16,
        item_type: u8,
        is_identified: bool,
        is_damaged: bool,
        refining_level: u8,
        slot: [u16; 4],
        location: u16,
        result: u8,
    },
    InventoryUseItemResult {
        index: u16,
        count: i16,
        success: bool,
    },
    InventoryEquipResult {
        index: u16,
        wear_location: u16,
        view_id: u16,
        success: bool,
    },
    InventoryArrowEquipped {
        index: u16,
    },
    InventoryUnequipResult {
        index: u16,
        wear_location: u16,
        success: bool,
    },
    InventoryItemRemoved {
        index: u16,
        count: i16,
    },

    // Inventory (client → server)
    RequestUseItem {
        index: u16,
    },
    RequestEquipItem {
        index: u16,
        location: u16,
    },
    RequestUnequipItem {
        index: u16,
    },
    RequestDropItem {
        index: u16,
        count: i16,
    },

    // Card composition (client → server)
    RequestCardInsertList {
        card_index: u16,
    },
    RequestCardInsert {
        card_index: u16,
        equip_index: u16,
    },

    // Card composition (server → client)
    CardInsertItemList {
        card_index: u16,
        equip_indices: Vec<u16>,
    },
    CardInsertResult {
        equip_index: u16,
        card_index: u16,
        result: u8,
    },

    // Floor items (server → client)
    FloorItemAppeared {
        id: u32,
        item_id: u16,
        is_identified: bool,
        x: i16,
        y: i16,
        sub_x: u8,
        sub_y: u8,
        count: i16,
        is_falling: bool,
    },
    FloorItemDisappeared {
        id: u32,
    },

    // Floor items (client → server)
    RequestPickupItem {
        id: u32,
    },

    // Skills (server → client)
    SkillListReceived {
        skills: Vec<SkillInfo>,
    },
    SkillUpdated {
        id: u16,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        upgradable: bool,
    },
    SkillAdded {
        skill: SkillInfo,
    },

    // Skills (client → server)
    RequestSkillLevelUp {
        skill_id: u16,
    },

    // Stats (client → server)
    RequestStatChange {
        status_id: u16,
        amount: u8,
    },

    // UI (client-internal)
    ToggleStatusWindow,

    // Hotkeys (server → client)
    HotkeyListReceived {
        slots: Vec<(i8, u32, i16)>,
    },

    // Hotkeys (client → server)
    RequestHotkeyChange {
        index: u16,
        is_skill: bool,
        id: u32,
        count: i16,
    },
    RequestUseSkill {
        skill_id: u16,
        level: i16,
    },

    // UI actions
    ShowItemInfo {
        index: u16,
    },
    ShowCardInfo {
        item_id: u16,
    },
    ShowCardIllustration {
        item_id: u16,
    },

    // UI lifecycle
    DialogClosed,

    // Window toggle (from basic info menu buttons)
    ToggleInventory,
    ToggleEquipment,
    ToggleSkills,
    ToggleMinimap,

    // No-op acknowledgement (packet parsed but no action needed yet)
    Acknowledged,
}

#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub id: u16,
    pub name: String,
    pub level: i16,
    pub sp_cost: i16,
    pub attack_range: i16,
    pub upgradable: bool,
    pub skill_target_type: SkillTargetType,
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub ip: u32,
    pub port: i16,
    pub name: String,
    pub user_count: u16,
}

impl From<&ServerAddr> for ServerInfo {
    fn from(addr: &ServerAddr) -> Self {
        Self {
            ip: addr.ip,
            port: addr.port,
            name: addr.name.iter().take_while(|c| **c != '\0').collect(),
            user_count: addr.user_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub gid: u32,
    pub name: String,
    pub class: u16,
    pub base_level: u16,
    pub job_level: u32,
    pub map: String,
    pub slot: i8,
    pub head: u16,
    pub hair_color: u16,
    pub weapon: u16,
    pub head_top: u16,
    pub head_mid: u16,
    pub head_bottom: u16,
    pub shield: u16,
    pub sex: u8,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u16,
    pub max_sp: u16,
    pub str: u8,
    pub agi: u8,
    pub vit: u8,
    pub int: u8,
    pub dex: u8,
    pub luk: u8,
}

impl CharacterInfo {
    pub fn from_neo_union(info: &CharacterInfoNeoUnion, packetver: u32) -> Self {
        tracing::debug!(
            "CharInfo raw: gid={} class={} level={} joblevel={} name_raw={:?} slot={} hp={}/{} sp={}/{} speed={}",
            info.gid,
            info.class,
            info.level,
            info.joblevel,
            &info.name_raw[..16],
            info.char_num,
            info.hp,
            info.maxhp,
            info.sp,
            info.maxsp,
            info.speed,
        );
        let name: String = info.name.iter().take_while(|c| **c != '\0').collect();
        let map: String = if packetver >= 20100720 {
            info.last_map.iter().take_while(|c| **c != '\0').collect()
        } else {
            String::new()
        };
        let (hp, max_hp) = if packetver > 20081217 {
            (info.hp, info.maxhp)
        } else {
            (info.hp_16 as u32, info.maxhp_16 as u32)
        };
        let sex = if packetver >= 20141016 { info.sex } else { 0 };

        Self {
            gid: info.gid,
            name,
            class: info.class,
            base_level: info.level,
            job_level: info.joblevel,
            map,
            slot: info.char_num,
            head: info.head,
            hair_color: info.hair_color,
            weapon: info.weapon,
            head_top: info.head_top,
            head_mid: info.head_mid,
            head_bottom: info.head_bottom,
            shield: info.shield,
            sex,
            hp,
            max_hp,
            sp: info.sp,
            max_sp: info.maxsp,
            str: info.str,
            agi: info.agi,
            vit: info.vit,
            int: info.int,
            dex: info.dex,
            luk: info.luk,
        }
    }
}
