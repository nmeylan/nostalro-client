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
    expect_aid_preamble: bool,
}

impl Connection {
    pub async fn connect(
        addr: &str,
        expect_aid_preamble: bool,
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
            expect_aid_preamble,
        })
    }

    pub async fn send_packet(&mut self, data: &[u8], packetver: u32) -> io::Result<()> {
        if self.trace_packets_send {
            let name = panic::catch_unwind(|| packets_parser::parse(data, packetver).name().to_string())
                .unwrap_or_else(|_| "<parse panic>".to_string());
            let preview: Vec<String> = data.iter().take(16).map(|b| format!("{b:02x}")).collect();
            tracing::info!(
                "send packet: {} ({} bytes) [{}]",
                name,
                data.len(),
                preview.join(" ")
            );
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

        tracing::info!(
            "TCP read: {n} bytes, buffer_total={}, first_16={:02x?}",
            self.recv_buffer.len(),
            &buf[..n.min(16)]
        );

        if self.expect_aid_preamble {
            if self.recv_buffer.len() < 4 {
                return Ok(Vec::new());
            }
            let aid = u32::from_le_bytes([
                self.recv_buffer[0],
                self.recv_buffer[1],
                self.recv_buffer[2],
                self.recv_buffer[3],
            ]);
            tracing::info!("consumed account_id preamble: {aid}");
            self.recv_buffer.drain(..4);
            self.expect_aid_preamble = false;
        }

        let mut packets = Vec::new();
        let mut offset = 0;

        while offset < self.recv_buffer.len() {
            let remaining = self.recv_buffer[offset..].to_vec();
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
                        if self.trace_packets_recv {
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
