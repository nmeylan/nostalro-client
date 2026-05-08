use std::io::{Cursor, Read};

use byteorder::{LittleEndian as LE, ReadBytesExt};
use models::enums::EnumWithMaskValueU16;
use models::enums::cell::CellType;

use crate::FormatError;

pub struct GatCell {
    pub height_sw: f32,
    pub height_se: f32,
    pub height_nw: f32,
    pub height_ne: f32,
    pub cell_flags: u16,
}

impl GatCell {
    pub fn is_walkable(&self) -> bool {
        self.cell_flags & CellType::Walkable.as_flag() != 0
    }

    pub fn is_water(&self) -> bool {
        self.cell_flags & CellType::Water.as_flag() != 0
    }

    pub fn is_shootable(&self) -> bool {
        self.cell_flags & CellType::Shootable.as_flag() != 0
    }

    pub fn interpolate_height(&self, fx: f32, fy: f32) -> f32 {
        let top = self.height_sw * (1.0 - fx) + self.height_se * fx;
        let bot = self.height_nw * (1.0 - fx) + self.height_ne * fx;
        top * (1.0 - fy) + bot * fy
    }
}

/// Maps raw GAT cell type values to server-compatible cell flags.
fn raw_cell_to_flags(raw: i32) -> u16 {
    match raw {
        0 | 2 | 4 | 6 => CellType::Walkable.as_flag() | CellType::Shootable.as_flag(),
        3 => {
            CellType::Walkable.as_flag() | CellType::Shootable.as_flag() | CellType::Water.as_flag()
        }
        5 => CellType::Shootable.as_flag(),
        _ => 0,
    }
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
        let (version, width, height) = Self::parse_header(&mut r)?;
        let cells = Self::parse_cells(&mut r, width, height)?;
        Ok(GatFile {
            version,
            width,
            height,
            cells,
        })
    }

    fn parse_header(r: &mut Cursor<&[u8]>) -> Result<((u8, u8), i32, i32), FormatError> {
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != b"GRAT" {
            return Err(FormatError::InvalidMagic);
        }
        let ver_major = r.read_u8()?;
        let ver_minor = r.read_u8()?;
        let width = r.read_i32::<LE>()?;
        let height = r.read_i32::<LE>()?;
        Ok(((ver_major, ver_minor), width, height))
    }

    fn parse_cells(
        r: &mut Cursor<&[u8]>,
        width: i32,
        height: i32,
    ) -> Result<Vec<GatCell>, FormatError> {
        let cell_count = (width as usize) * (height as usize);
        let mut cells = Vec::with_capacity(cell_count);
        for _ in 0..cell_count {
            cells.push(GatCell {
                height_sw: r.read_f32::<LE>()?,
                height_se: r.read_f32::<LE>()?,
                height_nw: r.read_f32::<LE>()?,
                height_ne: r.read_f32::<LE>()?,
                cell_flags: raw_cell_to_flags(r.read_i32::<LE>()?),
            });
        }
        Ok(cells)
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return false;
        }
        self.cells[(y * self.width + x) as usize].is_walkable()
    }

    pub fn get_height(&self, x: f32, y: f32) -> f32 {
        let cx = x as i32;
        let cy = y as i32;
        if cx < 0 || cy < 0 || cx >= self.width || cy >= self.height {
            return 0.0;
        }
        self.cells[(cy * self.width + cx) as usize].interpolate_height(x.fract(), y.fract())
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
            (0.0, 2.0, 4.0, 6.0, 0),     // (0,0) walkable ground
            (1.0, 1.0, 1.0, 1.0, 1),     // (1,0) unwalkable
            (10.0, 10.0, 10.0, 10.0, 3), // (0,1) walkable water
            (5.0, 5.0, 5.0, 5.0, 5),     // (1,1) shootable cliff
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

        // Water flags
        assert!(!gat.cells[0].is_water());
        assert!(gat.cells[2].is_water());

        // Shootable flags
        assert!(gat.cells[0].is_shootable());
        assert!(!gat.cells[1].is_shootable());
        assert!(gat.cells[3].is_shootable());
        assert!(!gat.cells[3].is_walkable());

        // Height at cell corners
        assert_eq!(gat.get_height(0.0, 0.0), 0.0);
        // Flat cell should return constant
        assert_eq!(gat.get_height(1.0, 0.0), 1.0);
    }
}
