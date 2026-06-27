use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::act::ActFile;
use crate::{Color, FormatError, version_at_least};

pub struct SpriteData {
    pub images: Vec<RgbaImageData>,
    pub indexed_count: usize,
    pub act: ActFile,
}

pub struct SprImage {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

pub struct RgbaImageData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl SprImage {
    pub fn apply_palette(&self, palette: &[Color; 256]) -> RgbaImageData {
        let w = self.width as u32;
        let h = self.height as u32;
        let mut data = vec![0u8; (w * h * 4) as usize];
        for (i, &index) in self.data.iter().enumerate() {
            if index == 0 {
                continue;
            }
            let c = palette[index as usize];
            if c[0] >= 0xFE && c[1] <= 0x01 && c[2] >= 0xFE {
                continue;
            }
            let offset = i * 4;
            data[offset] = c[0];
            data[offset + 1] = c[1];
            data[offset + 2] = c[2];
            data[offset + 3] = 255;
        }
        RgbaImageData {
            width: w,
            height: h,
            data,
        }
    }

    pub fn swizzle_pixel_bytes(&self) -> RgbaImageData {
        let w = self.width as u32;
        let h = self.height as u32;
        let mut data = vec![0u8; (w * h * 4) as usize];
        for y in 0..h as usize {
            let dst_y = (h as usize - 1) - y;
            for x in 0..w as usize {
                let src = (y * w as usize + x) * 4;
                let dst = (dst_y * w as usize + x) * 4;
                data[dst] = self.data[src + 3];
                data[dst + 1] = self.data[src + 2];
                data[dst + 2] = self.data[src + 1];
                data[dst + 3] = self.data[src];
            }
        }
        RgbaImageData {
            width: w,
            height: h,
            data,
        }
    }
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
                decompress_indexed(&encoded, pixel_count)
            } else {
                let mut raw = vec![0u8; pixel_count];
                r.read_exact(&mut raw)?;
                raw
            };

            indexed_sprites.push(SprImage {
                width,
                height,
                data,
            });
        }

        let mut rgba_sprites = Vec::with_capacity(rgba_image_count);
        for _ in 0..rgba_image_count {
            let width = r.read_u16::<LE>()?;
            let height = r.read_u16::<LE>()?;
            let byte_count = width as usize * height as usize * 4;
            let mut data = vec![0u8; byte_count];
            r.read_exact(&mut data)?;
            rgba_sprites.push(SprImage {
                width,
                height,
                data,
            });
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

    pub fn to_rgba_images(&self) -> (Vec<RgbaImageData>, usize) {
        self.to_rgba_images_with_palette(None)
    }

    pub fn to_rgba_images_with_palette(
        &self,
        override_palette: Option<&[Color; 256]>,
    ) -> (Vec<RgbaImageData>, usize) {
        let palette = override_palette
            .or(self.palette.as_ref())
            .expect("SPR file has no palette");
        let mut images = Vec::with_capacity(self.indexed_sprites.len() + self.rgba_sprites.len());
        for sprite in &self.indexed_sprites {
            images.push(sprite.apply_palette(palette));
        }
        let indexed_count = self.indexed_sprites.len();
        for sprite in &self.rgba_sprites {
            images.push(sprite.swizzle_pixel_bytes());
        }
        (images, indexed_count)
    }
}

fn decompress_indexed(data: &[u8], pixel_count: usize) -> Vec<u8> {
    let mut output = vec![0u8; pixel_count];
    let mut i = 0;
    let mut next = 0;

    while i < data.len() && next < pixel_count {
        if data[i] == 0 {
            i += 1;
            let count = if i < data.len() {
                (data[i] as usize).max(1)
            } else {
                1
            };
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
        let encoded = [5, 10, 15, 0, 2, 20];
        let result = decompress_indexed(&encoded, 6);
        assert_eq!(result, [5, 10, 15, 0, 0, 20]);
    }

    #[test]
    fn apply_palette_transparent_and_magenta() {
        let sprite = SprImage {
            width: 3,
            height: 1,
            data: vec![0, 1, 2],
        };
        let mut palette = [[0u8; 4]; 256];
        palette[1] = [255, 0, 0, 0];
        palette[2] = [255, 0, 255, 0];
        let img = sprite.apply_palette(&palette);
        assert_eq!(img.width, 3);
        assert_eq!(img.height, 1);
        assert_eq!(&img.data[0..4], &[0, 0, 0, 0]);
        assert_eq!(&img.data[4..8], &[255, 0, 0, 255]);
        assert_eq!(&img.data[8..12], &[0, 0, 0, 0]);
    }

    #[test]
    fn swizzle_pixel_bytes_swizzle() {
        let sprite = SprImage {
            width: 1,
            height: 1,
            data: vec![200, 50, 100, 150],
        };
        let img = sprite.swizzle_pixel_bytes();
        assert_eq!(&img.data, &[150, 100, 50, 200]);
    }

    #[test]
    fn swizzle_flips_rows_vertically() {
        let sprite = SprImage {
            width: 2,
            height: 2,
            data: vec![
                255, 10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120,
            ],
        };
        let img = sprite.swizzle_pixel_bytes();
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 2);
        assert_eq!(&img.data[0..4], &[90, 80, 70, 255]);
        assert_eq!(&img.data[4..8], &[120, 110, 100, 255]);
        assert_eq!(&img.data[8..12], &[30, 20, 10, 255]);
        assert_eq!(&img.data[12..16], &[60, 50, 40, 255]);
    }

    #[test]
    fn to_rgba_images_converts_all_sprites() {
        let mut data = Vec::new();
        data.extend_from_slice(b"SP");
        data.push(1);
        data.push(2);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        let encoded: &[u8] = &[1, 2];
        data.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
        data.extend_from_slice(encoded);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&[255, 50, 100, 150]);
        let mut palette_data = [0u8; 1024];
        palette_data[4..8].copy_from_slice(&[255, 0, 0, 0]);
        palette_data[8..12].copy_from_slice(&[0, 255, 0, 0]);
        data.extend_from_slice(&palette_data);

        let spr = SprFile::parse(&data).unwrap();
        let (images, indexed_count) = spr.to_rgba_images();
        assert_eq!(indexed_count, 1);
        assert_eq!(images.len(), 2);
        assert_eq!((images[0].width, images[0].height), (2, 1));
        assert_eq!((images[1].width, images[1].height), (1, 1));
        assert_eq!(&images[1].data, &[150, 100, 50, 255]);
    }

    #[test]
    fn override_palette_changes_indexed_sprite_colors() {
        let mut data = Vec::new();
        data.extend_from_slice(b"SP");
        data.push(1);
        data.push(2);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        let encoded: &[u8] = &[1];
        data.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
        data.extend_from_slice(encoded);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&[255, 50, 100, 150]);
        let mut palette_data = [0u8; 1024];
        palette_data[4..8].copy_from_slice(&[255, 0, 0, 0]);
        data.extend_from_slice(&palette_data);

        let spr = SprFile::parse(&data).unwrap();

        let (images, _) = spr.to_rgba_images();
        assert_eq!(&images[0].data[0..4], &[255, 0, 0, 255]);

        let mut override_pal = [[0u8; 4]; 256];
        override_pal[1] = [0, 0, 255, 0];
        let (images, _) = spr.to_rgba_images_with_palette(Some(&override_pal));
        assert_eq!(&images[0].data[0..4], &[0, 0, 255, 255]);
        assert_eq!(&images[1].data, &[150, 100, 50, 255]);
    }

    #[test]
    fn parse_minimal_spr_v2_1_with_rle() {
        let mut data = Vec::new();
        data.extend_from_slice(b"SP");
        data.push(1);
        data.push(2);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        let encoded: &[u8] = &[1, 2, 0, 2];
        data.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
        data.extend_from_slice(encoded);
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
