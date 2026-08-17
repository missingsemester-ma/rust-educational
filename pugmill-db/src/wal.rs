use crate::{Entry, Result};
use bytes::{BufMut, BytesMut};
use std::io::Write;
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

    fn build_entry(entry: Entry) -> BytesMut {
        let (crc, op) = match entry {
            Entry::Put(key, value) => {
                let mut buf = vec![];
                buf.put_u8(0u8);
                buf.put_slice(&key[..]);
                buf.put_slice(&value[..]);
                (crc32fast::hash(&buf[..]), buf)
            }
            Entry::Delete(key) => {
                let mut buf = vec![];
                buf.put_u8(1u8);
                buf.put_slice(&key[..]);
                (crc32fast::hash(&buf[..]), buf)
            }
        };

        let mut buf = BytesMut::new();
        buf.put_u32(crc);
        buf.put_slice(&op);
        buf
    }
}

impl Drop for WalWriter {
    fn drop(&mut self) {
        if let Err(e) = self.fs.sync_all() {
            dbg!("Failed to sync disk WAL content to disk {:?}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

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

        let mut file = OpenOptions::new()
            .read(true)
            .open(file.path())
            .expect("should open file");
        file.read_to_end(&mut buf).expect("should read file");

        dbg!(&buf);
        assert_eq!(buf.len(), 13);
    }
}
