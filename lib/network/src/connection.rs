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

    pub async fn recv_packets(&mut self, packetver: u32) -> Result<Vec<Box<dyn Packet>>, ConnectionError> {
        let mut buf = [0u8; 4096];
        let n = self.reader.read(&mut buf).await?;
        if n == 0 {
            return Err(ConnectionError::Disconnected);
        }
        self.recv_buffer.extend_from_slice(&buf[..n]);

        let mut packets = Vec::new();
        let mut offset = 0;

        while offset < self.recv_buffer.len() {
            let remaining = self.recv_buffer[offset..].to_vec();
            let result = panic::catch_unwind(|| {
                packets_parser::parse(&remaining, packetver)
            });
            match result {
                Ok(packet) => {
                    let consumed = packet.raw().len();
                    debug!("recv {}", packet.name());
                    offset += consumed;
                    packets.push(packet);
                }
                Err(_) => break,
            }
        }

        self.recv_buffer.drain(..offset);
        Ok(packets)
    }
}
