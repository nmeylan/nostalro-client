use std::io;
use std::panic;

use packets::packets::Packet;
use packets::packets_parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tracing::debug;

#[derive(Debug)]
pub enum ConnectionError {
    Io(io::Error),
    Disconnected,
}

impl From<io::Error> for ConnectionError {
    fn from(err: io::Error) -> Self {
        ConnectionError::Io(err)
    }
}

pub struct Connection {
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    recv_buffer: Vec<u8>,
    trace_packets: bool,
}

impl Connection {
    pub async fn connect(addr: &str, trace_packets: bool) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader,
            writer,
            recv_buffer: Vec::with_capacity(4096),
            trace_packets,
        })
    }

    pub async fn send_packet(&mut self, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data).await
    }

    /// Check if a packet is variable-length, supplementing the server's
    /// `is_variable_length` with packets it doesn't list.
    /// Packets whose `from()` reads msg/data using `buffer.len()` instead of
    /// `packet_length` MUST be listed here so we slice the buffer first.
    fn is_variable_length_packet(packet_id: [u8; 2], packetver: u32) -> bool {
        if packets_parser::is_variable_length(packet_id, packetver) {
            return true;
        }
        matches!(packet_id,
            [0x8d, 0x00] | // ZC_NOTIFY_CHAT
            [0x8e, 0x00] | // ZC_NOTIFY_PLAYERCHAT
            [0x92, 0x00] | // ZC_NPCACK_SERVERMOVE
            [0x97, 0x00] | // ZC_WHISPER
            [0x9a, 0x00] | // ZC_BROADCAST
            [0xb4, 0x00] | // ZC_SAY_DIALOG
            [0xb7, 0x00] | // ZC_MENU_LIST
            [0xd7, 0x00] | // ZC_ROOM_NEWENTRY
            [0xdf, 0x00] | // ZC_CHANGE_CHATROOM
            [0x09, 0x01] | // ZC_NOTIFY_CHAT_PARTY
            [0x11, 0x01] | // ZC_ADD_SKILL
            [0x52, 0x01] | // ZC_GUILD_EMBLEM_IMG
            [0x76, 0x01] | // ZC_ACK_GUILD_MEMBER_INFO
            [0x7b, 0x01] | // ZC_ITEMCOMPOSITION_LIST
            [0x77, 0x01] | // ZC_ITEMIDENTIFY_LIST
            [0x7f, 0x01] | // ZC_GUILD_CHAT
            [0x82, 0x01] | // ZC_MEMBER_ADD
            [0x8c, 0x01] | // ZC_MONSTER_INFO
            [0x8d, 0x01] | // ZC_MAKABLEITEMLIST
            [0xc1, 0x02] | // ZC_NPC_CHAT
            [0xc3, 0x01] | // ZC_BROADCAST2
            [0x1f, 0x02] | // ZC_NOTIFY_PKINFO
            [0x42, 0x02] | // ZC_MAIL_REQ_OPEN
            [0x5a, 0x02] | // ZC_MAKINGITEM_LIST
            [0xdc, 0x02] | // ZC_BATTLEFIELD_CHAT
            [0xe7, 0x02] | // ZC_MAPPROPERTY
            [0x47, 0x01] | // ZC_AUTORUN_SKILL
            [0x3b, 0x0a] | // ZC_HAT_EFFECT
            [0x1f, 0x08]   // ZC_BROADCAST4
        )
    }

    /// Fixed-size packets whose parser uses `buffer.len()` to count
    /// repeating entries. We must slice the buffer to the correct size
    /// before parsing, otherwise they consume trailing packets.
    fn fixed_packet_size(packet_id: [u8; 2], packetver: u32) -> Option<usize> {
        match packet_id {
            // ZC_SHORTCUT_KEY_LIST_V2 (0x07d9): 2 + 38*7 = 268
            // ZC_SHORTCUT_KEY_LIST   (0x02b9): 2 + 27*7 = 191
            [0xd9, 0x07] => {
                let count = if packetver >= 20090617 { 38 } else { 36 };
                Some(2 + count * 7)
            }
            [0xb9, 0x02] => Some(2 + 27 * 7),
            // ZC_SHORTCUT_KEY_LIST_V3 (0x0a00): 3 + 38*7 = 269
            [0x00, 0x0a] => Some(3 + 38 * 7),
            _ => None,
        }
    }

    /// Estimate the byte length of an unknown RO packet.
    /// Variable-length packets store their total length at bytes 2-3.
    /// Fixed-length packets need a lookup table we don't have, so we
    /// fall back to the minimum packet size (4 bytes).
    fn estimate_packet_len(data: &[u8]) -> usize {
        if data.len() >= 4 {
            let len = u16::from_le_bytes([data[2], data[3]]) as usize;
            if len >= 4 && len <= data.len() {
                return len;
            }
        }
        4.min(data.len())
    }

    /// If bytes 2-3 look like a valid variable-length packet header,
    /// return a slice limited to that length. This prevents parsers that
    /// use `buffer.len()` instead of `packet_length` from over-reading.
    fn slice_to_packet_len(data: &[u8]) -> &[u8] {
        if data.len() >= 4 {
            let len = u16::from_le_bytes([data[2], data[3]]) as usize;
            if len >= 4 && len <= data.len() {
                return &data[..len];
            }
        }
        data
    }

    pub async fn recv_packets(&mut self, packetver: u32) -> Result<Vec<Box<dyn Packet>>, ConnectionError> {
        let mut buf = [0u8; 4096];
        let n = self.reader.read(&mut buf).await?;
        if n == 0 {
            return Err(ConnectionError::Disconnected);
        }
        self.recv_buffer.extend_from_slice(&buf[..n]);

        // Log every TCP read to diagnose missing packets
        tracing::info!("TCP read: {n} bytes, buffer_total={}, first_16={:02x?}",
            self.recv_buffer.len(), &buf[..n.min(16)]);

        let mut packets = Vec::new();
        let mut offset = 0;

        while offset < self.recv_buffer.len() {
            let remaining = self.recv_buffer[offset..].to_vec();
            // Slice the buffer to the correct packet size before parsing:
            // - Variable-length packets: use the length field at bytes [2:3]
            // - Known fixed-size packets with arrays: use our lookup table
            // - Other packets: pass the full remaining buffer
            let parse_buf = if remaining.len() >= 2 {
                let pkt_id = [remaining[0], remaining[1]];
                if Self::is_variable_length_packet(pkt_id, packetver) {
                    Self::slice_to_packet_len(&remaining)
                } else if let Some(fixed_size) = Self::fixed_packet_size(pkt_id, packetver) {
                    if fixed_size <= remaining.len() {
                        &remaining[..fixed_size]
                    } else {
                        &remaining
                    }
                } else {
                    &remaining
                }
            } else {
                &remaining
            };
            let result = panic::catch_unwind(|| {
                packets_parser::parse(parse_buf, packetver)
            });
            match result {
                Ok(packet) => {
                    if packet.name() == "Unknown" {
                        let skip = Self::estimate_packet_len(&remaining);
                        tracing::info!("skipping unknown packet 0x{:02x}{:02x} ({skip} bytes), buffer_remaining={}",
                               remaining[0], remaining[1], remaining.len());
                        if self.trace_packets {
                            let dump_len = skip.min(remaining.len());
                            tracing::debug!("unknown packet dump: {:02x?}", &remaining[..dump_len]);
                        }
                        offset += skip;
                        continue;
                    }
                    let consumed = packet.raw().len();
                    tracing::info!("recv {} ({consumed} bytes, remaining={})", packet.name(), remaining.len());
                    if self.trace_packets {
                        tracing::debug!("packet dump {}: {:02x?}", packet.name(), packet.raw());
                    }
                    offset += consumed;
                    packets.push(packet);
                }
                Err(_) => {
                    tracing::warn!("packet parse panic at offset {offset}, buffer_len={}, first_bytes=0x{:02x}{:02x}",
                        self.recv_buffer.len(),
                        remaining.get(0).copied().unwrap_or(0),
                        remaining.get(1).copied().unwrap_or(0));
                    // Skip past the bad data to avoid permanently blocking the buffer
                    let skip = Self::estimate_packet_len(&remaining);
                    offset += skip;
                    continue;
                }
            }
        }

        self.recv_buffer.drain(..offset);
        Ok(packets)
    }
}
