//! The single Escape router. Escape has exactly one effect per press: it walks a
//! fixed priority chain, front-most first, and stops at the first claimant.

use crate::game_state::{CombatState, PendingCasts};
use crate::ui::windows::{Dispatch, REGISTRY, Windows};
use ragnarok_game::event::GameEvent;
use ragnarok_game::pet::PetRoulette;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui_component::game::book_window::BOOK_WINDOW_ID;
use ragnarok_ui_component::game::cart_select_window::CART_SELECT_WINDOW_ID;
use ragnarok_ui_component::game::cart_window::CART_WINDOW_ID;
use ragnarok_ui_component::game::chat_room_create_window::CHAT_ROOM_CREATE_WINDOW_ID;
use ragnarok_ui_component::game::chat_room_member_window::CHAT_ROOM_MEMBER_WINDOW_ID;
use ragnarok_ui_component::game::companion_ai_config_window::COMPANION_AI_CONFIG_WINDOW_ID;
use ragnarok_ui_component::game::emblem_picker_window::EMBLEM_PICKER_WINDOW_ID;
use ragnarok_ui_component::game::emotion_window::EMOTION_WINDOW_ID;
use ragnarok_ui_component::game::equipment_window::EQ_WINDOW_ID;
use ragnarok_ui_component::game::graphic_options::GRAPHIC_OPTIONS_WINDOW_ID;
use ragnarok_ui_component::game::guild_window::GUILD_WINDOW_ID;
use ragnarok_ui_component::game::homun_skill_window::HOMUN_SKILL_WINDOW_ID;
use ragnarok_ui_component::game::homun_window::HOMUN_WINDOW_ID;
use ragnarok_ui_component::game::hotkey_config_window::HOTKEY_CONFIG_WINDOW_ID;
use ragnarok_ui_component::game::inventory_window::INV_WINDOW_ID;
use ragnarok_ui_component::game::mailbox_window::MAILBOX_WINDOW_ID;
use ragnarok_ui_component::game::make_item_window::MAKE_ITEM_WINDOW_ID;
use ragnarok_ui_component::game::mercenary_skill_window::MERCENARY_SKILL_WINDOW_ID;
use ragnarok_ui_component::game::mercenary_window::MERCENARY_WINDOW_ID;
use ragnarok_ui_component::game::monster_info_window::MONSTER_INFO_WINDOW_ID;
use ragnarok_ui_component::game::my_shop_window::MY_SHOP_WINDOW_ID;
use ragnarok_ui_component::game::party_friends_window::PARTY_FRIENDS_WINDOW_ID;
use ragnarok_ui_component::game::party_helper_window::PARTY_HELPER_WINDOW_ID;
use ragnarok_ui_component::game::pet_window::PET_WINDOW_ID;
use ragnarok_ui_component::game::quest_window::{QUEST_DETAIL_WINDOW_ID, QUEST_WINDOW_ID};
use ragnarok_ui_component::game::read_mail_window::READ_MAIL_WINDOW_ID;
use ragnarok_ui_component::game::shortcut_list_window::SHORTCUT_LIST_WINDOW_ID;
use ragnarok_ui_component::game::skill_tree_window::SKILL_WINDOW_ID;
use ragnarok_ui_component::game::sound_options::SOUND_OPTIONS_WINDOW_ID;
use ragnarok_ui_component::game::status_window::STATUS_WINDOW_ID;
use ragnarok_ui_component::game::storage_window::STORAGE_WINDOW_ID;
use ragnarok_ui_component::game::trade_window::TRADE_WINDOW_ID;
use ragnarok_ui_component::game::vending_setup_window::VENDING_SETUP_WINDOW_ID;
use ragnarok_ui_component::game::vending_shop_window::VENDING_SHOP_WINDOW_ID;
use ragnarok_ui_component::game::world_map_window::WORLD_MAP_WINDOW_ID;
use ragnarok_ui_component::{BuildCtx, InGameWindow};

/// How long after the chat input loses focus Escape stays swallowed, so the
/// press that dropped the focus cannot also dismiss a window.
const CHAT_BLUR_GUARD_SECS: f32 = 0.2;

/// Names accepted by `custom.window.exclude_close_via_esc`. Only the windows the
/// player opens are listable: server-driven modals (NPC dialogs, shops, warp
/// lists) must stay answerable with Escape.
pub const ESC_WINDOW_NAMES: &[(&str, WidgetId)] = &[
    ("Inventory", INV_WINDOW_ID),
    ("Equip", EQ_WINDOW_ID),
    ("Stats", STATUS_WINDOW_ID),
    ("Skills", SKILL_WINDOW_ID),
    ("Cart", CART_WINDOW_ID),
    ("Cart Select", CART_SELECT_WINDOW_ID),
    ("Storage", STORAGE_WINDOW_ID),
    ("Trade", TRADE_WINDOW_ID),
    ("Mail", MAILBOX_WINDOW_ID),
    ("Read Mail", READ_MAIL_WINDOW_ID),
    ("Party", PARTY_FRIENDS_WINDOW_ID),
    ("Party Setup", PARTY_HELPER_WINDOW_ID),
    ("Guild", GUILD_WINDOW_ID),
    ("Guild Emblem", EMBLEM_PICKER_WINDOW_ID),
    ("Quest", QUEST_WINDOW_ID),
    ("Quest Detail", QUEST_DETAIL_WINDOW_ID),
    ("World Map", WORLD_MAP_WINDOW_ID),
    ("Emotion", EMOTION_WINDOW_ID),
    ("Shortcuts", SHORTCUT_LIST_WINDOW_ID),
    ("Chat Room Create", CHAT_ROOM_CREATE_WINDOW_ID),
    ("Chat Room", CHAT_ROOM_MEMBER_WINDOW_ID),
    ("Homunculus", HOMUN_WINDOW_ID),
    ("Homunculus Skills", HOMUN_SKILL_WINDOW_ID),
    ("Mercenary", MERCENARY_WINDOW_ID),
    ("Mercenary Skills", MERCENARY_SKILL_WINDOW_ID),
    ("Pet", PET_WINDOW_ID),
    ("Companion AI", COMPANION_AI_CONFIG_WINDOW_ID),
    ("Book", BOOK_WINDOW_ID),
    ("Monster Info", MONSTER_INFO_WINDOW_ID),
    ("Sound", SOUND_OPTIONS_WINDOW_ID),
    ("Graphics", GRAPHIC_OPTIONS_WINDOW_ID),
    ("Hotkeys", HOTKEY_CONFIG_WINDOW_ID),
    ("Make Item", MAKE_ITEM_WINDOW_ID),
    ("Vending Setup", VENDING_SETUP_WINDOW_ID),
    ("Vending Shop", VENDING_SHOP_WINDOW_ID),
    ("My Shop", MY_SHOP_WINDOW_ID),
];

fn name_key(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

#[derive(Default)]
pub struct EscapeState {
    chat_blur_at: Option<f32>,
    excluded: Vec<WidgetId>,
}

impl EscapeState {
    /// Resolves configured window names once; unknown names are reported and
    /// dropped rather than failing the launch.
    pub fn set_excluded(&mut self, names: &[String]) {
        self.excluded.clear();
        for name in names {
            let key = name_key(name);
            match ESC_WINDOW_NAMES
                .iter()
                .find(|(known, _)| name_key(known) == key)
            {
                Some((_, id)) => self.excluded.push(*id),
                None => tracing::warn!(
                    "custom.window.exclude_close_via_esc: unknown window {name:?}, ignoring"
                ),
            }
        }
    }
}

/// The non-window state Escape can act on, borrowed field-by-field because
/// `BuildCtx` already holds the rest of `GameState`.
pub struct EscapeGame<'a> {
    pub pending_casts: &'a mut PendingCasts,
    pub capture_targeting: &'a mut bool,
    pub pet_roulette: &'a mut Option<PetRoulette>,
    pub combat: &'a mut CombatState,
}

pub fn route_escape(
    ui: &mut UiFrame,
    windows: &mut Windows,
    game: EscapeGame,
    ctx: &mut BuildCtx,
) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let now = ui.elapsed_secs;

    if windows.chat_window.is_active() {
        if ui.take_escape() {
            windows.chat_window.cancel_input();
            windows.escape.chat_blur_at = Some(now);
        }
        return events;
    }
    if let Some(blurred_at) = windows.escape.chat_blur_at {
        if now - blurred_at < CHAT_BLUR_GUARD_SECS {
            ui.take_escape();
            return events;
        }
        windows.escape.chat_blur_at = None;
    }

    if !ui.take_escape() {
        return events;
    }

    if game.pending_casts.pending_skill_target.is_some() {
        game.pending_casts.pending_skill_target = None;
        game.pending_casts.pending_skill_id = None;
        game.pending_casts.pending_skill_level = None;
        game.pending_casts.pending_skill_unit_cast = None;
        return events;
    }
    if *game.capture_targeting || game.pet_roulette.is_some() {
        *game.capture_targeting = false;
        *game.pet_roulette = None;
        return events;
    }
    if game.pending_casts.marriage_targeting {
        game.pending_casts.marriage_targeting = false;
        return events;
    }

    if windows.context_menu.is_open() {
        windows.context_menu.close();
        return events;
    }
    if windows.card_insert_dialog.take().is_some() {
        return events;
    }
    if windows.skill_talkbox_dialog.take().is_some() {
        return events;
    }
    if windows.guild_expel_dialog.take().is_some() {
        return events;
    }
    if windows.drop_quantity_dialog.take().is_some() {
        return events;
    }
    if windows.confirm_dialog.escape() {
        return events;
    }
    if let Some(window_events) = offer(&mut windows.item_info_window, ctx) {
        events.extend(window_events);
        return events;
    }
    if let Some(window_events) = offer(&mut windows.system_menu, ctx) {
        events.extend(window_events);
        return events;
    }
    if windows.item_list_selection_window.is_open() {
        windows.item_list_selection_window.cancel(&mut events);
        return events;
    }
    if windows.warp_list_window.is_open() {
        windows.warp_list_window.cancel(&mut events);
        return events;
    }
    if let Some(window_events) = offer(&mut windows.npc_shop, ctx) {
        events.extend(window_events);
        return events;
    }
    if let Some(window_events) = offer(&mut windows.npc_dialog, ctx) {
        events.extend(window_events);
        return events;
    }

    for win_id in ui.get_z_order().into_iter().rev() {
        if windows.escape.excluded.contains(&win_id) {
            continue;
        }
        let Some((_, Dispatch::Trait(accessor))) = REGISTRY.iter().find(|(id, _)| *id == win_id)
        else {
            continue;
        };
        let window = accessor(windows);
        if window.wants_escape(ctx) {
            events.extend(window.on_escape(ctx));
            return events;
        }
    }

    if game.combat.attack_target_id.take().is_some() {
        game.combat.attack_is_locked = false;
        return events;
    }

    if !windows.system_menu.dead {
        windows.system_menu.open = true;
    }
    events
}

/// Whether a modal (or a window's nested input dialog) is waiting for an Enter.
/// Run after [`route_escape`] and before the window builds: it drives
/// `UiFrame::block_keyboard`, so the press that answers the modal cannot also
/// open the chat line.
pub fn modal_owns_keyboard(windows: &mut Windows, ctx: &BuildCtx) -> bool {
    if windows.confirm_dialog.state.is_some()
        || windows.warp_list_window.is_open()
        || windows.item_list_selection_window.is_open()
    {
        return true;
    }
    if windows
        .drop_quantity_dialog
        .as_ref()
        .is_some_and(|d| d.owns_keyboard(ctx))
        || windows
            .guild_expel_dialog
            .as_ref()
            .is_some_and(|d| d.owns_keyboard(ctx))
        || windows
            .skill_talkbox_dialog
            .as_ref()
            .is_some_and(|d| d.owns_keyboard(ctx))
        || windows
            .card_insert_dialog
            .as_ref()
            .is_some_and(|d| d.owns_keyboard(ctx))
    {
        return true;
    }
    if windows.npc_dialog.owns_keyboard(ctx)
        || windows.npc_shop.owns_keyboard(ctx)
        || windows.system_menu.owns_keyboard(ctx)
    {
        return true;
    }
    REGISTRY.iter().any(|(_, dispatch)| match dispatch {
        Dispatch::Trait(accessor) => accessor(windows).owns_keyboard(ctx),
        Dispatch::VendingAvailable => false,
    })
}

fn offer<W: InGameWindow>(window: &mut W, ctx: &mut BuildCtx) -> Option<Vec<GameEvent>> {
    if window.wants_escape(ctx) {
        Some(window.on_escape(ctx))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_ai::config::CompanionAiConfig;
    use ragnarok_game::character::Character;
    use ragnarok_game::cursor::PendingSkillTarget;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_game::friends::FriendList;
    use ragnarok_game::pet::PetState;
    use ragnarok_game::quest::QuestLog;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use std::collections::HashMap;

    struct Harness {
        character: Character,
        data: DataTable,
        friends: FriendList,
        quest_log: QuestLog,
        pet: PetState,
        companion_ai: CompanionAiConfig,
        pending_casts: PendingCasts,
        capture_targeting: bool,
        pet_roulette: Option<PetRoulette>,
        combat: CombatState,
    }

    impl Harness {
        fn new() -> Self {
            Self {
                character: Character::new(),
                data: DataTable::new(),
                friends: FriendList::default(),
                quest_log: QuestLog::default(),
                pet: PetState::default(),
                companion_ai: CompanionAiConfig::default(),
                pending_casts: PendingCasts::default(),
                capture_targeting: false,
                pet_roulette: None,
                combat: CombatState::new(),
            }
        }

        fn press_escape(&mut self, windows: &mut Windows, state: &mut StateCache) {
            let atlas = FontAtlas::from_embedded(14.0, 1.0);
            let mut ui_ctx = UiContext::new(800.0, 600.0);
            ui_ctx.key_escape = true;
            let positions = HashMap::new();
            let mut ui = UiFrame::new(&ui_ctx, &atlas, state, 0.0, false, None, &positions);
            let mut ctx = BuildCtx {
                character: &mut self.character,
                data: &self.data,
                party: None,
                friends: &self.friends,
                guild: None,
                quest_log: &self.quest_log,
                homunculus: None,
                mercenary: None,
                pet: &self.pet,
                companion_ai: &mut self.companion_ai,
                job_class: 0,
                local_aid: 0,
                local_gid: 0,
            };
            route_escape(
                &mut ui,
                windows,
                EscapeGame {
                    pending_casts: &mut self.pending_casts,
                    capture_targeting: &mut self.capture_targeting,
                    pet_roulette: &mut self.pet_roulette,
                    combat: &mut self.combat,
                },
                &mut ctx,
            );
        }

        /// Runs one Enter press through the same pre-build pass the frame does,
        /// then lets the chat window read it. Returns whether chat took it.
        fn press_enter_reaches_chat(
            &mut self,
            windows: &mut Windows,
            state: &mut StateCache,
        ) -> bool {
            let atlas = FontAtlas::from_embedded(14.0, 1.0);
            let mut ui_ctx = UiContext::new(800.0, 600.0);
            ui_ctx.key_enter = true;
            let positions = HashMap::new();
            let mut ui = UiFrame::new(&ui_ctx, &atlas, state, 0.0, false, None, &positions);
            let ctx = BuildCtx {
                character: &mut self.character,
                data: &self.data,
                party: None,
                friends: &self.friends,
                guild: None,
                quest_log: &self.quest_log,
                homunculus: None,
                mercenary: None,
                pet: &self.pet,
                companion_ai: &mut self.companion_ai,
                job_class: 0,
                local_aid: 0,
                local_gid: 0,
            };
            if modal_owns_keyboard(windows, &ctx) {
                ui.block_keyboard();
            }
            ui.enter_pressed()
        }
    }

    fn open_three(windows: &mut Windows, state: &mut StateCache) {
        windows.equipment_window.open = true;
        windows.status_window.open();
        windows.guild_window.open();

        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let ui_ctx = UiContext::new(800.0, 600.0);
        let positions = HashMap::new();
        let mut ui = UiFrame::new(&ui_ctx, &atlas, state, 0.0, false, None, &positions);
        ui.ensure_in_z_order(EQ_WINDOW_ID);
        ui.ensure_in_z_order(STATUS_WINDOW_ID);
        ui.ensure_in_z_order(GUILD_WINDOW_ID);
    }

    #[test]
    fn each_press_closes_only_the_front_most_window() {
        let mut windows = Windows::new();
        let mut state = StateCache::new();
        let mut harness = Harness::new();
        open_three(&mut windows, &mut state);

        harness.press_escape(&mut windows, &mut state);
        assert!(!windows.guild_window.is_open());
        assert!(windows.status_window.is_visible());
        assert!(windows.equipment_window.open);

        harness.press_escape(&mut windows, &mut state);
        assert!(!windows.status_window.is_visible());
        assert!(windows.equipment_window.open);

        harness.press_escape(&mut windows, &mut state);
        assert!(!windows.equipment_window.open);
        assert!(!windows.system_menu.open);

        harness.press_escape(&mut windows, &mut state);
        assert!(windows.system_menu.open);
    }

    #[test]
    fn an_excluded_window_is_skipped_and_the_next_one_closes() {
        let mut windows = Windows::new();
        let mut state = StateCache::new();
        let mut harness = Harness::new();
        windows
            .escape
            .set_excluded(&["stats".into(), "Equip".into(), "no such window".into()]);
        open_three(&mut windows, &mut state);

        harness.press_escape(&mut windows, &mut state);
        assert!(!windows.guild_window.is_open());

        harness.press_escape(&mut windows, &mut state);
        assert!(windows.status_window.is_visible());
        assert!(windows.equipment_window.open);
        assert!(windows.system_menu.open);
    }

    #[test]
    fn a_modal_awaiting_ok_keeps_enter_from_the_chat_line() {
        let mut windows = Windows::new();
        let mut state = StateCache::new();
        let mut harness = Harness::new();

        assert!(harness.press_enter_reaches_chat(&mut windows, &mut state));

        windows.confirm_dialog.show_confirm("Are you sure?");
        assert!(!harness.press_enter_reaches_chat(&mut windows, &mut state));

        windows.confirm_dialog.dismiss();
        windows.warp_list_window.open(26, vec!["Random".into()]);
        assert!(!harness.press_enter_reaches_chat(&mut windows, &mut state));
    }

    #[test]
    fn a_pending_cast_swallows_the_press_before_any_window() {
        let mut windows = Windows::new();
        let mut state = StateCache::new();
        let mut harness = Harness::new();
        open_three(&mut windows, &mut state);
        harness.pending_casts.pending_skill_target = Some(PendingSkillTarget::Ground {
            skill_id: 27,
            level: 3,
        });

        harness.press_escape(&mut windows, &mut state);

        assert!(harness.pending_casts.pending_skill_target.is_none());
        assert!(windows.guild_window.is_open());
    }
}
