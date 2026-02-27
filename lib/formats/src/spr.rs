use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::{Color, FormatError, version_at_least};

pub struct SprImage {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

pub struct SprFile {
    pub version: (u8, u8),
    pub indexed_sprites: Vec<SprImage>,
    pub rgba_sprites: Vec<SprImage>,
    pub palette: Option<[Color; 256]>,
}

impl SprFile {
    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Cursor::new(data);

        let mut sig = [0u8; 2];
        r.read_exact(&mut sig)?;
        if &sig != b"SP" {
            return Err(FormatError::InvalidMagic);
        }

        // MinorFirst: minor byte first, then major
        let ver_minor = r.read_u8()?;
        let ver_major = r.read_u8()?;
        let version = (ver_major, ver_minor);

        let palette_image_count = r.read_u16::<LE>()? as usize;

        let rgba_image_count = if version_at_least(version, 1, 2) {
            r.read_u16::<LE>()? as usize
        } else {
            0
        };

        let mut indexed_sprites = Vec::with_capacity(palette_image_count);
        for _ in 0..palette_image_count {
            let width = r.read_u16::<LE>()?;
            let height = r.read_u16::<LE>()?;
            let pixel_count = width as usize * height as usize;

            let data = if pixel_count == 0 {
                Vec::new()
            } else if version_at_least(version, 2, 1) {
                let encoded_size = r.read_u16::<LE>()? as usize;
                let mut encoded = vec![0u8; encoded_size];
                r.read_exact(&mut encoded)?;
                decode_rle(&encoded, pixel_count)
            } else {
                let mut raw = vec![0u8; pixel_count];
                r.read_exact(&mut raw)?;
                raw
            };

            indexed_sprites.push(SprImage { width, height, data });
        }

        let mut rgba_sprites = Vec::with_capacity(rgba_image_count);
        for _ in 0..rgba_image_count {
            let width = r.read_u16::<LE>()?;
            let height = r.read_u16::<LE>()?;
            let byte_count = width as usize * height as usize * 4;
            let mut data = vec![0u8; byte_count];
            r.read_exact(&mut data)?;
            rgba_sprites.push(SprImage { width, height, data });
        }

        let palette = if version_at_least(version, 1, 1) {
            let mut colors = [[0u8; 4]; 256];
            for color in &mut colors {
                r.read_exact(color)?;
            }
            Some(colors)
        } else {
            None
        };

        Ok(SprFile {
            version,
            indexed_sprites,
            rgba_sprites,
            palette,
        })
    }
}

fn decode_rle(data: &[u8], pixel_count: usize) -> Vec<u8> {
    let mut output = vec![0u8; pixel_count];
    let mut i = 0;
    let mut next = 0;

    while i < data.len() && next < pixel_count {
        if data[i] == 0 {
            i += 1;
            let count = if i < data.len() { (data[i] as usize).max(1) } else { 1 };
            // zeros are already written (vec initialized to 0)
            next += count;
        } else {
            output[next] = data[i];
            next += 1;
        }
        i += 1;
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_decode_with_runs_and_literals() {
        // 3 literal, 0x00 0x02 = 2 zeros, 1 literal
        let encoded = [5, 10, 15, 0, 2, 20];
        let result = decode_rle(&encoded, 6);
        assert_eq!(result, [5, 10, 15, 0, 0, 20]);
    }

    #[test]
    fn parse_minimal_spr_v2_1_with_rle() {
        let mut data = Vec::new();
        data.extend_from_slice(b"SP");
        data.push(1); // minor = 1
        data.push(2); // major = 2 → version 2.1
        data.extend_from_slice(&1u16.to_le_bytes()); // 1 palette image
        data.extend_from_slice(&0u16.to_le_bytes()); // 0 rgba images
        // palette image: 2x2 = 4 pixels
        data.extend_from_slice(&2u16.to_le_bytes()); // width
        data.extend_from_slice(&2u16.to_le_bytes()); // height
        // RLE: [1, 2, 0, 2] → [1, 2, 0, 0]
        let encoded: &[u8] = &[1, 2, 0, 2];
        data.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
        data.extend_from_slice(encoded);
        // palette at end
        let mut palette_data = [0u8; 1024];
        palette_data[4..8].copy_from_slice(&[255, 0, 0, 0]);
        data.extend_from_slice(&palette_data);

        let spr = SprFile::parse(&data).unwrap();
        assert_eq!(spr.version, (2, 1));
        assert_eq!(spr.indexed_sprites.len(), 1);
        assert_eq!(spr.indexed_sprites[0].data, [1, 2, 0, 0]);
        assert!(spr.palette.is_some());
        assert_eq!(spr.palette.unwrap()[1], [255, 0, 0, 0]);
    }
}
