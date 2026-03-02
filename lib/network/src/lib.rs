pub mod connection;
pub mod handler;
pub mod helpers;
pub mod sender;
pub mod session;

use connection::{Connection, ConnectionError};
use handler::dispatch_packet;
pub use helpers::{encode_pos, ip_u32_to_string};
use ragnarok_game::event::GameEvent;
pub use sender::{build_char_enter_packet, build_login_packet, build_request_move_packet, build_select_char_packet, build_zone_enter_packet};
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

