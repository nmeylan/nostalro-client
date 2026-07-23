//! Reader for the Granny (GR2) 3D asset format used by RO for WOE guardians,
//! the Emperium, guild flags, and treasure boxes. A file is a set of
//! Oodle-compressed sections (`oodle` module) that decompress into one buffer,
//! with pointer fix-ups rewritten to absolute offsets; [`model`] then walks the
//! type tree to extract skeletons, meshes, textures, and animations.
//!
//! Format references:
//! - <https://github.com/rdw-archive/RagnarokFileFormats/blob/master/GR2.MD>
//! - <https://github.com/arves100/Granny2-research/wiki/File-Format-Documentation>

mod bink;
pub mod model;
mod oodle;
mod range_coder;

pub use bink::decode as bink_decode;
pub use model::Gr2File;

use crate::FormatError;

const HEADER_SIZE: usize = 0x20;
const SECTOR_SIZE: usize = 44;
const FIXUP_SIZE: usize = 12;
const OODLE_TAIL_PAD: usize = 4;

// The 16-byte file signature (read as four little-endian words) also encodes
// the byte order and pointer size of the file; these two are the little-endian,
// 32-bit variants for format versions 6 and 7 respectively.
const MAGIC_FF6_LE: [u32; 4] = [0xCAB0_67B8, 0x0FB1_6DF8, 0x7E8C_7284, 0x1E00_195E];
const MAGIC_FF7_LE: [u32; 4] = [0xC06C_DE29, 0x2B53_A4BA, 0xA5B7_F525, 0xEEE2_66F6];

/// A `(sector, position)` reference into the decompressed data buffer. The GR2
/// header stores its root object and type as these pairs, and pointer fix-ups
/// resolve to them.
#[derive(Clone, Copy)]
pub struct SectorRef {
    pub sector: u32,
    pub position: u32,
}

#[derive(Clone, Copy)]
pub struct SectorInfo {
    pub compress_type: u32,
    pub data_offset: u32,
    pub compressed_len: u32,
    pub decompress_len: u32,
    pub oodle_stop0: u32,
    pub oodle_stop1: u32,
    pub fixup_offset: u32,
    pub fixup_count: u32,
}

/// A parsed GR2 file: all sectors decompressed into one contiguous buffer with
/// pointer fix-ups rewritten to absolute offsets within it.
pub struct Gr2Container {
    pub version: u32,
    pub data: Vec<u8>,
    pub sector_offsets: Vec<usize>,
    pub sectors: Vec<SectorInfo>,
    pub type_ref: SectorRef,
    pub root_ref: SectorRef,
}

/// Standard reflected CRC-32 (polynomial `0xEDB88320`), used to verify the file
/// body against the checksum stored in the header.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Read a little-endian `u32` at a byte offset. Shared by the container,
/// object-graph, and texture-header parsers, which all address the buffer by
/// absolute offset rather than reading sequentially.
pub(super) fn read_u32(data: &[u8], off: usize) -> Result<u32, FormatError> {
    let bytes = data.get(off..off + 4).ok_or(FormatError::UnexpectedEof)?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

impl Gr2Container {
    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        if bytes.len() < HEADER_SIZE {
            return Err(FormatError::UnexpectedEof);
        }
        let magic = [
            read_u32(bytes, 0)?,
            read_u32(bytes, 4)?,
            read_u32(bytes, 8)?,
            read_u32(bytes, 12)?,
        ];
        if magic != MAGIC_FF6_LE && magic != MAGIC_FF7_LE {
            return Err(FormatError::InvalidMagic);
        }
        if read_u32(bytes, 0x18)? != 0 {
            return Err(FormatError::InvalidMagic);
        }

        let version = read_u32(bytes, 0x20)?;
        if version != 6 && version != 7 {
            return Err(FormatError::UnsupportedVersion(version as u8, 0));
        }
        let total_size = read_u32(bytes, 0x24)? as usize;
        let file_crc = read_u32(bytes, 0x28)?;
        let file_info_size = read_u32(bytes, 0x2c)? as usize;
        let sector_count = read_u32(bytes, 0x30)? as usize;
        let type_ref = SectorRef {
            sector: read_u32(bytes, 0x34)?,
            position: read_u32(bytes, 0x38)?,
        };
        let root_ref = SectorRef {
            sector: read_u32(bytes, 0x3c)?,
            position: read_u32(bytes, 0x40)?,
        };

        if total_size != bytes.len() {
            return Err(FormatError::DecompressionFailed(
                "gr2: size mismatch".into(),
            ));
        }

        let crc_start = HEADER_SIZE + file_info_size;
        let crc = crc32(bytes.get(crc_start..).ok_or(FormatError::UnexpectedEof)?);
        if crc != file_crc {
            return Err(FormatError::DecompressionFailed("gr2: crc mismatch".into()));
        }

        let mut sectors = Vec::with_capacity(sector_count);
        let sector_table = HEADER_SIZE + file_info_size;
        for i in 0..sector_count {
            let base = sector_table + i * SECTOR_SIZE;
            sectors.push(SectorInfo {
                compress_type: read_u32(bytes, base)?,
                data_offset: read_u32(bytes, base + 4)?,
                compressed_len: read_u32(bytes, base + 8)?,
                decompress_len: read_u32(bytes, base + 12)?,
                oodle_stop0: read_u32(bytes, base + 20)?,
                oodle_stop1: read_u32(bytes, base + 24)?,
                fixup_offset: read_u32(bytes, base + 28)?,
                fixup_count: read_u32(bytes, base + 32)?,
            });
        }

        let total: usize = sectors.iter().map(|s| s.decompress_len as usize).sum();
        let mut data = vec![0u8; total];
        let mut sector_offsets = Vec::with_capacity(sector_count);
        let mut ofs = 0usize;

        // Decompress each sector into its slice of the contiguous output buffer.
        for s in &sectors {
            sector_offsets.push(ofs);
            let dst = &mut data[ofs..ofs + s.decompress_len as usize];
            let src_start = s.data_offset as usize;
            if s.compress_type == 0 {
                let src = bytes
                    .get(src_start..src_start + s.decompress_len as usize)
                    .ok_or(FormatError::UnexpectedEof)?;
                dst.copy_from_slice(src);
            } else {
                let src = bytes
                    .get(src_start..src_start + s.compressed_len as usize)
                    .ok_or(FormatError::UnexpectedEof)?;
                // The decoder may read a few bytes past the compressed input, so
                // pad the tail with zeros (which decode as a graceful stop).
                let mut compressed = src.to_vec();
                compressed.resize(src.len() + OODLE_TAIL_PAD, 0);
                oodle::decompress(&compressed, dst, s.oodle_stop0, s.oodle_stop1)?;
            }
            ofs += s.decompress_len as usize;
        }

        apply_fixups(bytes, &mut data, &sector_offsets, &sectors)?;

        Ok(Gr2Container {
            version,
            data,
            sector_offsets,
            sectors,
            type_ref,
            root_ref,
        })
    }

    /// Absolute offset of a sector reference within `data`.
    pub fn ref_offset(&self, r: SectorRef) -> usize {
        self.sector_offsets[r.sector as usize] + r.position as usize
    }
}

fn apply_fixups(
    bytes: &[u8],
    data: &mut [u8],
    sector_offsets: &[usize],
    sectors: &[SectorInfo],
) -> Result<(), FormatError> {
    for (i, s) in sectors.iter().enumerate() {
        for k in 0..s.fixup_count as usize {
            let base = s.fixup_offset as usize + k * FIXUP_SIZE;
            let src_offset = read_u32(bytes, base)? as usize;
            let dst_sector = read_u32(bytes, base + 4)? as usize;
            let dst_offset = read_u32(bytes, base + 8)? as usize;
            if dst_sector >= sector_offsets.len() {
                return Err(FormatError::DecompressionFailed(
                    "gr2: bad fixup sector".into(),
                ));
            }
            let target = (sector_offsets[dst_sector] + dst_offset) as u32;
            let at = sector_offsets[i] + src_offset;
            data.get_mut(at..at + 4)
                .ok_or(FormatError::UnexpectedEof)?
                .copy_from_slice(&target.to_le_bytes());
        }
    }
    Ok(())
}
