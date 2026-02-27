use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::FormatError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellType {
    Walkable,
    Unwalkable,
    WaterWalkable,
    WaterUnwalkable,
    WaterSnipable,
    CliffSnipable,
    Cliff,
    Unknown(i32),
}

impl CellType {
    fn from_i32(value: i32) -> Self {
        match value {
            0 => CellType::Walkable,
            1 => CellType::Unwalkable,
            2 => CellType::WaterWalkable,
            3 => CellType::WaterUnwalkable,
            4 => CellType::WaterSnipable,
            5 => CellType::CliffSnipable,
            6 => CellType::Cliff,
            v => CellType::Unknown(v),
        }
    }

    pub fn is_walkable(&self) -> bool {
        matches!(self, CellType::Walkable | CellType::WaterWalkable)
    }
}

pub struct GatCell {
    pub heights: [f32; 4],
    pub cell_type: CellType,
}

pub struct GatFile {
    pub version: (u8, u8),
    pub width: i32,
    pub height: i32,
    pub cells: Vec<GatCell>,
}

impl GatFile {
    pub fn parse(data: &[u8]) -> Result<Self, FormatError> {
        let mut r = Cursor::new(data);

        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != b"GRAT" {
            return Err(FormatError::InvalidMagic);
        }

        let ver_major = r.read_u8()?;
        let ver_minor = r.read_u8()?;
        let width = r.read_i32::<LE>()?;
        let height = r.read_i32::<LE>()?;

        let cell_count = (width as usize) * (height as usize);
        let mut cells = Vec::with_capacity(cell_count);
        for _ in 0..cell_count {
            cells.push(GatCell {
                heights: [
                    r.read_f32::<LE>()?,
                    r.read_f32::<LE>()?,
                    r.read_f32::<LE>()?,
                    r.read_f32::<LE>()?,
                ],
                cell_type: CellType::from_i32(r.read_i32::<LE>()?),
            });
        }

        Ok(GatFile {
            version: (ver_major, ver_minor),
            width,
            height,
            cells,
        })
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return false;
        }
        self.cells[(y * self.width + x) as usize].cell_type.is_walkable()
    }

    pub fn get_height(&self, x: f32, y: f32) -> f32 {
        let cx = x as i32;
        let cy = y as i32;
        if cx < 0 || cy < 0 || cx >= self.width || cy >= self.height {
            return 0.0;
        }
        let cell = &self.cells[(cy * self.width + cx) as usize];
        let fx = x.fract();
        let fy = y.fract();
        let h = &cell.heights;
        let top = h[0] * (1.0 - fx) + h[1] * fx;
        let bot = h[2] * (1.0 - fx) + h[3] * fx;
        top * (1.0 - fy) + bot * fy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_gat_bytes(width: i32, height: i32, cells: &[(f32, f32, f32, f32, i32)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"GRAT");
        data.push(1); // major
        data.push(2); // minor
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        for &(h1, h2, h3, h4, flag) in cells {
            data.extend_from_slice(&h1.to_le_bytes());
            data.extend_from_slice(&h2.to_le_bytes());
            data.extend_from_slice(&h3.to_le_bytes());
            data.extend_from_slice(&h4.to_le_bytes());
            data.extend_from_slice(&flag.to_le_bytes());
        }
        data
    }

    #[test]
    fn parse_2x2_gat_and_check_walkability_and_height() {
        let cells = [
            (0.0, 2.0, 4.0, 6.0, 0), // (0,0) walkable
            (1.0, 1.0, 1.0, 1.0, 1), // (1,0) unwalkable
            (10.0, 10.0, 10.0, 10.0, 2), // (0,1) water walkable
            (5.0, 5.0, 5.0, 5.0, 5), // (1,1) cliff snipable
        ];
        let data = build_gat_bytes(2, 2, &cells);
        let gat = GatFile::parse(&data).unwrap();

        assert_eq!(gat.width, 2);
        assert_eq!(gat.height, 2);
        assert_eq!(gat.cells.len(), 4);

        assert!(gat.is_walkable(0, 0));
        assert!(!gat.is_walkable(1, 0));
        assert!(gat.is_walkable(0, 1));
        assert!(!gat.is_walkable(1, 1));
        assert!(!gat.is_walkable(-1, 0));
        assert!(!gat.is_walkable(0, 2));

        // Height at cell corners
        assert_eq!(gat.get_height(0.0, 0.0), 0.0);
        // Flat cell should return constant
        assert_eq!(gat.get_height(1.0, 0.0), 1.0);
    }
}
