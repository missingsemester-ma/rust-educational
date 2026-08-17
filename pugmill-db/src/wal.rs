use crate::{Entry, PugError, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::{Read, Write};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub(crate) struct WalWriter {
    fs: File,
}

impl WalWriter {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        let fs = OpenOptions::new().append(true).create(true).open(path)?;

        Ok(WalWriter { fs })
    }

    pub(crate) fn append(&mut self, entry: Entry) -> Result<()> {
        let wal_entry = Self::build_entry(entry);

        self.fs.write_all(&wal_entry)?;
        self.fs.sync_all()?;

        Ok(())
    }

    fn build_entry(entry: Entry) -> Bytes {
        let (crc, op) = match entry {
            Entry::Put(key, value) => {
                let mut buf = vec![];
                buf.put_u8(0u8);
                buf.put_u32(key.len() as u32);
                buf.put_slice(&key[..]);
                buf.put_u32(value.len() as u32);
                buf.put_slice(&value[..]);
                (crc32fast::hash(&buf[..]), buf)
            }
            Entry::Delete(key) => {
                let mut buf = vec![];
                buf.put_u8(1u8);
                buf.put_u32(key.len() as u32);
                buf.put_slice(&key[..]);
                (crc32fast::hash(&buf[..]), buf)
            }
        };

        let mut buf = BytesMut::new();
        buf.put_u32(crc);
        buf.put_slice(&op);
        buf.into()
    }
}

impl Drop for WalWriter {
    fn drop(&mut self) {
        if let Err(e) = self.fs.sync_all() {
            dbg!("Failed to sync disk WAL content to disk {:?}", e);
        }
    }
}

pub(crate) struct WalReader {
    fs: File,
}

impl WalReader {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        Ok(WalReader {
            fs: OpenOptions::new().read(true).open(path)?,
        })
    }

    pub(crate) fn all_entries(&mut self) -> Result<Vec<Entry>> {
        let mut buf = vec![];
        self.fs.read_to_end(&mut buf)?;
        let mut entries = vec![];
        let mut buf = BytesMut::from(&buf[..]);

        while buf.has_remaining() {
            let entry = Self::parse_entry(&mut buf)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    fn parse_entry(buf: &mut BytesMut) -> Result<Entry> {
        let crc = buf.get_u32();
        let op = buf.get_u8();

        let entry = match op {
            0u8 => {
                let ksize = buf.get_u32();
                let key = buf.copy_to_bytes(ksize as usize).to_vec();
                let vsize = buf.get_u32();
                let value = buf.copy_to_bytes(vsize as usize).to_vec();
                Entry::Put(key, value)
            }
            1u8 => {
                let ksize = buf.get_u32();
                let key = buf.copy_to_bytes(ksize as usize).to_vec();
                Entry::Delete(key)
            }
            _ => return Err(PugError::UnepectedOperation(op)),
        };

        let actual_crc = compute_crc(&entry);

        if actual_crc != crc {
            return Err(PugError::InvalidCRC);
        }
        Ok(entry)
    }
}

pub(crate) fn compute_crc(entry: &Entry) -> u32 {
    match entry {
        Entry::Put(key, value) => {
            let mut buf = vec![];
            buf.put_u8(0u8);
            buf.put_u32(key.len() as u32);
            buf.put_slice(&key[..]);
            buf.put_u32(value.len() as u32);
            buf.put_slice(&value[..]);
            crc32fast::hash(&buf[..])
        }
        Entry::Delete(key) => {
            let mut buf = vec![];
            buf.put_u8(1u8);
            buf.put_u32(key.len() as u32);
            buf.put_slice(&key[..]);
            crc32fast::hash(&buf[..])
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ascii::AsciiExt, assert_matches, io::Read};

    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn it_works() {
        let file = NamedTempFile::new().expect("should create temporary file");
        let mut wal = WalWriter::new(file.path()).expect("should create WalWriter");

        assert!(
            wal.append(Entry::Put(
                b"key".to_ascii_uppercase(),
                b"value".to_ascii_uppercase()
            ))
            .is_ok()
        );
        let mut buf = Vec::new();

        let mut write_file = OpenOptions::new()
            .read(true)
            .open(file.path())
            .expect("should open file");
        write_file.read_to_end(&mut buf).expect("should read file");
        assert_eq!(buf.len(), 21);

        let mut read_wal = WalReader::new(file.path()).expect("should create wal reader");
        let entries = read_wal.all_entries().expect("should read all entries");
        let first = entries.first().expect("There should be one entry.");
        match first {
            Entry::Put(k, v) => {
                assert_eq!(k, &b"key".to_ascii_uppercase());
                assert_eq!(v, &b"value".to_ascii_uppercase());
            }
            Entry::Delete(_) => {
                assert!(false);
            }
        }
    }
}
