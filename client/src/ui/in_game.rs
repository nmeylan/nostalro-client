use crate::game_state::{GameState, TOKEN_OF_SIEGFRIED};
use crate::ui::escape::{EscapeGame, modal_owns_keyboard, route_escape};
use crate::ui::windows::{Dispatch, REGISTRY, Windows};
use ragnarok_game::boss_info::BossMark;
use ragnarok_game::cursor::RenderEntry;
use ragnarok_game::event::GameEvent;
use ragnarok_game::guild::Guild;
use ragnarok_game::minimap_mark::MinimapMarks;
use ragnarok_game::party::Party;
use ragnarok_game::quest::QuestMarker;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui_component::game::drop_quantity_dialog::DropQuantityDialog;
use ragnarok_ui_component::game::hotkey_bar::HOTKEY_BAR_WINDOW_ID;
use ragnarok_ui_component::game::inventory_window::INV_WINDOW_ID;
use ragnarok_ui_component::game::levelup_notification_window::LevelUpClick;
use ragnarok_ui_component::game::minimap_window::{MarkerType, MinimapMarker, quest_marker_color};
use ragnarok_ui_component::{BuildCtx, InGameWindow, Window};
use std::collections::HashMap;

pub fn build_in_game_ui(
    game: &mut GameState,
    windows: &mut Windows,
    ui: &mut UiFrame,
    texture_size_fn: &dyn Fn(&str) -> Option<(u32, u32)>,
    _render_list: &[RenderEntry],
) -> Vec<GameEvent> {
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

    // Snapshot before the Escape router, which can dismiss the dialog itself.
    let had_disconnect_dialog =
        game.session.disconnect_dialog_shown && windows.confirm_dialog.state.is_some();

    events.extend(route_escape(
        ui,
        windows,
        EscapeGame {
            pending_casts: &mut game.pending_casts,
            capture_targeting: &mut game.companions.capture_targeting,
            pet_roulette: &mut game.companions.pet_roulette,
            combat: &mut game.combat,
        },
        &mut ctx,
    ));

    if modal_owns_keyboard(windows, &ctx) {
        ui.block_keyboard();
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
    game.minimap_marks.prune(ui.elapsed_secs);
    windows.minimap_window.entity_markers = collect_minimap_markers(
        ctx.party,
        ctx.guild,
        &game.quest_markers,
        &game.minimap_marks,
        game.boss_mark.as_ref(),
        game.session.current_map.as_deref(),
        ctx.local_aid,
    );
    events.extend(windows.minimap_window.build(ui, &mut ctx));

    events.extend(windows.status_icon_bar.build(ui, &mut ctx));

    events.extend(windows.npc_dialog.build(ui, &mut ctx));
    events.extend(windows.npc_shop.build(ui, &mut ctx));
    events.extend(windows.warp_list_window.build(ui));
    events.extend(windows.item_list_selection_window.build(ui));
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

const BOSS_MARK_COLOR: [f32; 3] = [1.0, 0.2, 0.2];

/// What the minimap marks: party members and same-map guild members, the
/// server's own mark channel (quest markers and `ZC_COMPASS` viewpoints) and the
/// Convex Mirror's MVP. NPCs and portals are deliberately absent — the original
/// never marked them.
fn collect_minimap_markers(
    party: Option<&Party>,
    guild: Option<&Guild>,
    quest_markers: &HashMap<u32, QuestMarker>,
    marks: &MinimapMarks,
    boss_mark: Option<&BossMark>,
    current_map: Option<&str>,
    local_aid: u32,
) -> Vec<MinimapMarker> {
    let mut markers = Vec::new();
    let current_map = current_map.map(ragnarok_game::map_key);
    if let Some(party) = party {
        for member in &party.members {
            if member.aid == local_aid
                || !member.online
                || !member.has_live_position
                || current_map.as_deref() != Some(ragnarok_game::map_key(&member.map).as_str())
            {
                continue;
            }
            markers.push(MinimapMarker {
                x: member.x as f32,
                y: member.y as f32,
                marker_type: MarkerType::PartyMember {
                    leader: member.leader,
                },
                name: Some(member.name.clone()),
            });
        }
    }
    if let Some(guild) = guild {
        for member in &guild.members {
            if member.aid == local_aid || !member.has_live_position {
                continue;
            }
            markers.push(MinimapMarker {
                x: member.x as f32,
                y: member.y as f32,
                marker_type: MarkerType::GuildMember,
                name: None,
            });
        }
    }
    for marker in quest_markers.values() {
        markers.push(MinimapMarker {
            x: marker.x as f32,
            y: marker.y as f32,
            marker_type: MarkerType::Mark(quest_marker_color(marker.color)),
            name: None,
        });
    }
    for mark in marks.iter() {
        markers.push(MinimapMarker {
            x: mark.x as f32,
            y: mark.y as f32,
            marker_type: MarkerType::Mark(mark.rgb()),
            name: None,
        });
    }
    if let Some(boss) = boss_mark {
        markers.push(MinimapMarker {
            x: boss.x as f32,
            y: boss.y as f32,
            marker_type: MarkerType::Mark(BOSS_MARK_COLOR),
            name: Some(boss.name.clone()),
        });
    }
    markers
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
                // On-screen beats waiting for the next position packet; a member
                // who is on our map but out of view keeps the packet's answer.
                m.has_live_position = true;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::minimap_mark::MarkAction;
    use ragnarok_game::party::PartyMember;

    fn member(aid: u32, name: &str, map: &str, leader: bool, live: bool) -> PartyMember {
        PartyMember {
            aid,
            name: name.to_string(),
            map: map.to_string(),
            leader,
            online: true,
            hp: None,
            max_hp: None,
            x: 100,
            y: 120,
            has_live_position: live,
        }
    }

    #[test]
    fn only_same_map_party_members_guilds_and_server_marks_are_marked() {
        let mut party = Party::new("Adventurers".to_string());
        party.members = vec![
            member(1, "Me", "prontera.gat", false, true),
            member(2, "Leader", "prontera.gat", true, true),
            member(3, "Lidia", "prontera.gat", false, true),
            // Left our map: the server cleared their position.
            member(4, "Garm", "prontera.gat", false, false),
            member(5, "Sohee", "payon.gat", false, true),
        ];
        let mut offline = member(6, "Afk", "prontera.gat", false, true);
        offline.online = false;
        party.members.push(offline);

        let mut marks = MinimapMarks::default();
        marks.apply(0, MarkAction::Show, 134, 221, 0xFF0000, 0.0);

        let mut quest_markers = HashMap::new();
        quest_markers.insert(
            900,
            QuestMarker {
                x: 50,
                y: 60,
                effect: 1,
                color: 2,
            },
        );

        let boss = BossMark {
            x: 100,
            y: 100,
            name: "Baphomet".to_string(),
        };
        let markers = collect_minimap_markers(
            Some(&party),
            None,
            &quest_markers,
            &marks,
            Some(&boss),
            Some("prontera"),
            1,
        );

        let party_names: Vec<(&str, bool)> = markers
            .iter()
            .filter_map(|m| match (m.marker_type, &m.name) {
                (MarkerType::PartyMember { leader }, Some(name)) => Some((name.as_str(), leader)),
                _ => None,
            })
            .collect();
        assert_eq!(party_names, vec![("Leader", true), ("Lidia", false)]);

        let mark_colors: Vec<[f32; 3]> = markers
            .iter()
            .filter_map(|m| match m.marker_type {
                MarkerType::Mark(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(
            mark_colors,
            vec![quest_marker_color(2), [1.0, 0.0, 0.0], BOSS_MARK_COLOR],
            "quest marker, the viewpoint mark and the MVP, all on the mark channel"
        );
        assert_eq!(markers.len(), 5, "no NPC or portal markers");
    }
}
