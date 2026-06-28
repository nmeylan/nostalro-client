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
    build_action_request_packet, build_card_composition_list_packet, build_card_composition_packet,
    build_cartoff_packet, build_change_cart_packet, build_change_party_exp_option_packet,
    build_char_enter_packet, build_chat_packet, build_expel_party_member_packet,
    build_join_party_reply_packet, build_leave_party_packet, build_make_party_packet,
    build_party_chat_packet, build_req_join_party_packet,
    build_contact_npc_packet, build_drop_item_packet, build_equip_item_packet, build_login_packet,
    build_map_loaded_packet, build_move_item_body_to_cart_packet,
    build_move_item_cart_to_body_packet, build_move_item_cart_to_store_packet,
    build_move_item_store_to_cart_packet, build_npc_close_packet, build_npc_deal_type_packet,
    build_npc_input_number_packet, build_npc_input_string_packet, build_npc_menu_select_packet,
    build_npc_next_packet, build_pickup_item_packet, build_purchase_item_list_packet,
    build_remove_option_packet, build_req_enter_room_packet, build_reqname_packet,
    build_request_move_packet, build_restart_packet, build_select_char_packet,
    build_select_warppoint_packet, build_sell_item_list_packet, build_shortcut_key_change_packet,
    build_stat_change_packet, build_unequip_item_packet, build_upgrade_skill_packet,
    build_use_item_packet, build_use_skill_packet, build_use_skill_to_ground_packet,
    build_zone_enter_packet,
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
    Connect(String),
    SendPacket(Vec<u8>),
    Disconnect,
    SetKeepalive(KeepaliveMode),
}

pub async fn network_loop(
    mut cmd_rx: mpsc::UnboundedReceiver<NetworkCommand>,
    event_tx: mpsc::UnboundedSender<GameEvent>,
    packetver: u32,
    debug_delay_ms: u32,
    trace_packets_send: bool,
    trace_packets_recv: bool,
) {
    let mut connection: Option<Connection> = None;
    let mut session = Session::new(packetver);
    let mut keepalive = KeepaliveMode::Off;
    let mut keepalive_interval = time::interval(Duration::from_secs(10));
    keepalive_interval.reset();
    let start_time = Instant::now();
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
                        Some(NetworkCommand::Connect(addr)) => {
                            match Connection::connect(&addr, trace_packets_send, trace_packets_recv).await {
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
                            info!("disconnecting");
                            connection = None;
                            session.state = SessionState::Disconnected;
                            keepalive = KeepaliveMode::Off;
                        }
                        Some(NetworkCommand::SetKeepalive(mode)) => {
                            keepalive = mode;
                            keepalive_interval.reset();
                        }
                        None => {
                            return;
                        }
                    }
                }
            }
        } else {
            match cmd_rx.recv().await {
                Some(NetworkCommand::Connect(addr)) => {
                    match Connection::connect(&addr, trace_packets_send, trace_packets_recv).await {
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
                None => return,
            }
        }
    }
}
