pub mod connection;
pub mod handler;
pub mod helpers;
pub mod sender;
pub mod session;

use connection::{Connection, ConnectionError};
use handler::dispatch_packet;
pub use helpers::{encode_pos, ip_u32_to_string};
use ragnarok_game::event::GameEvent;
pub use sender::{
    build_ack_add_friend_packet, build_ack_exchange_item_packet, build_action_request_packet,
    build_active_quest_packet, build_add_exchange_item_packet, build_add_friend_packet,
    build_adopt_reply_packet, build_adopt_request_packet, build_agree_star_place_packet,
    build_alchemist_rank_packet, build_ally_guild, build_ans_join_guild,
    build_blacksmith_rank_packet, build_cancel_exchange_item_packet, build_cancel_lockon_packet,
    build_card_composition_list_packet, build_card_composition_packet, build_cartoff_packet,
    build_change_cart_packet, build_change_chat_owner_packet, build_change_chatroom_packet,
    build_change_direction_packet, build_change_party_exp_option_packet,
    build_change_party_leader_packet, build_char_enter_packet, build_chat_packet,
    build_close_store_packet, build_command_pet_packet, build_companion_attack_packet,
    build_companion_move_packet, build_companion_move_to_owner_packet,
    build_conclude_exchange_item_packet, build_config_packet, build_contact_npc_packet,
    build_create_chatroom_packet, build_delete_char_cancel_packet,
    build_delete_char_confirm_packet, build_delete_char_reserve_packet, build_delete_friend_packet,
    build_drop_item_packet, build_emotion_packet, build_equip_item_packet,
    build_exec_exchange_item_packet, build_exit_room_packet, build_expel_chat_member_packet,
    build_expel_party_member_packet, build_guild_chat_packet, build_guild_notice,
    build_homun_menu_packet, build_join_party_reply_packet, build_leave_party_packet,
    build_lesseffect_packet, build_login_packet, build_mail_add_item_packet,
    build_mail_delete_packet, build_mail_get_item_packet, build_mail_get_list_packet,
    build_mail_open_packet, build_mail_reset_item_packet, build_mail_send_packet,
    build_make_char_packet, build_make_char_with_stats_packet, build_make_guild,
    build_make_party_packet, build_make_party2_packet, build_map_loaded_packet,
    build_mercenary_command_packet, build_move_item_body_to_cart_packet,
    build_move_item_body_to_store_packet, build_move_item_cart_to_body_packet,
    build_move_item_cart_to_store_packet, build_move_item_store_to_body_packet,
    build_move_item_store_to_cart_packet, build_npc_close_packet, build_npc_deal_type_packet,
    build_npc_input_number_packet, build_npc_input_string_packet, build_npc_menu_select_packet,
    build_npc_next_packet, build_party_chat_packet, build_party_invite_by_name_packet,
    build_pet_act_packet, build_pickup_item_packet, build_progress_done_packet,
    build_purchase_frommc_dispatch, build_purchase_frommc_packet, build_purchase_frommc2_packet,
    build_purchase_item_list_packet, build_reg_change_guild_positioninfo,
    build_register_guild_emblem, build_remember_warppoint_packet, build_remove_option_packet,
    build_rename_homun_packet, build_rename_pet_packet, build_req_ally_guild, build_req_ban_guild,
    build_req_buy_frommc_packet, build_req_cancel_openstore_packet, build_req_change_memberpos,
    build_req_closestore_packet, build_req_delete_related_guild, build_req_disconnect_packet,
    build_req_disorganize_guild, build_req_enter_room_packet, build_req_exchange_item_packet,
    build_req_guild_emblem_img, build_req_guild_menu, build_req_guild_menuinterface,
    build_req_hostile_guild, build_req_itemidentify_packet, build_req_itemrepair_packet,
    build_req_join_guild, build_req_join_party_packet, build_req_leave_guild,
    build_req_mail_return_packet, build_req_makingarrow_packet, build_req_makingitem_packet,
    build_req_openstore2_packet, build_req_weaponrefine_packet, build_reqname_packet,
    build_request_move_packet, build_restart_packet, build_return_savepoint_packet,
    build_select_accessible_map_packet, build_select_autospell_packet, build_select_char_packet,
    build_select_petegg_packet, build_select_warppoint_packet, build_sell_item_list_packet,
    build_setting_whisper_pc_packet, build_setting_whisper_state_packet,
    build_shortcut_key_change_packet, build_solve_char_name_packet,
    build_standing_resurrection_packet, build_stat_change_packet, build_taekwon_rank_packet,
    build_trycapture_packet, build_unequip_item_packet, build_upgrade_skill_packet,
    build_use_item_packet, build_use_skill_packet, build_use_skill_to_ground_packet,
    build_use_skill_to_ground_with_talkbox_packet, build_whisper_packet, build_zone_enter_packet,
};
use session::{Session, SessionState};
use std::collections::VecDeque;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub enum KeepaliveMode {
    Off,
    CharServer { account_id: u32 },
    MapServer,
}

pub enum NetworkCommand {
    Connect { addr: String, expect_aid: bool },
    SendPacket(Vec<u8>),
    Disconnect,
    SetKeepalive(KeepaliveMode),
    SetPacketver(u32),
}

pub async fn network_loop(
    mut cmd_rx: mpsc::UnboundedReceiver<NetworkCommand>,
    event_tx: mpsc::UnboundedSender<GameEvent>,
    packetver: u32,
    debug_delay_ms: u32,
    start_time: Instant,
) {
    let mut connection: Option<Connection> = None;
    let mut session = Session::new(packetver);
    let mut keepalive = KeepaliveMode::Off;
    let mut keepalive_interval = time::interval(Duration::from_secs(10));
    keepalive_interval.reset();
    let mut keepalive_send_time_ms: u32 = 0;
    let delay_duration = Duration::from_millis(debug_delay_ms as u64);
    let mut delayed_events: VecDeque<(Instant, GameEvent)> = VecDeque::new();
    let mut delayed_sends: VecDeque<(Instant, Vec<u8>)> = VecDeque::new();

    loop {
        while delayed_events
            .front()
            .is_some_and(|(t, _)| *t <= Instant::now())
        {
            let (_, event) = delayed_events.pop_front().unwrap();
            let _ = event_tx.send(event);
        }

        let mut ready_sends: Vec<Vec<u8>> = Vec::new();
        while delayed_sends
            .front()
            .is_some_and(|(t, _)| *t <= Instant::now())
        {
            ready_sends.push(delayed_sends.pop_front().unwrap().1);
        }
        if !ready_sends.is_empty() {
            let mut send_err = None;
            if let Some(conn) = &mut connection {
                for data in &ready_sends {
                    if let Err(e) = conn.send_packet(data, session.packetver).await {
                        send_err = Some(e.to_string());
                        break;
                    }
                }
            }
            if let Some(e) = send_err {
                error!("delayed send error: {e}");
                let _ = event_tx.send(GameEvent::Disconnected(e));
                connection = None;
                session.state = SessionState::Disconnected;
                keepalive = KeepaliveMode::Off;
            }
        }

        let next_release = delayed_events
            .front()
            .map(|(t, _)| *t)
            .into_iter()
            .chain(delayed_sends.front().map(|(t, _)| *t))
            .min();

        if let Some(conn) = &mut connection {
            tokio::select! {
                result = conn.recv_packets(session.packetver) => {
                    match result {
                        Ok(packets) => {
                            for packet in &packets {
                                let events = dispatch_packet(packet.as_ref(), session.packetver);
                                if events.is_empty() {
                                    info!("unhandled packet: {} (id={})", packet.name(), packet.id(session.packetver));
                                }
                                for event in events {
                                    let event = match event {
                                        GameEvent::ServerTick { server_tick, .. } => {
                                            GameEvent::ServerTick { server_tick, local_send_time_ms: keepalive_send_time_ms }
                                        }
                                        other => other,
                                    };
                                    if debug_delay_ms > 0 {
                                        delayed_events.push_back((Instant::now() + delay_duration, event));
                                    } else {
                                        let _ = event_tx.send(event);
                                    }
                                }
                            }
                        }
                        Err(ConnectionError::Disconnected) => {
                            info!("disconnected from server");
                            let _ = event_tx.send(GameEvent::Disconnected("connection closed".into()));
                            connection = None;
                            session.state = SessionState::Disconnected;
                            keepalive = KeepaliveMode::Off;
                        }
                        Err(ConnectionError::Io(e)) => {
                            error!("network error: {e}");
                            let _ = event_tx.send(GameEvent::Disconnected(e.to_string()));
                            connection = None;
                            session.state = SessionState::Disconnected;
                            keepalive = KeepaliveMode::Off;
                        }
                    }
                }
                _ = async {
                    match next_release {
                        Some(t) => time::sleep(t.saturating_duration_since(Instant::now())).await,
                        None => std::future::pending::<()>().await,
                    }
                } => {}
                _ = keepalive_interval.tick() => {
                    if let Some(conn) = &mut connection {
                        let packet = match &keepalive {
                            KeepaliveMode::Off => None,
                            KeepaliveMode::CharServer { account_id } => {
                                Some(sender::build_char_ping_packet(*account_id, session.packetver))
                            }
                            KeepaliveMode::MapServer => {
                                let client_time = start_time.elapsed().as_millis() as u32;
                                keepalive_send_time_ms = client_time;
                                Some(sender::build_request_time_packet(client_time, session.packetver))
                            }
                        };
                        if let Some(data) = packet {
                            if debug_delay_ms > 0 {
                                delayed_sends.push_back((Instant::now() + delay_duration, data));
                            } else if let Err(e) = conn.send_packet(&data, session.packetver).await {
                                error!("keepalive send error: {e}");
                                let _ = event_tx.send(GameEvent::Disconnected(e.to_string()));
                                connection = None;
                                session.state = SessionState::Disconnected;
                                keepalive = KeepaliveMode::Off;
                            }
                        }
                    }
                }
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(NetworkCommand::SendPacket(data)) => {
                            if debug_delay_ms > 0 {
                                delayed_sends.push_back((Instant::now() + delay_duration, data));
                            } else if let Some(conn) = &mut connection
                                && let Err(e) = conn.send_packet(&data, session.packetver).await {
                                    error!("send error: {e}");
                                    let _ = event_tx.send(GameEvent::Disconnected(e.to_string()));
                                    connection = None;
                                    session.state = SessionState::Disconnected;
                                }
                        }
                        Some(NetworkCommand::Connect { addr, expect_aid }) => {
                            match Connection::connect(&addr, expect_aid).await {
                                Ok(conn) => {
                                    info!("connected to {addr}");
                                    connection = Some(conn);
                                }
                                Err(e) => {
                                    error!("connect error: {e}");
                                    let _ = event_tx.send(GameEvent::Disconnected(e.to_string()));
                                }
                            }
                        }
                        Some(NetworkCommand::Disconnect) => {
                            connection = None;
                            session.state = SessionState::Disconnected;
                            keepalive = KeepaliveMode::Off;
                        }
                        Some(NetworkCommand::SetKeepalive(mode)) => {
                            keepalive = mode;
                            keepalive_interval.reset();
                        }
                        Some(NetworkCommand::SetPacketver(ver)) => {
                            session.packetver = ver;
                        }
                        None => {
                            return;
                        }
                    }
                }
            }
        } else {
            match cmd_rx.recv().await {
                Some(NetworkCommand::Connect { addr, expect_aid }) => {
                    match Connection::connect(&addr, expect_aid).await {
                        Ok(conn) => {
                            info!("connected to {addr}");
                            connection = Some(conn);
                        }
                        Err(e) => {
                            error!("connect error: {e}");
                            let _ = event_tx.send(GameEvent::Disconnected(e.to_string()));
                        }
                    }
                }
                Some(NetworkCommand::Disconnect) => {}
                Some(NetworkCommand::SendPacket(_)) => {
                    error!("cannot send packet: not connected");
                }
                Some(NetworkCommand::SetKeepalive(mode)) => {
                    keepalive = mode;
                    keepalive_interval.reset();
                }
                Some(NetworkCommand::SetPacketver(ver)) => {
                    session.packetver = ver;
                }
                None => return,
            }
        }
    }
}
