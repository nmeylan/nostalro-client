use std::io;
use std::panic;

use packets::packets::Packet;
use packets::packets_parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

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
    trace_packets_send: bool,
    trace_packets_recv: bool,
}

impl Connection {
    pub async fn connect(
        addr: &str,
        trace_packets_send: bool,
        trace_packets_recv: bool,
    ) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader,
            writer,
            recv_buffer: Vec::with_capacity(4096),
            trace_packets_send,
            trace_packets_recv,
        })
    }

    pub async fn send_packet(&mut self, data: &[u8], packetver: u32) -> io::Result<()> {
        if self.trace_packets_send {
            let result = panic::catch_unwind(|| packets_parser::parse(data, packetver));
            if let Ok(packet) = result {
                tracing::info!("send packet: {:?}", packet.name());
            }
        }
        self.writer.write_all(data).await
    }

    fn is_variable_length_packet(packet_id: [u8; 2], packetver: u32) -> bool {
        if packets_parser::is_variable_length(packet_id, packetver) {
            return true;
        }
        matches!(
            packet_id,
            [0x8d, 0x00] | // ZC_NOTIFY_CHAT
            [0x8e, 0x00] | // ZC_NOTIFY_PLAYERCHAT
            [0x92, 0x00] | // ZC_NPCACK_SERVERMOVE
            [0x97, 0x00] | // ZC_WHISPER
            [0x9a, 0x00] | // ZC_BROADCAST
            [0xb4, 0x00] | // ZC_SAY_DIALOG
            [0xb7, 0x00] | // ZC_MENU_LIST
            [0xd7, 0x00] | // ZC_ROOM_NEWENTRY
            [0xdb, 0x00] | // ZC_ENTER_ROOM
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
            [0xad, 0x01] | // ZC_MAKINGARROW_LIST
            [0xfc, 0x01] | // ZC_REPAIRITEMLIST
            [0x21, 0x02] | // ZC_NOTIFY_WEAPONITEMLIST
            [0x36, 0x01] | // ZC_PC_PURCHASE_MYITEMLIST
            [0x00, 0x08] | // ZC_PC_PURCHASE_ITEMLIST_FROMMC (>=20100105)
            [0xc1, 0x02] | // ZC_NPC_CHAT
            [0xc3, 0x01] | // ZC_BROADCAST2
            [0x1f, 0x02] | // ZC_NOTIFY_PKINFO
            [0x42, 0x02] | // ZC_MAIL_REQ_OPEN
            [0x5a, 0x02] | // ZC_MAKINGITEM_LIST
            [0xdc, 0x02] | // ZC_BATTLEFIELD_CHAT
            [0xe7, 0x02] | // ZC_MAPPROPERTY
            [0x47, 0x01] | // ZC_AUTORUN_SKILL
            [0x3b, 0x0a] | // ZC_HAT_EFFECT
            [0x1f, 0x08] // ZC_BROADCAST4
        )
    }

    fn fixed_packet_size(packet_id: [u8; 2], packetver: u32) -> Option<usize> {
        match packet_id {
            // ZC_SHORTCUT_KEY_LIST_V2 (0x07d9): 2 + 38*7 = 268
            // ZC_SHORTCUT_KEY_LIST   (0x02b9): 2 + 27*7 = 191
            [0xd9, 0x07] => {
                let count = if packetver >= 20090617 { 38 } else { 36 };
                Some(2 + count * 7)
            }
            [0xb9, 0x02] => Some(2 + 27 * 7),
            // ZC_WARPLIST (0x011c): 2 + 2 + 4*16 = 68
            [0x1c, 0x01] => Some(2 + 2 + 4 * 16),
            // ZC_SHORTCUT_KEY_LIST_V3 (0x0a00): 3 + 38*7 = 269
            [0x00, 0x0a] => Some(3 + 38 * 7),
            // ZC_FASTMOVE (0x08d2): 2 + 4 + 2 + 2 = 10 (Snap). The parser has no
            // struct for it, so without this the id byte 0xe4 is misread as a
            // length and the stream desyncs.
            [0xd2, 0x08] => Some(10),
            _ => None,
        }
    }

    fn estimate_packet_len(data: &[u8]) -> usize {
        if data.len() >= 4 {
            let len = u16::from_le_bytes([data[2], data[3]]) as usize;
            if len >= 4 && len <= data.len() {
                return len;
            }
        }
        4.min(data.len())
    }

    /// Bytes to advance past a packet the typed parser can't decode. A known
    /// fixed size wins over the length-guess (which reads bytes [2..4] and is
    /// wrong for fixed packets whose id is followed by an id/coord field).
    fn skip_len(packet_id: [u8; 2], data: &[u8], packetver: u32) -> usize {
        if let Some(size) = Self::fixed_packet_size(packet_id, packetver) {
            return size.min(data.len());
        }
        Self::estimate_packet_len(data)
    }

    fn slice_to_packet_len(data: &[u8]) -> &[u8] {
        if data.len() >= 4 {
            let len = u16::from_le_bytes([data[2], data[3]]) as usize;
            if len >= 4 && len <= data.len() {
                return &data[..len];
            }
        }
        data
    }

    pub async fn recv_packets(
        &mut self,
        packetver: u32,
    ) -> Result<Vec<Box<dyn Packet>>, ConnectionError> {
        let mut buf = [0u8; 4096];
        let n = self.reader.read(&mut buf).await?;
        if n == 0 {
            return Err(ConnectionError::Disconnected);
        }
        self.recv_buffer.extend_from_slice(&buf[..n]);

        tracing::info!(
            "TCP read: {n} bytes, buffer_total={}, first_16={:02x?}",
            self.recv_buffer.len(),
            &buf[..n.min(16)]
        );

        let mut packets = Vec::new();
        let mut offset = 0;

        while offset < self.recv_buffer.len() {
            let remaining = self.recv_buffer[offset..].to_vec();
            // ZC_FASTMOVE (Snap, 0x08d2): id.W aid.L x.W y.W — byte-identical to
            // ZC_STOPMOVE, but the packet crate has no struct for it. Rebrand the
            // header so it parses as a stopmove and relocates the caster.
            if remaining.len() >= 10 && remaining[0] == 0xd2 && remaining[1] == 0x08 {
                let mut buf = remaining[..10].to_vec();
                buf[0] = 0x88;
                buf[1] = 0x00;
                if let Ok(pkt) = panic::catch_unwind(|| packets_parser::parse(&buf, packetver))
                    && pkt.name() != "Unknown"
                {
                    packets.push(pkt);
                }
                offset += 10;
                continue;
            }
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
            let result = panic::catch_unwind(|| packets_parser::parse(parse_buf, packetver));
            match result {
                Ok(packet) => {
                    if packet.name() == "Unknown" {
                        let skip =
                            Self::skip_len([remaining[0], remaining[1]], &remaining, packetver);
                        tracing::info!(
                            "skipping unknown packet 0x{:02x}{:02x} ({skip} bytes), buffer_remaining={}",
                            remaining[0],
                            remaining[1],
                            remaining.len()
                        );
                        if self.trace_packets_recv {
                            let dump_len = skip.min(remaining.len());
                            tracing::debug!("unknown packet dump: {:02x?}", &remaining[..dump_len]);
                        }
                        offset += skip;
                        continue;
                    }
                    let consumed = packet.raw().len();
                    if self.trace_packets_recv {
                        tracing::info!(
                            "recv {} ({consumed} bytes, remaining={})",
                            packet.name(),
                            remaining.len()
                        );
                    }
                    offset += consumed;
                    packets.push(packet);
                }
                Err(_) => {
                    tracing::warn!(
                        "packet parse panic at offset {offset}, buffer_len={}, first_bytes=0x{:02x}{:02x}",
                        self.recv_buffer.len(),
                        remaining.first().copied().unwrap_or(0),
                        remaining.get(1).copied().unwrap_or(0)
                    );
                    let pkt_id = [
                        remaining.first().copied().unwrap_or(0),
                        remaining.get(1).copied().unwrap_or(0),
                    ];
                    let skip = Self::skip_len(pkt_id, &remaining, packetver);
                    offset += skip;
                    continue;
                }
            }
        }

        self.recv_buffer.drain(..offset);
        Ok(packets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fastmove_skips_its_full_length_not_a_guess_from_the_id_field() {
        // ZC_FASTMOVE (Snap): id 0x08d2, then AID 0x001e84e4, then coords. The
        // bytes after the id (0xe4, 0x84) must not be misread as a length.
        let fastmove = [0xd2, 0x08, 0xe4, 0x84, 0x1e, 0x00, 0x36, 0x00, 0x86, 0x00];
        assert_eq!(
            Connection::skip_len([0xd2, 0x08], &fastmove, 20120307),
            10,
            "the whole 10-byte packet must be consumed to keep the stream aligned"
        );
    }
}
