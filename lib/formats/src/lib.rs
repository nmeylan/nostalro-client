pub mod act;
pub mod builtin_accessory_table;
pub mod builtin_name_table;
pub mod fog_table;
pub mod gat;
pub mod gnd;
pub mod gr2;
pub mod grf;
pub mod imf;
pub mod lua_table;
pub mod pettalk;
mod mixcrypt;
pub mod pal;
pub mod rsm;
pub mod rsw;
pub mod spr;
pub mod str_effect;

use std::io::{Cursor, Read, Write};

use byteorder::{LittleEndian as LE, ReadBytesExt};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Mat3 = [[f32; 3]; 3];
pub type Color = [u8; 4];

#[derive(Debug)]
pub enum FormatError {
    InvalidMagic,
    UnsupportedVersion(u8, u8),
    UnexpectedEof,
    DecompressionFailed(String),
    InvalidString,
    ReadOnly,
    Io(std::io::Error),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::InvalidMagic => write!(f, "invalid magic signature"),
            FormatError::UnsupportedVersion(maj, min) => {
                write!(f, "unsupported version {maj}.{min}")
            }
            FormatError::UnexpectedEof => write!(f, "unexpected end of file"),
            FormatError::DecompressionFailed(msg) => write!(f, "decompression failed: {msg}"),
            FormatError::InvalidString => write!(f, "invalid string encoding"),
            FormatError::ReadOnly => write!(f, "archive opened as read-only"),
            FormatError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for FormatError {}

impl From<std::io::Error> for FormatError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            FormatError::UnexpectedEof
        } else {
            FormatError::Io(e)
        }
    }
}

/// RO convention: magenta pixels (FF00FF) represent transparency in BMP textures.
pub fn apply_magenta_transparency(rgba_data: &mut [u8]) {
    for pixel in rgba_data.chunks_exact_mut(4) {
        if pixel[0] >= 0xFE && pixel[1] <= 0x01 && pixel[2] >= 0xFE {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 0;
        }
    }
}

pub fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    if encoder.write_all(data).is_err() {
        return Vec::new();
    }
    encoder.finish().unwrap_or_default()
}

pub fn zlib_decompress(data: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).ok()?;
    Some(out)
}

pub(crate) fn read_string(cursor: &mut Cursor<&[u8]>, len: usize) -> Result<String, FormatError> {
    let mut buf = vec![0u8; len];
    cursor.read_exact(&mut buf)?;
    let trimmed = buf.split(|&b| b == 0).next().unwrap_or(&buf);
    let (decoded, _, had_errors) = encoding_rs::EUC_KR.decode(trimmed);
    if had_errors {
        return Err(FormatError::InvalidString);
    }
    Ok(decoded.into_owned())
}

#[allow(dead_code)]
pub(crate) fn read_string_lossy(
    cursor: &mut Cursor<&[u8]>,
    len: usize,
) -> Result<String, FormatError> {
    let mut buf = vec![0u8; len];
    cursor.read_exact(&mut buf)?;
    let trimmed = buf.split(|&b| b == 0).next().unwrap_or(&buf);
    let (decoded, _, _) = encoding_rs::EUC_KR.decode(trimmed);
    Ok(decoded.into_owned())
}

pub(crate) fn read_length_string(cursor: &mut Cursor<&[u8]>) -> Result<String, FormatError> {
    let len = cursor.read_u32::<LE>()? as usize;
    read_string(cursor, len)
}

pub(crate) fn read_vec3(cursor: &mut Cursor<&[u8]>) -> Result<Vec3, FormatError> {
    Ok([
        cursor.read_f32::<LE>()?,
        cursor.read_f32::<LE>()?,
        cursor.read_f32::<LE>()?,
    ])
}

pub(crate) fn version_at_least(version: (u8, u8), major: u8, minor: u8) -> bool {
    version.0 > major || (version.0 == major && version.1 >= minor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zlib_round_trip_small_buffer() {
        let data = b"BM emblem payload \x00\x01\x02\x03 magenta".to_vec();
        let compressed = zlib_compress(&data);
        assert_ne!(compressed, data);
        assert_eq!(zlib_decompress(&compressed), Some(data));
        assert_eq!(zlib_decompress(b"not zlib data"), None);
    }
}
