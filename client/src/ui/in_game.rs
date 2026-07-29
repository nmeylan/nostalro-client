use crate::game_state::{GameState, TOKEN_OF_SIEGFRIED};
use crate::ui::windows::{Dispatch, REGISTRY, Windows};
use ragnarok_game::cursor::RenderEntry;
use ragnarok_game::entity::EntityCategory;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui_component::game::drop_quantity_dialog::DropQuantityDialog;
use ragnarok_ui_component::game::hotkey_bar::HOTKEY_BAR_WINDOW_ID;
use ragnarok_ui_component::game::inventory_window::INV_WINDOW_ID;
use ragnarok_ui_component::game::levelup_notification_window::LevelUpClick;
use ragnarok_ui_component::game::minimap_window::{MarkerType, MinimapMarker};
use ragnarok_ui_component::{BuildCtx, InGameWindow, Window};

pub fn build_in_game_ui(
    game: &mut GameState,
    windows: &mut Windows,
    ui: &mut UiFrame,
    texture_size_fn: &dyn Fn(&str) -> Option<(u32, u32)>,
    _render_list: &[RenderEntry],
) -> Vec<GameEvent> {
    let chat_was_active = windows.chat_window.is_active();
    let mut events = Vec::new();

    windows.npc_shop.setup_modal(ui);

    // The server never sends our own HP via party, and same-map members' HP only
    // arrives on change — so refresh rows from live state before `ctx` borrows
    // the party immutably.
    sync_party_live_state(game);

    let local_aid = game
        .session
        .login_session
        .as_ref()
        .map(|s| s.account_id)
        .unwrap_or(0);
    let local_gid = local_aid;
    let job_class = game.world.entities.player_job();

    let mut ctx = BuildCtx {
        character: &mut game.character,
        data: &game.data_table,
        party: game.party.as_ref(),
        friends: &game.friends,
        guild: game.guild.as_ref(),
        quest_log: &game.quest_log,
        homunculus: game.companions.homunculus.as_ref(),
        mercenary: game.companions.mercenary.as_ref(),
        pet: &game.companions.pet,
        companion_ai: &mut game.companions.companion_ai,
        job_class,
        local_aid,
        local_gid,
    };

    if windows.world_map_window.is_open() {
        windows.world_map_window.current_map = game.session.current_map.clone();
        if let Some(coords) = &game.session.map_coords {
            windows.world_map_window.map_width = coords.gat_width();
            windows.world_map_window.map_height = coords.gat_height();
        }
        if let Some(player) = game.world.entities.player() {
            windows.world_map_window.player_position = Some(player.movement.position());
            windows.world_map_window.player_direction = player.direction;
        }
    }

    let z_order = ui.get_z_order();
    ui.compute_hovered_window(&z_order);
    for &win_id in &z_order {
        dispatch_window(windows, win_id, ui, &mut ctx, &mut events);
    }
    for &(win_id, _) in REGISTRY {
        if !z_order.contains(&win_id) {
            dispatch_window(windows, win_id, ui, &mut ctx, &mut events);
        }
    }

    let deposit_intents: Vec<u16> = events
        .iter()
        .filter_map(|e| match e {
            GameEvent::RequestDepositItem { index } => Some(*index),
            _ => None,
        })
        .collect();
    if !deposit_intents.is_empty() {
        events.retain(|e| !matches!(e, GameEvent::RequestDepositItem { .. }));
        for index in deposit_intents {
            let deposit = windows
                .storage_window
                .begin_deposit_body(ctx.character, index);
            events.extend(deposit);
        }
    }

    windows.hotkey_bar.chat_is_active = windows.chat_window.is_active();
    windows.hotkey_bar.companion_skills.clear();
    if let Some(m) = ctx.mercenary {
        windows
            .hotkey_bar
            .companion_skills
            .extend(m.skills.iter().cloned());
    }
    if let Some(h) = ctx.homunculus {
        windows
            .hotkey_bar
            .companion_skills
            .extend(h.skills.iter().cloned());
    }
    events.extend(windows.hotkey_bar.build(ui, &mut ctx));

    if let Some(player) = game.world.entities.player() {
        windows.minimap_window.player_position = Some(player.movement.position());
        windows.minimap_window.player_direction = player.direction;
    }
    if let Some(coords) = &game.session.map_coords {
        windows.minimap_window.map_width = coords.gat_width();
        windows.minimap_window.map_height = coords.gat_height();
    }
    windows.minimap_window.map_name = game.session.current_map.clone();
    windows.minimap_window.entity_markers.clear();
    for entity in game.world.entities.iter() {
        if Some(entity.id) == game.world.entities.player_id() {
            continue;
        }
        let marker_type = match entity.category() {
            EntityCategory::Npc => MarkerType::Npc,
            EntityCategory::WarpPoint => MarkerType::WarpPortal,
            _ => continue,
        };
        let (ex, ey) = entity.movement.position();
        windows.minimap_window.entity_markers.push(MinimapMarker {
            x: ex,
            y: ey,
            marker_type,
        });
    }
    if let Some(party) = ctx.party {
        let current_map = game.session.current_map.as_deref().unwrap_or("");
        for member in &party.members {
            if member.aid == ctx.local_aid || !member.online || member.map != current_map {
                continue;
            }
            windows.minimap_window.entity_markers.push(MinimapMarker {
                x: member.x as f32,
                y: member.y as f32,
                marker_type: MarkerType::PartyMember,
            });
        }
    }
    if let Some(guild) = ctx.guild {
        for member in &guild.members {
            if member.aid == ctx.local_aid || !member.has_live_position {
                continue;
            }
            windows.minimap_window.entity_markers.push(MinimapMarker {
                x: member.x as f32,
                y: member.y as f32,
                marker_type: MarkerType::GuildMember,
            });
        }
    }
    for marker in game.quest_markers.values() {
        windows.minimap_window.entity_markers.push(MinimapMarker {
            x: marker.x as f32,
            y: marker.y as f32,
            marker_type: MarkerType::Quest(marker.color),
        });
    }
    events.extend(windows.minimap_window.build(ui, &mut ctx));

    events.extend(windows.status_icon_bar.build(ui, &mut ctx));

    let npc_dialog_open = windows.npc_dialog.dialog.is_open();
    events.extend(windows.npc_dialog.build(ui, &mut ctx));
    let shop_open = windows.npc_shop.shop.is_open();
    events.extend(windows.npc_shop.build(ui, &mut ctx));
    let warp_list_open = windows.warp_list_window.is_open();
    events.extend(windows.warp_list_window.build(ui));
    let item_list_open = windows.item_list_selection_window.is_open();
    events.extend(windows.item_list_selection_window.build(ui));
    let mut allow_escape =
        !chat_was_active && !npc_dialog_open && !shop_open && !warp_list_open && !item_list_open;
    if allow_escape && ui.ctx.key_escape && game.pending_casts.pending_skill_target.is_some() {
        game.pending_casts.pending_skill_target = None;
        allow_escape = false;
    }
    if allow_escape
        && ui.ctx.key_escape
        && (game.companions.capture_targeting || game.companions.pet_roulette.is_some())
    {
        game.companions.capture_targeting = false;
        game.companions.pet_roulette = None;
        allow_escape = false;
    }
    windows.system_menu.allow_escape_toggle = allow_escape;
    windows.system_menu.can_resurrect = windows.system_menu.dead
        && !game.session.map_properties.enable_pk()
        && !game.session.map_properties.is_siege()
        && ctx
            .character
            .inventory
            .all_items()
            .iter()
            .any(|item| item.item_id == TOKEN_OF_SIEGFRIED && item.count > 0);
    events.extend(windows.system_menu.build(ui, &mut ctx));
    events.extend(windows.item_info_window.build(ui, &mut ctx));
    events.extend(InGameWindow::build(
        &mut windows.item_pickup_notification,
        ui,
        &mut ctx,
    ));

    let had_disconnect_dialog =
        game.session.disconnect_dialog_shown && windows.confirm_dialog.state.is_some();
    windows.confirm_dialog.build(ui);
    if had_disconnect_dialog && windows.confirm_dialog.state.is_none() {
        game.session.pending_disconnect_exit = true;
        game.session.disconnect_dialog_shown = false;
    }

    if let Some(result) = windows.confirm_dialog.take_result()
        && let Some(event) = game.pending_confirms.dispatch(result)
    {
        events.push(event);
    }

    events.extend(windows.context_menu.build(ui));

    events.extend(windows.map_missing_window.build(ui));

    match windows.levelup_notification.build(ui) {
        LevelUpClick::Base => windows.status_window.open(),
        LevelUpClick::Job => ctx.character.skills.open(),
        LevelUpClick::None => {}
    }

    ui.flush_tooltips();

    if let Some(cancelled) = ui.draw_drag_icon() {
        if cancelled.source_id == HOTKEY_BAR_WINDOW_ID {
            if ctx.character.hotkeys.get_slot(cancelled.item_index)
                != ragnarok_game::hotkey::HotkeySlotContent::Empty
            {
                ctx.character.hotkeys.clear_slot(cancelled.item_index);
                events.push(GameEvent::RequestHotkeyChange {
                    index: cancelled.item_index as u16,
                    is_skill: false,
                    id: 0,
                    count: 0,
                });
            }
        } else if cancelled.source_id == INV_WINDOW_ID && ui.hovered_window().is_none() {
            if game.combat.waiting_item_throw_ack {
            } else if windows.equipment_window.is_visible() {
                windows
                    .chat_window
                    .add_system("Please close the Equipment window.".to_string());
            } else if let Some(item) = ctx
                .character
                .inventory
                .get_item(cancelled.item_index as u16)
            {
                if item.count > 1 {
                    let mut dialog = DropQuantityDialog::new(item.index, item.count);
                    dialog.has_grf_textures = windows.drop_dialog_has_grf_textures;
                    if dialog.has_grf_textures {
                        dialog.set_texture_sizes(texture_size_fn);
                    }
                    windows.drop_quantity_dialog = Some(dialog);
                } else {
                    events.push(GameEvent::RequestDropItem {
                        index: item.index,
                        count: 1,
                    });
                    game.combat.waiting_item_throw_ack = true;
                }
            }
        }
    }

    if run_transient_dialog(
        &mut windows.drop_quantity_dialog,
        ui,
        &mut ctx,
        &mut events,
        |e| matches!(e, GameEvent::RequestDropItem { .. }),
    ) {
        game.combat.waiting_item_throw_ack = true;
    }

    if let Some(dialog) = &mut windows.guild_expel_dialog {
        dialog.has_grf_textures = windows.drop_dialog_has_grf_textures;
        if dialog.has_grf_textures {
            dialog.set_texture_sizes(texture_size_fn);
        }
    }
    run_transient_dialog(
        &mut windows.guild_expel_dialog,
        ui,
        &mut ctx,
        &mut events,
        |e| matches!(e, GameEvent::ConfirmedGuildExpel { .. }),
    );

    if let Some(dialog) = &mut windows.skill_talkbox_dialog {
        dialog.has_grf_textures = windows.drop_dialog_has_grf_textures;
        if dialog.has_grf_textures {
            dialog.set_texture_sizes(texture_size_fn);
        }
    }
    run_transient_dialog(
        &mut windows.skill_talkbox_dialog,
        ui,
        &mut ctx,
        &mut events,
        |e| matches!(e, GameEvent::ConfirmedSkillTalkbox { .. }),
    );

    run_transient_dialog(
        &mut windows.card_insert_dialog,
        ui,
        &mut ctx,
        &mut events,
        |e| matches!(e, GameEvent::RequestCardInsert { .. }),
    );

    drop(ctx);
    update_broadcast_overlays(game, ui);

    events
}

/// Builds a transient dialog held in `slot`, clears the slot when the dialog
/// emits its confirm event or a `DialogClosed`, and appends its events (minus
/// `DialogClosed`) to `events`. Returns whether the confirm event fired, so the
/// caller can run any follow-up side effect. A no-op when the slot is empty.
fn run_transient_dialog<D: InGameWindow>(
    slot: &mut Option<D>,
    ui: &mut UiFrame,
    ctx: &mut BuildCtx,
    events: &mut Vec<GameEvent>,
    is_confirm: impl Fn(&GameEvent) -> bool,
) -> bool {
    let Some(dialog) = slot.as_mut() else {
        return false;
    };
    let dialog_events = InGameWindow::build(dialog, ui, ctx);
    let confirmed = dialog_events.iter().any(&is_confirm);
    if confirmed
        || dialog_events
            .iter()
            .any(|e| matches!(e, GameEvent::DialogClosed))
    {
        *slot = None;
    }
    events.extend(
        dialog_events
            .into_iter()
            .filter(|e| !matches!(e, GameEvent::DialogClosed)),
    );
    confirmed
}

fn sync_party_live_state(game: &mut GameState) {
    let local_aid = game
        .session
        .login_session
        .as_ref()
        .map(|s| s.account_id)
        .unwrap_or(0);
    if let Some(party) = &mut game.party {
        for m in &mut party.members {
            if m.aid == local_aid {
                m.hp = Some(game.character.hp);
                m.max_hp = Some(game.character.max_hp);
                if let Some(p) = game.world.entities.player() {
                    (m.x, m.y) = p.movement.cell_position();
                }
            } else if let Some(e) = game.world.entities.get(m.aid) {
                if let (Some(hp), Some(max_hp)) = (e.hp, e.max_hp) {
                    m.hp = Some(hp);
                    m.max_hp = Some(max_hp);
                }
                (m.x, m.y) = e.movement.cell_position();
            }
        }
    }
}

fn update_broadcast_overlays(game: &mut GameState, ui: &mut UiFrame) {
    let now = ui.elapsed_secs;
    let dt = game
        .broadcast
        .broadcast_last_elapsed
        .map_or(0.0, |last| (now - last).clamp(0.0, 0.1));
    game.broadcast.broadcast_last_elapsed = Some(now);

    game.broadcast.poptip.tick(dt);
    draw_broadcast_poptip(game, ui);
    update_broadcast_banner(game, ui, dt);
}

fn draw_broadcast_poptip(game: &mut GameState, ui: &mut UiFrame) {
    const BASE_Y: f32 = 90.0;
    if game.broadcast.poptip.is_empty() {
        return;
    }
    let center_x = ui.ctx.screen_width * 0.5;
    let line_h = ui.atlas.line_height + 4.0;
    const PAD: f32 = 4.0;
    for (index, (text, alpha)) in game.broadcast.poptip.iter().enumerate() {
        let width = ui.atlas.measure_text(text);
        let x = center_x - width * 0.5;
        let y = BASE_Y - index as f32 * line_h;

        let box_w = width + PAD * 2.0;
        let box_h = ui.atlas.line_height + PAD * 2.0;
        let box_x = x - PAD;
        let box_y = y - ui.atlas.line_height * 0.5 - PAD;
        let (bg_v, bg_i) = ragnarok_ui::draw::quad_vertices(
            box_x,
            box_y,
            box_w,
            box_h,
            [0.0, 0.0, 0.0, 0.8 * alpha],
        );
        ui.draw_calls.push(ragnarok_ui::draw::DrawCall {
            vertices: bg_v.to_vec(),
            indices: bg_i.to_vec(),
            texture: ragnarok_ui::draw::TextureRef::White,
        });

        ui.text(x, y, text, [1.0, 1.0, 1.0, alpha]);
    }
}

fn update_broadcast_banner(game: &mut GameState, ui: &mut UiFrame, dt: f32) {
    const BAR_Y: f32 = 40.0;
    const BAR_H: f32 = 24.0;
    const BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.7];
    const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    if !game.broadcast.banner.visible() {
        return;
    }
    game.broadcast.banner.tick(dt);

    let Some(render) = game.broadcast.banner.render() else {
        return;
    };
    let text_width = ui.atlas.measure_text(render.text);
    if game.broadcast.banner.current_scrolled_off(text_width) {
        game.broadcast.banner.advance();
        return;
    }

    let center_x = ui.ctx.screen_width * 0.5;
    let bar_left = center_x - render.half_width;
    let bar_right = center_x + render.half_width;

    let (bg_v, bg_i) =
        ragnarok_ui::draw::quad_vertices(bar_left, BAR_Y, render.half_width * 2.0, BAR_H, BG_COLOR);
    ui.draw_calls.push(ragnarok_ui::draw::DrawCall {
        vertices: bg_v.to_vec(),
        indices: bg_i.to_vec(),
        texture: ragnarok_ui::draw::TextureRef::White,
    });

    let text_x = bar_left + render.text_offset_x;
    let baseline_y = BAR_Y + (BAR_H + ui.atlas.ascent) * 0.5;
    let (tv, ti) = ragnarok_ui::draw::text_vertices_clipped(
        render.text,
        text_x,
        baseline_y,
        TEXT_COLOR,
        ui.atlas,
        bar_left,
        bar_right,
    );
    if !tv.is_empty() {
        ui.draw_calls.push(ragnarok_ui::draw::DrawCall {
            vertices: tv,
            indices: ti,
            texture: ragnarok_ui::draw::TextureRef::FontAtlas,
        });
    }
}

fn dispatch_window(
    windows: &mut Windows,
    win_id: WidgetId,
    ui: &mut UiFrame,
    ctx: &mut BuildCtx,
    events: &mut Vec<GameEvent>,
) {
    let Some((_, dispatch)) = REGISTRY.iter().find(|(id, _)| *id == win_id) else {
        return;
    };
    match dispatch {
        Dispatch::Trait(acc) => events.extend(acc(windows).build(ui, ctx)),
        Dispatch::VendingAvailable => {
            events.extend(windows.vending_setup_window.build_available(ui, ctx))
        }
    }
}
