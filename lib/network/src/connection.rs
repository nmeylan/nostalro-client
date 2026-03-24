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
}

impl Connection {
    pub async fn connect(addr: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader,
            writer,
            recv_buffer: Vec::with_capacity(4096),
        })
    }

    pub async fn send_packet(&mut self, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data).await
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

        // Log first bytes of each TCP read to diagnose missing packets
        // 0x0915 = PacketZcNotifyStandentry7
        let has_entity = buf[..n].windows(2).any(|w| w[0] == 0x15 && w[1] == 0x09);
        if has_entity || n > 200 {
            tracing::info!("TCP read: {n} bytes, buffer_total={}, has_standentry7={has_entity}, first_16={:02x?}",
                self.recv_buffer.len(), &buf[..n.min(16)]);
        }

        let mut packets = Vec::new();
        let mut offset = 0;

        while offset < self.recv_buffer.len() {
            let remaining = self.recv_buffer[offset..].to_vec();
            // Slice variable-length packets to their declared size so that
            // greedy parsers (e.g. NormalItemlist3) don't consume trailing data.
            let parse_buf = Self::slice_to_packet_len(&remaining);
            let result = panic::catch_unwind(|| {
                packets_parser::parse(parse_buf, packetver)
            });
            match result {
                Ok(packet) => {
                    if packet.name() == "Unknown" {
                        let skip = Self::estimate_packet_len(&remaining);
                        tracing::info!("skipping unknown packet 0x{:02x}{:02x} ({skip} bytes), buffer_remaining={}",
                               remaining[0], remaining[1], remaining.len());
                        offset += skip;
                        continue;
                    }
                    let consumed = packet.raw().len();
                    tracing::info!("recv {} ({consumed} bytes, remaining={})", packet.name(), remaining.len());
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
