use crate::{Color, FormatError};

pub struct PalFile {
    pub colors: [Color; 256],
}

impl PalFile {
    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        if data.len() < 1024 {
            return Err(FormatError::UnexpectedEof);
        }
        let mut colors = [[0u8; 4]; 256];
        for i in 0..256 {
            let offset = i * 4;
            colors[i] = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ];
        }
        Ok(PalFile { colors })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_palette_roundtrip() {
        let mut data = [0u8; 1024];
        // Set a few known colors
        data[0..4].copy_from_slice(&[255, 0, 0, 0]); // index 0: red, reserved=0
        data[4..8].copy_from_slice(&[0, 255, 0, 128]); // index 1: green
        data[1020..1024].copy_from_slice(&[0, 0, 255, 255]); // index 255: blue

        let pal = PalFile::parse(&data).unwrap();
        assert_eq!(pal.colors[0], [255, 0, 0, 0]);
        assert_eq!(pal.colors[1], [0, 255, 0, 128]);
        assert_eq!(pal.colors[255], [0, 0, 255, 255]);
        assert_eq!(pal.colors[128], [0, 0, 0, 0]);
    }
}
