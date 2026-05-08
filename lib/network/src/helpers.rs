/// Decode 3 bytes into (x, y, dir) using RO's bit-packing:
/// byte0[7:0] byte1[7:0] byte2[7:0] = x[9:0] y[9:0] dir[3:0]
pub fn decode_pos(data: &[u8; 3]) -> (u16, u16, u8) {
    let x = ((data[0] as u16) << 2) | ((data[1] as u16) >> 6);
    let y = (((data[1] as u16) & 0x3F) << 4) | ((data[2] as u16) >> 4);
    let dir = data[2] & 0x0F;
    (x, y, dir)
}

/// Decode 6 bytes into two positions (x1, y1, x2, y2) for move packets.
/// 48 bits = x1[9:0] y1[9:0] x2[9:0] y2[9:0] sx[3:0] sy[3:0]
pub fn decode_pos2(data: &[u8; 6]) -> (u16, u16, u16, u16) {
    let x1 = ((data[0] as u16) << 2) | ((data[1] as u16) >> 6);
    let y1 = (((data[1] as u16) & 0x3F) << 4) | ((data[2] as u16) >> 4);
    let x2 = (((data[2] as u16) & 0x0F) << 6) | ((data[3] as u16) >> 2);
    let y2 = (((data[3] as u16) & 0x03) << 8) | (data[4] as u16);
    (x1, y1, x2, y2)
}

/// Convert u32 IP (little-endian as sent by server) to dotted-quad string.
pub fn ip_u32_to_string(ip: u32) -> String {
    let bytes = ip.to_le_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

pub fn encode_pos(x: u16, y: u16, dir: u8) -> [u8; 3] {
    [
        (x >> 2) as u8,
        ((x << 6) as u8) | ((y >> 4) as u8),
        ((y << 4) as u8) | (dir & 0x0F),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_u32_to_string_converts_correctly() {
        assert_eq!(ip_u32_to_string(0x0100007F), "127.0.0.1");
        assert_eq!(ip_u32_to_string(0x6401A8C0), "192.168.1.100");
    }

    #[test]
    fn encode_decode_pos_roundtrip() {
        let cases = [(100, 200, 3), (0, 0, 0), (1023, 1023, 15), (512, 384, 7)];
        for (x, y, dir) in cases {
            let encoded = encode_pos(x, y, dir);
            let (dx, dy, ddir) = decode_pos(&encoded);
            assert_eq!(
                (dx, dy, ddir),
                (x, y, dir),
                "roundtrip failed for ({x}, {y}, {dir})"
            );
        }
    }

    #[test]
    fn decode_pos2_extracts_two_positions() {
        // Encode (100, 200) and (110, 210) manually
        // x1=100 (0b0001100100), y1=200 (0b0011001000)
        // x2=110 (0b0001101110), y2=210 (0b0011010010)
        let x1: u16 = 100;
        let y1: u16 = 200;
        let x2: u16 = 110;
        let y2: u16 = 210;
        // Pack into 6 bytes: x1[9:0] y1[9:0] x2[9:0] y2[9:0] padding[7:0]
        let b0 = (x1 >> 2) as u8;
        let b1 = ((x1 << 6) as u8) | ((y1 >> 4) as u8);
        let b2 = ((y1 << 4) as u8) | ((x2 >> 6) as u8);
        let b3 = ((x2 << 2) as u8) | ((y2 >> 8) as u8);
        let b4 = y2 as u8;
        let b5 = 0u8;
        let data = [b0, b1, b2, b3, b4, b5];
        let (dx1, dy1, dx2, dy2) = decode_pos2(&data);
        assert_eq!((dx1, dy1), (x1, y1));
        assert_eq!((dx2, dy2), (x2, y2));
    }
}
