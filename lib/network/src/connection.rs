use std::io;
use std::panic;

use packets::packets::Packet;
use packets::packets_parser;
use ragnarok_profiling::debug::{self, PacketTrace};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

const ZC_AID_PACKET_ID: u16 = 0x0283;

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

/// Periodic keepalive/time packets excluded from packet tracing regardless of mode.
fn is_muted_packet(name: &str) -> bool {
    matches!(
        name,
        "PacketCzRequestTime" | "PacketCzPing" | "PacketZcNotifyTime"
    )
}

pub struct Connection {
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    recv_buffer: Vec<u8>,
    expect_aid_preamble: bool,
}

impl Connection {
    pub async fn connect(addr: &str, expect_aid_preamble: bool) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader,
            writer,
            recv_buffer: Vec::with_capacity(4096),
            expect_aid_preamble,
        })
    }

    pub async fn send_packet(&mut self, data: &[u8], packetver: u32) -> io::Result<()> {
        if debug::packet_trace() == PacketTrace::All {
            let name =
                panic::catch_unwind(|| packets_parser::parse(data, packetver).name().to_string())
                    .unwrap_or_else(|_| "<parse panic>".to_string());
            if !is_muted_packet(&name) {
                let preview: Vec<String> =
                    data.iter().take(16).map(|b| format!("{b:02x}")).collect();
                tracing::info!(
                    "send packet: {} ({} bytes) [{}]",
                    name,
                    data.len(),
                    preview.join(" ")
                );
            }
        }
        self.writer.write_all(data).await
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

        let trace = debug::packet_trace();

        if trace == PacketTrace::All {
            tracing::info!(
                "TCP read: {n} bytes, buffer_total={}, first_16={:02x?}",
                self.recv_buffer.len(),
                &buf[..n.min(16)]
            );
        }

        if self.expect_aid_preamble {
            if self.recv_buffer.len() < 4 {
                return Ok(Vec::new());
            }
            if let Some(aid) = Self::take_bare_aid_preamble(&mut self.recv_buffer)
                && trace == PacketTrace::All
            {
                tracing::info!("consumed account_id preamble: {aid}");
            }
            self.expect_aid_preamble = false;
        }

        let packets = Self::drain_packets(&mut self.recv_buffer, packetver, trace);
        Ok(packets)
    }

    /// Drops the headerless account id a server may greet with. Zone servers from
    /// packetver 20070521 greet with `ZC_AID` instead, which the parser handles.
    fn take_bare_aid_preamble(buffer: &mut Vec<u8>) -> Option<u32> {
        if u16::from_le_bytes([buffer[0], buffer[1]]) == ZC_AID_PACKET_ID {
            return None;
        }
        let aid = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        buffer.drain(..4);
        Some(aid)
    }

    fn drain_packets(
        buffer: &mut Vec<u8>,
        packetver: u32,
        trace: PacketTrace,
    ) -> Vec<Box<dyn Packet>> {
        let mut packets = Vec::new();
        let mut offset = 0;

        while offset < buffer.len() {
            let remaining = buffer[offset..].to_vec();
            let is_variable = remaining.len() >= 2
                && packets_parser::is_variable_length([remaining[0], remaining[1]], packetver);
            let parse_buf = if is_variable {
                Self::slice_to_packet_len(&remaining)
            } else {
                &remaining
            };
            let declared_len = parse_buf.len();
            let result = panic::catch_unwind(|| packets_parser::parse(parse_buf, packetver));
            match result {
                Ok(packet) => {
                    if packet.name() == "Unknown" {
                        let skip = Self::estimate_packet_len(&remaining);
                        tracing::info!(
                            "skipping unknown packet 0x{:02x}{:02x} ({skip} bytes), buffer_remaining={}",
                            remaining[0],
                            remaining[1],
                            remaining.len()
                        );
                        if trace == PacketTrace::All {
                            let dump_len = skip.min(remaining.len());
                            tracing::debug!("unknown packet dump: {:02x?}", &remaining[..dump_len]);
                        }
                        offset += skip;
                        continue;
                    }
                    // Variable-length packets advance by their declared length; a
                    // struct that under-reads its body would otherwise desync the stream.
                    let consumed = if is_variable {
                        declared_len
                    } else {
                        packet.raw().len()
                    };
                    if trace == PacketTrace::All && !is_muted_packet(packet.name()) {
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
                        buffer.len(),
                        remaining.first().copied().unwrap_or(0),
                        remaining.get(1).copied().unwrap_or(0)
                    );
                    let skip = Self::estimate_packet_len(&remaining);
                    offset += skip;
                    continue;
                }
            }
        }

        buffer.drain(..offset);
        packets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use packets::packets::{PacketZcAcceptEnter2, PacketZcAid};

    fn greeting_stream(packetver: u32, aid_bytes: &[u8]) -> Vec<u8> {
        let mut enter = PacketZcAcceptEnter2::new(packetver);
        enter.set_x_size(5);
        enter.set_y_size(5);
        enter.fill_raw_with_packetver(Some(packetver));
        let mut stream = aid_bytes.to_vec();
        stream.extend_from_slice(&enter.raw);
        stream
    }

    #[test]
    fn zone_greeting_frames_whatever_form_the_aid_takes() {
        let packetver = 20111102;
        let mut aid = PacketZcAid::new(packetver);
        aid.set_aid(2000000);
        aid.fill_raw_with_packetver(Some(packetver));

        let mut buffer = greeting_stream(packetver, &aid.raw);
        assert!(Connection::take_bare_aid_preamble(&mut buffer).is_none());
        let framed = Connection::drain_packets(&mut buffer, packetver, PacketTrace::None);
        assert_eq!(framed.len(), 2);
        assert_eq!(
            framed[0].as_any().downcast_ref::<PacketZcAid>().unwrap().aid,
            2000000
        );
        assert!(framed[1].as_any().is::<PacketZcAcceptEnter2>());
        assert!(buffer.is_empty());

        let mut buffer = greeting_stream(packetver, &2000000u32.to_le_bytes());
        assert_eq!(Connection::take_bare_aid_preamble(&mut buffer), Some(2000000));
        let framed = Connection::drain_packets(&mut buffer, packetver, PacketTrace::None);
        assert_eq!(framed.len(), 1);
        assert!(framed[0].as_any().is::<PacketZcAcceptEnter2>());
        assert!(buffer.is_empty());
    }
}
