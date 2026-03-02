pub mod connection;
pub mod handler;
pub mod helpers;
pub mod session;

use connection::{Connection, ConnectionError};
use handler::dispatch_packet;
pub use helpers::{encode_pos, ip_u32_to_string};
use packets::packets::*;
use ragnarok_game::event::GameEvent;
use session::{Session, SessionState};
use tokio::sync::mpsc;
use tracing::{error, info};

pub enum NetworkCommand {
    Connect(String),
    SendPacket(Vec<u8>),
    Disconnect,
}

pub async fn network_loop(
    mut cmd_rx: mpsc::UnboundedReceiver<NetworkCommand>,
    event_tx: mpsc::UnboundedSender<GameEvent>,
    packetver: u32,
) {
    let mut connection: Option<Connection> = None;
    let mut session = Session::new(packetver);

    loop {
        if let Some(conn) = &mut connection {
            tokio::select! {
                result = conn.recv_packets(session.packetver) => {
                    match result {
                        Ok(packets) => {
                            for packet in &packets {
                                if let Some(event) = dispatch_packet(packet.as_ref(), session.packetver) {
                                    let _ = event_tx.send(event);
                                }
                            }
                        }
                        Err(ConnectionError::Disconnected) => {
                            info!("disconnected from server");
                            let _ = event_tx.send(GameEvent::Disconnected("connection closed".into()));
                            connection = None;
                            session.state = SessionState::Disconnected;
                        }
                        Err(ConnectionError::Io(e)) => {
                            error!("network error: {e}");
                            let _ = event_tx.send(GameEvent::Disconnected(e.to_string()));
                            connection = None;
                            session.state = SessionState::Disconnected;
                        }
                    }
                }
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(NetworkCommand::SendPacket(data)) => {
                            if let Some(conn) = &mut connection {
                                if let Err(e) = conn.send_packet(&data).await {
                                    error!("send error: {e}");
                                    let _ = event_tx.send(GameEvent::Disconnected(e.to_string()));
                                    connection = None;
                                    session.state = SessionState::Disconnected;
                                }
                            }
                        }
                        Some(NetworkCommand::Connect(addr)) => {
                            match Connection::connect(&addr).await {
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
                        }
                        None => {
                            return;
                        }
                    }
                }
            }
        } else {
            // No connection, only process commands
            match cmd_rx.recv().await {
                Some(NetworkCommand::Connect(addr)) => {
                    match Connection::connect(&addr).await {
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
                None => return,
            }
        }
    }
}

/// Build a login packet and return its raw bytes.
pub fn build_login_packet(username: &str, password: &str, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCaLogin::new(packetver);
    pkt.set_version(20);

    let mut id = [0 as char; 24];
    for (i, c) in username.chars().take(23).enumerate() {
        id[i] = c;
    }
    pkt.set_id(id);

    let mut passwd = [0 as char; 24];
    for (i, c) in password.chars().take(23).enumerate() {
        passwd[i] = c;
    }
    pkt.set_passwd(passwd);
    pkt.set_client_type(0);
    pkt.fill_raw();
    pkt.raw().clone()
}

/// Build a char-server enter packet.
pub fn build_char_enter_packet(session: &Session) -> Vec<u8> {
    let mut pkt = PacketChEnter::new(session.packetver);
    pkt.set_aid(session.account_id);
    pkt.set_auth_code(session.login_id1);
    pkt.set_user_level(session.login_id2);
    pkt.set_client_type(0);
    pkt.set_sex(session.sex);
    pkt.fill_raw();
    pkt.raw().clone()
}

/// Build a character select packet.
pub fn build_select_char_packet(slot: u8, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketChSelectChar::new(packetver);
    pkt.set_char_num(slot);
    pkt.fill_raw();
    pkt.raw().clone()
}

/// Build a move request packet.
pub fn build_request_move_packet(dest_x: u16, dest_y: u16, packetver: u32) -> Vec<u8> {
    let mut pkt = PacketCzRequestMove::new(packetver);
    pkt.set_dest(helpers::encode_pos(dest_x, dest_y, 0));
    pkt.fill_raw();
    pkt.raw().clone()
}

/// Build a zone-server enter packet.
pub fn build_zone_enter_packet(session: &Session) -> Vec<u8> {
    let mut pkt = PacketCzEnter2::new(session.packetver);
    pkt.set_aid(session.account_id);
    pkt.set_gid(session.char_id);
    pkt.set_auth_code(session.login_id1);
    pkt.set_client_time(0);
    pkt.set_sex(session.sex);
    pkt.fill_raw();
    pkt.raw().clone()
}
