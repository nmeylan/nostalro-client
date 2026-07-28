use crate::banner::BannerKind;
use crate::guild::{
    GuildBanEntry, GuildMember, GuildPosition, GuildRelation, GuildSkill, OtherGuild,
};
use crate::inventory::{EquipmentItemData, NormalItemData};
use crate::item::Item;
use crate::mail::{MailEntry, OpenedMail};
use crate::targeting::MapProperties;
use models::enums::action::ActionType;
use models::enums::skill::SkillTargetType;
use models::enums::vanish::VanishType;

#[derive(Debug)]
pub enum GameEvent {
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
    CharServerConnectRefused {
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
    /// The destination map is hosted by a different zone server: reconnect there
    /// and re-enter.
    ZoneServerChanged {
        map_name: String,
        ip: u32,
        port: i16,
    },
    AccessibleMapsReceived {
        maps: Vec<AccessibleMap>,
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
    MapPropertyChanged(MapProperties),
    ServerTick {
        server_tick: u32,
        local_send_time_ms: u32,
    },
    Disconnected(String),
    RequestLogin {
        username: String,
        password: String,
    },
    SelectLoginServer {
        index: usize,
    },
    RequestSelectServer {
        index: usize,
    },
    RequestSelectCharacter {
        slot: u8,
    },
    RequestCreateCharacter {
        slot: u8,
    },
    RequestMakeCharacter {
        name: String,
        slot: u8,
        hair_style: u16,
        hair_color: u16,
        /// STR, AGI, VIT, INT, DEX, LUK. Only sent for packetver < 20120307.
        stats: [u8; 6],
    },
    CharacterCreated {
        character: CharacterInfo,
    },
    CharacterCreateFailed {
        error_code: u8,
    },
    CancelCreateCharacter,
    RequestDeleteCharacterReserve {
        gid: u32,
    },
    RequestDeleteCharacterConfirm {
        gid: u32,
        birthdate: String,
    },
    RequestDeleteCharacterCancel {
        gid: u32,
    },
    CharacterDeleteReserved {
        gid: u32,
        result: u32,
        delete_reserved_date: i32,
    },
    CharacterDeleted {
        gid: u32,
        result: u32,
    },
    CharacterDeleteCancelled {
        gid: u32,
        result: u32,
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
    ReturnToSavePoint,
    RequestStandingResurrection,
    RequestMapRecoveryWarp,
    RestartAck,
    QuitGame,
    DisconnectAck {
        allowed: bool,
    },
    EntitySpawned {
        gid: u32,
        aid: u32,
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
        health_state: i16,
        effect_state: i32,
        base_level: i16,
        is_boss: bool,
        posture: u8,
        guild_id: u32,
        guild_emblem_version: i32,
        is_new_entry: bool,
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
    EntityHighJumped {
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
    EntityNamesReceived {
        gid: u32,
        name: String,
        /// Monsters carry their `show_mob_info` string here instead of a party name.
        party_name: String,
        guild_name: String,
        position_name: String,
    },
    EntityHpChanged {
        gid: u32,
        hp: u32,
        max_hp: u32,
    },
    EntityOptionChanged {
        gid: u32,
        body_state: i16,
        health_state: i16,
        effect_state: i32,
    },
    EntityOpt3Changed {
        gid: u32,
        effect_state: i32,
        base_level: i32,
        opt3: i32,
    },
    PlayEffectOnEntity {
        gid: u32,
        effect_id: i32,
        value: Option<i32>,
    },
    PlayMiscEffectOnEntity {
        gid: u32,
        code: u8,
    },
    SpiritsChanged {
        gid: u32,
        count: u8,
    },
    BladeStop {
        src_gid: u32,
        dest_gid: u32,
        active: bool,
    },
    StatusEffectChanged {
        gid: u32,
        efst: i16,
        active: bool,
        remain_ms: u32,
        val1: i32,
    },
    EntityResurrected {
        gid: u32,
    },
    MvpReward {
        gid: u32,
    },
    ParameterChanged {
        var_id: u16,
        value: i32,
    },
    Recovery {
        var_id: u16,
        amount: i32,
    },
    ExpGained {
        aid: u32,
        amount: i32,
        is_base: bool,
        is_quest: bool,
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
    SkillCasting {
        gid: u32,
        target_gid: u32,
        skill_id: u16,
        property: u32,
        delay_ms: u32,
        x: i16,
        y: i16,
        skill_name: Option<String>,
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
        level: i16,
        action: ActionType,
        start_time: u32,
    },
    SkillNoDamage {
        skill_id: u16,
        src_gid: u32,
        target_gid: u32,
        /// The packet's skill-level field. For AL_HEAL this carries the heal
        /// amount, which selects the green-heal size.
        level: i16,
    },
    GroundSkill {
        skill_id: u16,
        src_gid: u32,
        level: i16,
        x: i16,
        y: i16,
    },
    SkillUnitEntered {
        aid: u32,
        creator_aid: u32,
        x: i16,
        y: i16,
        unit_id: u8,
        is_visible: bool,
    },
    SkillUnitDisappeared {
        aid: u32,
    },
    GraffitiEntered {
        aid: u32,
        creator_aid: u32,
        x: i16,
        y: i16,
        message: String,
    },
    MapCellChanged {
        x: i16,
        y: i16,
        cell_type: i32,
    },
    SkillUnitUpdated {
        aid: u32,
    },
    EntityEmotion {
        gid: u32,
        emotion_type: u8,
    },
    ChatMessage {
        gid: u32,
        message: String,
    },
    OwnChatMessage {
        message: String,
    },
    RankingReceived {
        title: &'static str,
        entries: Vec<(String, i32)>,
    },
    BroadcastMessage {
        message: String,
        color: [f32; 4],
        banner: BannerKind,
    },
    RequestSendChat {
        message: String,
    },
    RequestEmotion {
        emote_type: u8,
    },
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
    WarpList {
        skill_id: u16,
        destinations: Vec<String>,
    },
    RequestSelectWarppoint {
        skill_id: u16,
        map_name: String,
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
    RequestNpcShopBuy {
        items: Vec<(i16, u16)>,
    },
    RequestNpcShopSell {
        items: Vec<(i16, i16)>,
    },
    RequestNpcShopClose,
    ChatRoomUpsert {
        owner_aid: u32,
        room_id: u32,
        max_count: i16,
        cur_count: i16,
        atype: u8,
        title: String,
    },
    ChatRoomDestroy {
        room_id: u32,
    },
    ChatRoomEntered {
        room_id: u32,
        members: Vec<crate::chat_room::ChatRoomMember>,
    },
    ChatRoomJoinRefused {
        result: u8,
    },
    ChatRoomCreateResult {
        flag: u8,
    },
    ChatRoomMemberJoined {
        name: String,
        cur_count: i16,
    },
    ChatRoomMemberLeft {
        name: String,
        cur_count: i16,
        kicked: bool,
    },
    ChatRoomOwnerChanged {
        name: String,
    },
    RequestJoinChatRoom {
        room_id: u32,
    },
    ToggleChatRoomCreate,
    RequestCreateChatRoom {
        title: String,
        limit: i16,
        public: bool,
        password: String,
    },
    RequestChangeChatRoom {
        title: String,
        limit: i16,
        public: bool,
        password: String,
    },
    RequestLeaveChatRoom,
    RequestEditChatRoomSettings,
    RequestKickChatMember {
        name: String,
    },
    RequestChangeChatOwner {
        name: String,
    },
    RequestOpenChatMemberMenu {
        name: String,
        x: f32,
        y: f32,
    },
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
    CartNormalItems {
        items: Vec<NormalItemData>,
    },
    CartEquipmentItems {
        items: Vec<EquipmentItemData>,
    },
    CartItemAdded {
        index: u16,
        item_id: u16,
        count: i16,
        item_type: u8,
        is_identified: bool,
        is_damaged: bool,
        refining_level: u8,
        slot: [u16; 4],
    },
    CartItemRemoved {
        index: u16,
        count: i16,
    },
    CartCountInfo {
        cur_weight: i32,
        max_weight: i32,
        cur_count: i16,
        max_count: i16,
    },
    CartOff,
    StorageNormalItems {
        items: Vec<NormalItemData>,
    },
    StorageEquipItems {
        items: Vec<EquipmentItemData>,
    },
    StorageOpened {
        cur: i16,
        max: i16,
    },
    StorageItemAdded {
        index: u16,
        item_id: u16,
        count: i16,
        item_type: u8,
        is_identified: bool,
        is_damaged: bool,
        refining_level: u8,
        slot: [u16; 4],
    },
    StorageItemRemoved {
        index: u16,
        amount: i16,
    },
    StorageClosed,

    ExchangeRequested {
        name: String,
        gid: u32,
        level: i16,
    },
    ExchangeAckResult {
        result: u8,
        level: i16,
    },
    ExchangeItemAdded {
        item_id: u16,
        item_type: u8,
        count: i32,
        is_identified: bool,
        is_damaged: bool,
        refining_level: u8,
        slot: [u16; 4],
    },
    ExchangeAddResult {
        index: u16,
        result: u8,
    },
    ExchangeConcluded {
        who: u8,
    },
    ExchangeCanceled,
    ExchangeCompleted {
        result: u8,
    },
    ExchangeUndo,
    RequestExchangeItem {
        target_aid: u32,
    },
    RespondExchangeRequest {
        accept: bool,
    },
    RequestAddExchangeItem {
        index: u16,
        count: i32,
    },
    RequestConcludeExchange,
    RequestCancelExchange,
    RequestExecExchange,

    MailWindow {
        open: bool,
    },
    MailInboxReceived {
        entries: Vec<MailEntry>,
    },
    MailOpened {
        mail: OpenedMail,
    },
    MailDeleteAck {
        mail_id: u32,
        ok: bool,
    },
    MailGetItemAck {
        result: u8,
    },
    MailAddItemAck {
        index: u16,
        ok: bool,
    },
    MailSendAck {
        ok: bool,
    },
    MailNewReceived {
        mail_id: u32,
        title: String,
        sender: String,
    },
    MailReturnAck {
        mail_id: u32,
        ok: bool,
    },
    RequestMailList,
    RequestMailOpen {
        mail_id: u32,
    },
    RequestMailDelete {
        mail_id: u32,
    },
    RequestMailGetItem {
        mail_id: u32,
    },
    RequestMailResetItem {
        ty: u8,
    },
    RequestMailAddItem {
        index: u16,
        amount: u32,
    },
    RequestMailSend {
        to: String,
        title: String,
        body: String,
    },
    RequestMailReturn {
        mail_id: u32,
        sender: String,
    },

    RequestMoveItemBodyToStore {
        index: u16,
        count: i16,
    },
    RequestDepositItem {
        index: u16,
    },
    RequestMoveItemStoreToBody {
        index: u16,
        count: i16,
    },
    RequestCloseStorage,
    RequestMoveItemBodyToCart {
        index: u16,
        count: i16,
    },
    RequestMoveItemCartToBody {
        index: u16,
        count: i16,
    },
    RequestMoveItemStoreToCart {
        index: u16,
        count: i16,
    },
    RequestMoveItemCartToStore {
        index: u16,
        count: i16,
    },
    RequestCartOff,
    RequestSetCartPick {
        pick_equip: bool,
        pick_usable: bool,
        pick_etc: bool,
    },
    RequestChangeCart {
        num: i16,
    },
    ToggleCart,
    RequestCardInsertList {
        card_index: u16,
    },
    RequestCardInsert {
        card_index: u16,
        equip_index: u16,
    },
    CardInsertItemList {
        card_index: u16,
        equip_indices: Vec<u16>,
    },
    CardInsertResult {
        equip_index: u16,
        card_index: u16,
        result: u8,
    },
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
    RequestPickupItem {
        id: u32,
    },
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
    RequestSkillLevelUp {
        skill_id: u16,
    },
    RequestStatChange {
        status_id: u16,
        amount: u8,
    },
    ToggleStatusWindow,
    HotkeyListReceived {
        slots: Vec<(i8, u32, i16)>,
    },
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
    RequestCompanionUseSkill {
        is_mercenary: bool,
        skill_id: u16,
        level: i16,
    },
    AutoCastSkill {
        skill_id: u16,
        level: i16,
    },
    ShowItemInfo {
        index: u16,
    },
    ShowItemInfoDirect {
        item: Box<Item>,
    },
    ShowCardInfo {
        item_id: u16,
    },
    ShowCardIllustration {
        item_id: u16,
    },
    ReadBook {
        item_id: u16,
    },
    RequestRemoveOption,
    ShowSystemMessage {
        message: String,
    },
    WhisperSettingResult {
        allow: bool,
        result: u8,
        all: bool,
    },
    MemoResult {
        result: u8,
    },
    ProgressBarStarted {
        duration_secs: u32,
    },
    ProgressBarCancelled,
    ServerMsg {
        msg_id: u16,
    },
    DialogClosed,
    ToggleInventory,
    ToggleEquipment,
    ToggleSkills,
    ToggleEmotionWindow,
    ToggleShortcutList,
    ShortcutBindingsChanged(Vec<String>),
    ToggleMinimap,
    ToggleSoundOptions,
    SoundSettingsChanged {
        bgm_volume: f32,
        sfx_volume: f32,
        bgm_enabled: bool,
        sfx_enabled: bool,
        persist: bool,
    },
    ToggleGraphicOptions,
    ToggleSystemMenu,
    ToggleHotkeyConfig,
    GraphicsSettingsChanged {
        ui_scale: f32,
        fullscreen: bool,
        fog: bool,
        show_skill_effects: bool,
        display: crate::display::DisplayOptions,
        refuse_trade: bool,
        refuse_party_invite: bool,
        persist: bool,
    },
    SoundEffect {
        name: String,
        act: u8,
        term_ms: u32,
        gid: u32,
    },
    PartyCreateResult {
        result: u8,
    },
    PartyMemberList {
        name: String,
        members: Vec<PartyMemberData>,
    },
    PartyMemberAdded {
        aid: u32,
        name: String,
        map: String,
        leader: bool,
        online: bool,
        x: u16,
        y: u16,
    },
    PartyMemberRemoved {
        aid: u32,
        name: String,
        result: u8,
    },
    PartyMemberHp {
        aid: u32,
        hp: u32,
        max_hp: u32,
    },
    PartyMemberPosition {
        aid: u32,
        x: u16,
        y: u16,
    },
    PartyInviteReceived {
        party_grid: u32,
        party_name: String,
    },
    PartyInviteResult {
        name: String,
        answer: u8,
    },
    PartyExpOptionChanged {
        exp_option: u32,
    },
    PartyConfigChanged {
        exp_option: u32,
        item_pickup_rule: u8,
        item_division_rule: u8,
    },
    SelfConfigChanged {
        kind: SelfConfigKind,
        enabled: bool,
    },
    RequestSetConfig {
        kind: SelfConfigKind,
        enabled: bool,
    },
    PartyChatMessage {
        aid: u32,
        message: String,
    },
    GuildChatMessage {
        message: String,
    },
    WhisperReceived {
        sender: String,
        message: String,
    },
    WhisperAck {
        result: u8,
    },
    RequestSendWhisper {
        name: String,
        message: String,
    },
    TogglePartyWindow,
    RequestPartyInvite {
        target_aid: u32,
    },
    RespondPartyInvite {
        party_grid: u32,
        accept: bool,
    },
    RequestLeaveParty,
    RequestExpelMember {
        aid: u32,
        name: String,
    },
    RequestPartyExpOption {
        exp_share: bool,
    },
    SendPartyChat {
        message: String,
    },
    RequestPartyCreate {
        name: String,
        item_pickup_rule: u8,
        item_division_rule: u8,
    },
    RequestChangePartyLeader {
        aid: u32,
    },
    RequestPartyInviteByName {
        name: String,
    },
    /// Open the party helper child window. mode: 0=create, 1=invite, 2=setup.
    ShowPartyHelper {
        mode: u8,
    },

    // --- Friends ---
    FriendListReceived {
        friends: Vec<FriendData>,
    },
    FriendStateChanged {
        aid: u32,
        gid: u32,
        online: bool,
    },
    FriendAddResult {
        result: u8,
        aid: u32,
        gid: u32,
        name: String,
    },
    FriendRemoved {
        aid: u32,
        gid: u32,
    },
    FriendRequestReceived {
        req_aid: u32,
        req_gid: u32,
        name: String,
    },
    ToggleFriendWindow,
    RequestAddFriend {
        name: String,
    },
    RespondFriendRequest {
        req_aid: u32,
        req_gid: u32,
        accept: bool,
    },
    RequestDeleteFriend {
        aid: u32,
        gid: u32,
    },
    RequestWhisper {
        name: String,
    },

    // --- Guild ---
    GuildMenuFlag {
        flag: i32,
    },
    GuildInfo {
        gdid: u32,
        name: String,
        level: i32,
        exp: i32,
        max_exp: i32,
        member_num: i32,
        max_member_num: i32,
        avg_level: i32,
        point: i32,
        honor: i32,
        virtue: i32,
        master_name: String,
        manage_land: String,
        emblem_version: i32,
    },
    GuildMembers {
        members: Vec<GuildMember>,
    },
    GuildPositions {
        positions: Vec<GuildPosition>,
    },
    GuildPositionNames {
        names: Vec<(i32, String)>,
    },
    GuildMemberPositionsChanged {
        entries: Vec<(u32, u32, i32)>,
    },
    GuildMemberPosition {
        aid: u32,
        x: u16,
        y: u16,
    },
    GuildMemberOnline {
        aid: u32,
        gid: u32,
        online: bool,
        appearance: Option<GuildMemberAppearance>,
    },
    GuildSkills {
        point: i16,
        skills: Vec<GuildSkill>,
    },
    GuildBanList {
        entries: Vec<GuildBanEntry>,
    },
    GuildNotice {
        subject: String,
        body: String,
    },
    GuildOtherList {
        guilds: Vec<OtherGuild>,
    },
    GuildRelations {
        relations: Vec<GuildRelation>,
    },
    GuildEmblem {
        gdid: u32,
        version: i32,
        bmp: Vec<u8>,
    },
    GuildIdentityUpdated {
        gdid: u32,
        emblem_version: i32,
        right: i32,
        is_master: bool,
        name: String,
    },
    GuildCreateResult {
        result: u8,
    },
    GuildMemberLeft {
        name: String,
        reason: String,
    },
    GuildMemberExpelled {
        name: String,
        reason: String,
    },
    EntityGuildChanged {
        aid: u32,
        gdid: u32,
        emblem_version: i32,
    },
    GuildDisbandResult {
        reason: i32,
    },
    GuildInviteReceived {
        gdid: u32,
        name: String,
    },
    RespondGuildInvite {
        gdid: u32,
        accept: bool,
    },
    GuildAllyRequestReceived {
        aid: u32,
        name: String,
    },
    RespondGuildAlly {
        aid: u32,
        accept: bool,
    },
    GuildAllyResult {
        answer: u8,
    },
    GuildHostileResult {
        result: u8,
    },
    GuildJoinResult {
        answer: u8,
    },
    GuildRelationDeleted {
        gdid: u32,
        relation: i32,
    },
    GuildRelationAdded {
        gdid: u32,
        relation: i32,
        name: String,
    },
    /// UI → client: window opened, fetch all guild tab data.
    RequestGuildInfoBurst,
    RequestGuildMenu {
        atype: i32,
    },
    /// UI → client: right-clicked a guild member row; open the player menu here.
    ShowGuildMemberMenu {
        aid: u32,
        gid: u32,
        name: String,
        x: f32,
        y: f32,
    },
    RequestSetGuildNotice {
        subject: String,
        body: String,
    },
    RequestGuildLeave,
    RequestGuildExpel {
        aid: u32,
        gid: u32,
        name: String,
    },
    RequestChangeMemberPosition {
        aid: u32,
        gid: u32,
        position_id: i32,
    },
    RequestChangePositionInfo {
        positions: Vec<GuildPosition>,
    },
    RequestUpgradeGuildSkill {
        skid: u16,
    },
    RequestGuildInvite {
        target_aid: u32,
    },
    RequestGuildAlly {
        target_aid: u32,
    },
    RequestGuildHostile {
        target_aid: u32,
    },
    RequestDeleteGuildRelation {
        gdid: u32,
        relation: i32,
    },
    ConfirmedSkillTalkbox {
        skill_id: u16,
        level: i16,
        x: i16,
        y: i16,
        message: String,
    },
    ConfirmedGuildExpel {
        aid: u32,
        gid: u32,
        name: String,
        reason: String,
    },
    ConfirmedGuildLeave,
    ConfirmedDeleteGuildRelation {
        gdid: u32,
        relation: i32,
    },
    /// UI → client: open the emblem picker for the configured folder.
    RequestSelectEmblem,
    /// UI → client: upload the chosen emblem BMP file.
    RequestUploadEmblem {
        path: String,
    },

    // --- Skill-triggered production / selection windows ---
    ItemIdentifyList {
        indices: Vec<u16>,
    },
    ItemIdentifyResult {
        index: i16,
        ok: bool,
    },
    RequestIdentifyItem {
        index: i16,
    },
    MakingArrowList {
        item_ids: Vec<u16>,
    },
    RequestMakingArrow {
        item_id: u16,
    },
    MakableItemList {
        item_ids: Vec<u16>,
    },
    MakingItemResult {
        result: i16,
        item_id: u16,
    },
    RequestMakingItem {
        item_id: u16,
        materials: [u16; 3],
    },
    WeaponRefineList {
        items: Vec<RefineItemRow>,
    },
    WeaponRefineResult {
        result: i32,
        item_id: u16,
    },
    RequestWeaponRefine {
        index: i32,
    },
    RepairItemList {
        items: Vec<RefineItemRow>,
    },
    RepairItemResult {
        index: i16,
        ok: bool,
    },
    RequestRepairItem {
        index: i16,
        item_id: u16,
        refine: u8,
        cards: [u16; 4],
    },
    AutoSpellList {
        skill_ids: Vec<i32>,
    },
    RequestSelectAutoSpell {
        skill_id: i32,
    },

    // --- Vending ---
    OpenVendingSetup {
        max_items: i16,
    },
    RequestOpenStore {
        shop_name: String,
        items: Vec<(i16, i16, i32)>,
    },
    RequestCancelVendingSetup,
    RequestCloseStore,
    VendingOwnStock {
        items: Vec<VendorItem>,
    },
    VendingBoardShown {
        aid: u32,
        name: String,
    },
    VendingBoardHidden {
        aid: u32,
    },
    RequestBuyFromVendor {
        aid: u32,
    },
    VendingShopList {
        aid: u32,
        unique_id: u32,
        items: Vec<VendorItem>,
    },
    RequestPurchaseFromVendor {
        aid: u32,
        unique_id: u32,
        items: Vec<(i16, i16)>,
    },
    VendingPurchaseResult {
        index: i16,
        curcount: i16,
        result: u8,
    },
    VendingStockDecrement {
        index: i16,
        count: i16,
    },
    VendingOpenResult {
        result: u8,
    },

    // --- Homunculus / Mercenary ---
    HomunPropertyReceived {
        property: HomunculusProperty,
    },
    /// 0x0230: state 0 = pre-init (carries the companion GID), 1 = intimacy, 2 = hunger.
    CompanionStateChanged {
        state: i8,
        gid: u32,
        data: i32,
    },
    HomunFeedResult {
        success: bool,
        item_id: u16,
    },
    MercenaryInfoReceived {
        info: MercenaryInfo,
        is_init: bool,
    },
    MercenaryParamChanged {
        var: u16,
        value: i32,
    },
    HomunParamChanged {
        var: u16,
        value: i32,
    },
    HomunSkillList {
        skills: Vec<SkillInfo>,
    },
    HomunSkillUpdate {
        id: u16,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        upgradable: bool,
    },
    MercenarySkillList {
        skills: Vec<SkillInfo>,
    },
    MercenarySkillUpdate {
        id: u16,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        upgradable: bool,
    },
    RequestCompanionMove {
        gid: u32,
        x: i32,
        y: i32,
    },
    RequestCompanionAttack {
        gid: u32,
        target_gid: u32,
    },
    RequestCompanionMoveToOwner {
        gid: u32,
    },
    /// 0 = info, 1 = feed, 2 = delete (permanent).
    RequestHomunMenu {
        command: u8,
    },
    /// User asked to delete the homunculus; needs confirmation before it fires
    /// `RequestHomunMenu { command: 2 }`.
    RequestHomunDelete,
    /// Rest button: self-cast the Rest skill to vaporize the homunculus.
    RequestHomunRest,
    /// 1 = info, 2 = dismiss.
    RequestMercenaryCommand {
        command: i8,
    },
    RequestRenameHomun {
        name: String,
    },
    ToggleHomunculusWindow,
    ToggleMercenaryWindow,
    ToggleMercenarySkillWindow,
    ToggleHomunSkillWindow,
    /// Standby button: toggle the companion between follow and hold.
    ToggleCompanionStandby {
        is_mercenary: bool,
    },
    ToggleCompanionAiConfig,
    SaveCompanionAiConfig,
    RevertCompanionAiConfig,
    ResetCompanionAiConfig,

    // --- Pet ---
    /// 0x19e: server armed pet-capture targeting.
    PetCaptureStart,
    /// 0x1a0: capture attempt resolved.
    PetCaptureResult {
        ok: bool,
    },
    /// 0x1a2: full pet info; refreshes pet data without opening the window.
    PetProperty {
        property: PetProperty,
    },
    /// 0x1a3: feed attempt resolved.
    PetFeedResult {
        ok: bool,
        food_item_id: u16,
    },
    /// 0x1a4: incremental pet state (ty: 0 init/1 intimacy/2 hunger/3 accessory/4 performance/5 marker).
    PetStateChanged {
        ty: i8,
        gid: u32,
        data: i32,
    },
    /// 0x1a6: eggs available to hatch (inventory indices).
    PetEggList {
        indices: Vec<u16>,
    },
    /// 0x1aa: pet emote / talk broadcast.
    PetAct {
        gid: u32,
        data: i32,
    },
    /// 0x19f: send capture attempt against the picked mob.
    RequestTryCapture {
        gid: u32,
    },
    /// 0x1a1: pet command (cSub 0 info / 1 feed / 2 perform / 3 return-to-egg / 4 unequip).
    RequestPetCommand {
        csub: i8,
    },
    /// 0x1a5: rename the pet (allowed once).
    RequestRenamePet {
        name: String,
    },
    /// 0x1a7: hatch the chosen egg.
    RequestSelectPetEgg {
        index: u16,
    },
    /// 0x1a9: owner-generated pet emote / talk.
    RequestPetAct {
        data: i32,
    },
    /// Feed command chosen: opens the confirm dialog before sending cSub=1.
    RequestPetFeed,
    TogglePetWindow,

    // --- Quest ---
    /// 0x2b1: full quest list (id + active flag). Clears the log first.
    QuestListReceived {
        quests: Vec<crate::quest::QuestListEntry>,
    },
    /// 0x2b2: mission data (objectives + expiry) for the listed quests.
    QuestMissionsReceived {
        missions: Vec<crate::quest::QuestMissionData>,
    },
    /// 0x2b3: a quest was received.
    QuestAdded {
        quest: crate::quest::QuestMissionData,
    },
    /// 0x2b4: a quest was removed (also means "completed").
    QuestRemoved {
        quest_id: u32,
    },
    /// 0x2b5: hunt progress — the sole source of required totals.
    QuestHuntUpdated {
        entries: Vec<crate::quest::QuestHuntEntry>,
    },
    /// 0x2b7: active-flag ack.
    QuestActiveChanged {
        quest_id: u32,
        active: bool,
    },
    /// 0x446: over-NPC quest marker.
    QuestNpcMarker {
        npc_id: u32,
        x: u16,
        y: u16,
        effect: i16,
        color: u8,
    },
    /// UI: right-click a row → send CZ_ACTIVE_QUEST.
    RequestToggleQuestActive {
        quest_id: u32,
        active: bool,
    },
    /// UI: View button → open the detail window for the quest.
    OpenQuestDetail {
        quest_id: u32,
    },
    ToggleQuestWindow,

    // --- Marriage ---
    /// 0x1e6: partner's name (empty = no partner), broadcast when someone finishes
    /// casting WE_CALLPARTNER.
    CoupleNameReceived {
        name: String,
    },
    /// 0x1ea: wedding celebration on an actor → confetti effect + sound.
    WeddingCelebration {
        account_id: u32,
    },
    /// 0x205: you are divorced from `name`.
    Divorced {
        name: String,
    },
    /// 0x1b3: NPC cutin illustration; `position` 0/1/2 = bottom left/middle/right,
    /// 255 = clear.
    NpcCutin {
        image: String,
        position: u8,
    },

    // --- Adoption ---
    /// 0x1f6: a married couple wishes to adopt the local player. `father_aid`/
    /// `mother_aid` are the two parents' account ids, echoed back unchanged on reply.
    AdoptionRequested {
        father_aid: u32,
        mother_aid: u32,
        name: String,
    },
    /// 0x216: adoption failure notice to the requesting parent.
    AdoptionMessage {
        msg_no: i32,
    },
    RequestAdoption {
        target_aid: u32,
    },
    RespondAdoptionRequest {
        accept: bool,
    },

    Acknowledged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfConfigKind {
    RefusePartyInvite,
    OpenEquipmentWindow,
    Call,
    PetAutofeed,
    HomunculusAutofeed,
}

impl SelfConfigKind {
    /// Value carried by the `config` field of CZ_CONFIG / ZC_CONFIG.
    pub fn config_id(self) -> i32 {
        match self {
            SelfConfigKind::OpenEquipmentWindow => 0,
            SelfConfigKind::Call => 1,
            SelfConfigKind::PetAutofeed => 2,
            SelfConfigKind::HomunculusAutofeed => 3,
            SelfConfigKind::RefusePartyInvite => -1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HomunculusProperty {
    pub name: String,
    pub renamed: bool,
    pub vaporized: bool,
    pub level: i16,
    pub hunger: i16,
    pub intimacy: i16,
    pub accessory: u16,
    pub atk: i16,
    pub matk: i16,
    pub hit: i16,
    pub critical: i16,
    pub def: i16,
    pub mdef: i16,
    pub flee: i16,
    pub aspd: i16,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub exp: i32,
    pub max_exp: i32,
    pub skill_points: i16,
    pub atk_range: i16,
}

#[derive(Debug, Clone)]
pub struct PetProperty {
    pub name: String,
    pub renamed: bool,
    pub level: i16,
    pub hunger: i16,
    pub intimacy: i16,
    pub accessory: u16,
    pub job: i16,
}

#[derive(Debug, Clone)]
pub struct MercenaryInfo {
    pub gid: u32,
    pub name: String,
    pub level: i16,
    pub atk: i16,
    pub matk: i16,
    pub hit: i16,
    pub critical: i16,
    pub def: i16,
    pub mdef: i16,
    pub flee: i16,
    pub aspd: i16,
    pub atk_range: i16,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub expire_date: i32,
    pub faith: i16,
    pub calls: i32,
    pub kills: i32,
}

#[derive(Debug, Clone)]
pub struct RefineItemRow {
    pub index: i16,
    pub item_id: u16,
    pub refine: u8,
    pub cards: [u16; 4],
}

#[derive(Debug, Clone)]
pub struct VendorItem {
    pub index: i16,
    pub item_id: u16,
    pub amount: i16,
    pub price: i32,
    pub refine: u8,
    pub is_identified: bool,
    pub is_damaged: bool,
    pub item_type: u8,
}

#[derive(Debug, Clone)]
pub struct PartyMemberData {
    pub aid: u32,
    pub name: String,
    pub map: String,
    pub leader: bool,
    pub online: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GuildMemberAppearance {
    pub sex: i16,
    pub head: i16,
    pub head_palette: i16,
}

#[derive(Debug, Clone)]
pub struct FriendData {
    pub aid: u32,
    pub gid: u32,
    pub name: String,
    pub online: bool,
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

impl SkillInfo {
    pub fn icon_path(&self) -> String {
        format!(
            "data/texture/유저인터페이스/item/{}.bmp",
            self.name.to_lowercase()
        )
    }
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub ip: u32,
    pub port: i16,
    pub name: String,
    pub user_count: u16,
}

#[derive(Debug, Clone)]
pub struct AccessibleMap {
    pub status: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub gid: u32,
    pub name: String,
    pub class: u16,
    pub base_level: u16,
    pub base_exp: u32,
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
    pub effect_state: i32,
    pub zeny: i32,
}
