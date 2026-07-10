use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

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

pub struct GrfFileInfo {
    pub name: String,
    pub compressed_size: u32,
    pub compressed_size_aligned: u32,
    pub uncompressed_size: u32,
    pub flags: u8,
}

pub struct GrfArchive {
    entries: HashMap<String, GrfEntry>,
    file: Mutex<File>,
    path: PathBuf,
    file_data_end_offset: u64,
    writable: bool,
}

fn parse_grf(file: &mut File) -> Result<(HashMap<String, GrfEntry>, u64), FormatError> {
    let mut header_buf = [0u8; HEADER_SIZE];
    file.read_exact(&mut header_buf)?;

    if &header_buf[..16] != b"Master of Magic\0" {
        return Err(FormatError::InvalidMagic);
    }

    let file_table_offset = u32::from_le_bytes(header_buf[30..34].try_into().unwrap());
    let reserved_files = u32::from_le_bytes(header_buf[34..38].try_into().unwrap());
    let file_count = u32::from_le_bytes(header_buf[38..42].try_into().unwrap());
    let version = u32::from_le_bytes(header_buf[42..46].try_into().unwrap());

    if !(0x100..=0x200).contains(&version) {
        return Err(FormatError::UnsupportedVersion(
            (version >> 8) as u8,
            version as u8,
        ));
    }

    let file_data_end_offset = HEADER_SIZE as u64 + file_table_offset as u64;
    let actual_file_count = (file_count - reserved_files) as usize - FILE_OFFSET;

    file.seek(SeekFrom::Start(file_data_end_offset))?;

    let entries = if version == 0x200 {
        parse_v2_entries(file, actual_file_count)?
    } else {
        parse_v1_entries(file, actual_file_count)?
    };

    Ok((entries, file_data_end_offset))
}

impl GrfArchive {
    pub fn open(path: &Path) -> Result<Self, FormatError> {
        let mut file = File::open(path).map_err(FormatError::Io)?;
        let (entries, file_data_end_offset) = parse_grf(&mut file)?;

        Ok(GrfArchive {
            entries,
            file: Mutex::new(file),
            path: path.to_path_buf(),
            file_data_end_offset,
            writable: false,
        })
    }

    pub fn open_rw(path: &Path) -> Result<Self, FormatError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(FormatError::Io)?;
        let (entries, file_data_end_offset) = parse_grf(&mut file)?;

        Ok(GrfArchive {
            entries,
            file: Mutex::new(file),
            path: path.to_path_buf(),
            file_data_end_offset,
            writable: true,
        })
    }

    pub fn create(path: &Path) -> Result<Self, FormatError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(FormatError::Io)?;

        let mut header = [0u8; HEADER_SIZE];
        header[..16].copy_from_slice(b"Master of Magic\0");
        header[30..34].copy_from_slice(&0u32.to_le_bytes());
        header[34..38].copy_from_slice(&0u32.to_le_bytes());
        header[38..42].copy_from_slice(&(FILE_OFFSET as u32).to_le_bytes());
        header[42..46].copy_from_slice(&0x200u32.to_le_bytes());
        file.write_all(&header)?;

        let empty_table = Vec::new();
        let compressed = zlib_compress(&empty_table)?;
        file.write_all(&(compressed.len() as u32).to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?;
        file.write_all(&compressed)?;
        file.flush()?;

        Ok(GrfArchive {
            entries: HashMap::new(),
            file: Mutex::new(file),
            path: path.to_path_buf(),
            file_data_end_offset: HEADER_SIZE as u64,
            writable: true,
        })
    }

    pub fn is_writable(&self) -> bool {
        self.writable
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

    pub fn add_file(&mut self, name: &str, data: &[u8]) -> Result<(), FormatError> {
        if !self.writable {
            return Err(FormatError::ReadOnly);
        }

        let name_lower = name.to_lowercase().replace('\\', "/");
        let compressed = zlib_compress(data)?;
        let compressed_len = compressed.len() as u32;

        let file = self.file.get_mut().unwrap();
        file.seek(SeekFrom::Start(self.file_data_end_offset))?;
        file.write_all(&compressed)?;

        let entry = GrfEntry {
            compressed_size: compressed_len,
            compressed_size_aligned: compressed_len,
            uncompressed_size: data.len() as u32,
            flags: 0x01,
            offset: (self.file_data_end_offset - HEADER_SIZE as u64) as u32,
        };

        self.file_data_end_offset += compressed_len as u64;
        self.entries.insert(name_lower, entry);
        Ok(())
    }

    pub fn remove_file(&mut self, name: &str) -> Result<bool, FormatError> {
        if !self.writable {
            return Err(FormatError::ReadOnly);
        }
        let name_lower = name.to_lowercase().replace('\\', "/");
        Ok(self.entries.remove(&name_lower).is_some())
    }

    pub fn save(&mut self) -> Result<(), FormatError> {
        if !self.writable {
            return Err(FormatError::ReadOnly);
        }

        let table_raw = build_file_table(&self.entries);
        let table_compressed = zlib_compress(&table_raw)?;

        let file = self.file.get_mut().unwrap();
        file.seek(SeekFrom::Start(self.file_data_end_offset))?;
        file.write_all(&(table_compressed.len() as u32).to_le_bytes())?;
        file.write_all(&(table_raw.len() as u32).to_le_bytes())?;
        file.write_all(&table_compressed)?;

        let end_pos = self.file_data_end_offset + 8 + table_compressed.len() as u64;
        file.set_len(end_pos)?;

        let file_table_offset = (self.file_data_end_offset - HEADER_SIZE as u64) as u32;
        let file_count = (self.entries.len() + FILE_OFFSET) as u32;

        file.seek(SeekFrom::Start(30))?;
        file.write_all(&file_table_offset.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?;
        file.write_all(&file_count.to_le_bytes())?;
        file.flush()?;

        Ok(())
    }

    pub fn repack(&mut self) -> Result<(), FormatError> {
        if !self.writable {
            return Err(FormatError::ReadOnly);
        }

        let mut blobs: Vec<(String, GrfEntry, Vec<u8>)> = Vec::with_capacity(self.entries.len());
        {
            let file = self.file.get_mut().unwrap();
            for (name, entry) in &self.entries {
                let position = entry.offset as u64 + HEADER_SIZE as u64;
                let mut blob = vec![0u8; entry.compressed_size_aligned as usize];
                file.seek(SeekFrom::Start(position))?;
                file.read_exact(&mut blob)?;
                blobs.push((
                    name.clone(),
                    GrfEntry {
                        compressed_size: entry.compressed_size,
                        compressed_size_aligned: entry.compressed_size_aligned,
                        uncompressed_size: entry.uncompressed_size,
                        flags: entry.flags,
                        offset: 0,
                    },
                    blob,
                ));
            }
        }

        let file = self.file.get_mut().unwrap();
        let mut write_pos = HEADER_SIZE as u64;
        self.entries.clear();

        for (name, mut entry, blob) in blobs {
            file.seek(SeekFrom::Start(write_pos))?;
            file.write_all(&blob)?;
            entry.offset = (write_pos - HEADER_SIZE as u64) as u32;
            write_pos += blob.len() as u64;
            self.entries.insert(name, entry);
        }

        self.file_data_end_offset = write_pos;
        self.save()
    }

    pub fn file_exists(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase().replace('\\', "/");
        self.entries.contains_key(&name_lower)
    }

    pub fn file_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entry_names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn find_first_with_extension(&self, extension: &str) -> Option<&str> {
        let ext = extension.to_lowercase();
        self.entries
            .keys()
            .find(|name| name.ends_with(&ext))
            .map(|s| s.as_str())
    }

    pub fn files_with_extension(&self, extension: &str) -> Vec<&str> {
        let ext = extension.to_lowercase();
        self.entries
            .keys()
            .filter(|name| name.ends_with(&ext))
            .map(|s| s.as_str())
            .collect()
    }

    pub fn file_list(&self) -> Vec<GrfFileInfo> {
        let mut list: Vec<GrfFileInfo> = self
            .entries
            .iter()
            .map(|(name, entry)| GrfFileInfo {
                name: name.clone(),
                compressed_size: entry.compressed_size,
                compressed_size_aligned: entry.compressed_size_aligned,
                uncompressed_size: entry.uncompressed_size,
                flags: entry.flags,
            })
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub fn file_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.entries.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn zlib_compress(data: &[u8]) -> Result<Vec<u8>, FormatError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish().map_err(FormatError::Io)
}

fn build_file_table(entries: &HashMap<String, GrfEntry>) -> Vec<u8> {
    let mut table = Vec::new();
    for (name, entry) in entries {
        let backslash_name = name.replace('/', "\\");
        let (encoded, _, _) = encoding_rs::EUC_KR.encode(&backslash_name);
        table.extend_from_slice(&encoded);
        table.push(0);
        table.extend_from_slice(&entry.compressed_size.to_le_bytes());
        table.extend_from_slice(&entry.compressed_size_aligned.to_le_bytes());
        table.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
        table.push(entry.flags);
        table.extend_from_slice(&entry.offset.to_le_bytes());
    }
    table
}

fn read_u32_le(file: &mut File) -> Result<u32, FormatError> {
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn parse_v2_entries(
    file: &mut File,
    file_count: usize,
) -> Result<HashMap<String, GrfEntry>, FormatError> {
    let table_compressed_size = read_u32_le(file)?;
    let table_uncompressed_size = read_u32_le(file)?;

    let mut compressed_table = vec![0u8; table_compressed_size as usize];
    file.read_exact(&mut compressed_table)?;

    let mut table = Vec::with_capacity(table_uncompressed_size as usize);
    let mut decoder = ZlibDecoder::new(compressed_table.as_slice());
    decoder
        .read_to_end(&mut table)
        .map_err(|e| FormatError::DecompressionFailed(e.to_string()))?;

    let mut entries = HashMap::with_capacity(file_count);
    let mut pos = 0;
    for _ in 0..file_count {
        let name_end = table[pos..]
            .iter()
            .position(|&b| b == 0)
            .ok_or(FormatError::UnexpectedEof)?;
        let name_bytes = &table[pos..pos + name_end];
        let (name_decoded, _, _) = encoding_rs::EUC_KR.decode(name_bytes);
        let name = name_decoded.into_owned().to_lowercase().replace('\\', "/");
        pos += name_end + 1;

        if pos + 17 > table.len() {
            return Err(FormatError::UnexpectedEof);
        }

        let compressed_size = u32::from_le_bytes(table[pos..pos + 4].try_into().unwrap());
        let compressed_size_aligned =
            u32::from_le_bytes(table[pos + 4..pos + 8].try_into().unwrap());
        let uncompressed_size = u32::from_le_bytes(table[pos + 8..pos + 12].try_into().unwrap());
        let flags = table[pos + 12];
        let offset = u32::from_le_bytes(table[pos + 13..pos + 17].try_into().unwrap());
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
    Ok(entries)
}

fn parse_v1_entries(
    file: &mut File,
    file_count: usize,
) -> Result<HashMap<String, GrfEntry>, FormatError> {
    let mut table = Vec::new();
    file.read_to_end(&mut table)?;

    let mut entries = HashMap::with_capacity(file_count);
    let mut ofs = 0;
    for _ in 0..file_count {
        if ofs + 6 > table.len() {
            return Err(FormatError::UnexpectedEof);
        }
        let entry_len = u32::from_le_bytes(table[ofs..ofs + 4].try_into().unwrap()) as usize;
        let name_len = (entry_len as u8).wrapping_sub(6) as usize;
        if ofs + 6 + name_len > table.len() {
            return Err(FormatError::UnexpectedEof);
        }
        let decrypt_len = (name_len + 7) & !7;
        let mut name_buf = vec![0u8; decrypt_len];
        let copy_len = name_len.min(table.len() - (ofs + 6));
        name_buf[..copy_len].copy_from_slice(&table[ofs + 6..ofs + 6 + copy_len]);
        mixcrypt::decrypt_filename(&mut name_buf);

        let name_end = name_buf.iter().position(|&b| b == 0).unwrap_or(name_len);
        let (name_decoded, _, _) = encoding_rs::EUC_KR.decode(&name_buf[..name_end]);
        let name = name_decoded.into_owned().to_lowercase().replace('\\', "/");

        let ofs2 = ofs + entry_len + 4;
        if ofs2 + 17 > table.len() {
            return Err(FormatError::UnexpectedEof);
        }

        let raw_compressed = u32::from_le_bytes(table[ofs2..ofs2 + 4].try_into().unwrap());
        let raw_aligned = u32::from_le_bytes(table[ofs2 + 4..ofs2 + 8].try_into().unwrap());
        let uncompressed_size = u32::from_le_bytes(table[ofs2 + 8..ofs2 + 12].try_into().unwrap());
        let entry_type = table[ofs2 + 12];
        let offset = u32::from_le_bytes(table[ofs2 + 13..ofs2 + 17].try_into().unwrap());

        let compressed_size = raw_compressed
            .wrapping_sub(uncompressed_size)
            .wrapping_sub(715);
        let compressed_size_aligned = raw_aligned.wrapping_sub(37579);
        let flags = if entry_type & 0x01 != 0 {
            encryption_flags_for_extension(&name) | 0x01
        } else {
            entry_type
        };

        ofs = ofs2 + 17;

        if entry_type & 0x01 == 0 {
            continue;
        }

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
    Ok(entries)
}

fn encryption_flags_for_extension(filename: &str) -> u8 {
    match filename.rsplit('.').next() {
        Some("gnd" | "gat" | "act" | "str") => mixcrypt::GRF_FLAG_HEADER_DES_CRYPT,
        _ => mixcrypt::GRF_FLAG_FULL_MIX_CRYPT,
    }
}
