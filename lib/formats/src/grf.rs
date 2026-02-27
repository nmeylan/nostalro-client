use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use flate2::read::ZlibDecoder;

use crate::FormatError;
use crate::mixcrypt;

const HEADER_SIZE: usize = 46;
const FILE_OFFSET: usize = 7;

struct GrfEntry {
    compressed_size: u32,
    compressed_size_aligned: u32,
    uncompressed_size: u32,
    flags: u8,
    offset: u32,
}

pub struct GrfArchive {
    entries: HashMap<String, GrfEntry>,
    file: Mutex<File>,
}

impl GrfArchive {
    pub fn open(path: &Path) -> Result<Self, FormatError> {
        let mut file = File::open(path).map_err(FormatError::Io)?;

        // Read header
        let mut header_buf = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_buf)?;

        if &header_buf[..16] != b"Master of Magic\0" {
            return Err(FormatError::InvalidMagic);
        }
        // Skip encryption key (14 bytes at offset 16)
        let file_table_offset = u32::from_le_bytes(header_buf[30..34].try_into().unwrap());
        let reserved_files = u32::from_le_bytes(header_buf[34..38].try_into().unwrap());
        let file_count = u32::from_le_bytes(header_buf[38..42].try_into().unwrap());
        let version = u32::from_le_bytes(header_buf[42..46].try_into().unwrap());

        if version != 0x200 {
            return Err(FormatError::UnsupportedVersion((version >> 8) as u8, version as u8));
        }

        let actual_file_count = (file_count - reserved_files) as usize - FILE_OFFSET;

        // Seek to file table
        file.seek(SeekFrom::Current(file_table_offset as i64))?;

        // Read asset table header (compressed_size, uncompressed_size)
        let table_compressed_size = read_u32_le(&mut file)?;
        let table_uncompressed_size = read_u32_le(&mut file)?;

        // Read and decompress file table
        let mut compressed_table = vec![0u8; table_compressed_size as usize];
        file.read_exact(&mut compressed_table)?;

        let mut decompressed = Vec::with_capacity(table_uncompressed_size as usize);
        let mut decoder = ZlibDecoder::new(compressed_table.as_slice());
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| FormatError::DecompressionFailed(e.to_string()))?;

        // Parse file entries
        let mut entries = HashMap::with_capacity(actual_file_count);
        let mut pos = 0;
        for _ in 0..actual_file_count {
            // Filename: null-terminated string
            let name_end = decompressed[pos..]
                .iter()
                .position(|&b| b == 0)
                .ok_or(FormatError::UnexpectedEof)?;
            let name_bytes = &decompressed[pos..pos + name_end];
            let (name_decoded, _, _) = encoding_rs::EUC_KR.decode(name_bytes);
            let name = name_decoded.into_owned().to_lowercase();
            // Replace backslashes with forward slashes
            let name = name.replace('\\', "/");
            pos += name_end + 1;

            if pos + 17 > decompressed.len() {
                return Err(FormatError::UnexpectedEof);
            }

            let compressed_size = u32::from_le_bytes(decompressed[pos..pos + 4].try_into().unwrap());
            let compressed_size_aligned = u32::from_le_bytes(decompressed[pos + 4..pos + 8].try_into().unwrap());
            let uncompressed_size = u32::from_le_bytes(decompressed[pos + 8..pos + 12].try_into().unwrap());
            let flags = decompressed[pos + 12];
            let offset = u32::from_le_bytes(decompressed[pos + 13..pos + 17].try_into().unwrap());
            pos += 17;

            entries.insert(
                name,
                GrfEntry {
                    compressed_size,
                    compressed_size_aligned,
                    uncompressed_size,
                    flags,
                    offset,
                },
            );
        }

        Ok(GrfArchive {
            entries,
            file: Mutex::new(file),
        })
    }

    pub fn read_file(&self, name: &str) -> Result<Vec<u8>, FormatError> {
        let name_lower = name.to_lowercase().replace('\\', "/");
        let entry = self
            .entries
            .get(&name_lower)
            .ok_or(FormatError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("file not found in archive: {name}"),
            )))?;

        let position = entry.offset as u64 + HEADER_SIZE as u64;
        let mut compressed = vec![0u8; entry.compressed_size_aligned as usize];

        {
            let mut file = self.file.lock().unwrap();
            file.seek(SeekFrom::Start(position))?;
            file.read_exact(&mut compressed)?;
        }

        mixcrypt::decrypt_file(entry.flags, entry.compressed_size, &mut compressed);

        let mut decompressed = Vec::with_capacity(entry.uncompressed_size as usize);
        let mut decoder = ZlibDecoder::new(compressed.as_slice());
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| FormatError::DecompressionFailed(e.to_string()))?;

        Ok(decompressed)
    }

    pub fn file_exists(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase().replace('\\', "/");
        self.entries.contains_key(&name_lower)
    }

    pub fn file_count(&self) -> usize {
        self.entries.len()
    }

    pub fn find_first_with_extension(&self, extension: &str) -> Option<&str> {
        let ext = extension.to_lowercase();
        self.entries.keys()
            .find(|name| name.ends_with(&ext))
            .map(|s| s.as_str())
    }

    pub fn files_with_extension(&self, extension: &str) -> Vec<&str> {
        let ext = extension.to_lowercase();
        self.entries.keys()
            .filter(|name| name.ends_with(&ext))
            .map(|s| s.as_str())
            .collect()
    }
}

fn read_u32_le(file: &mut File) -> Result<u32, FormatError> {
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}
