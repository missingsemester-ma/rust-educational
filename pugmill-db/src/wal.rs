use crate::{Entry, PugError, Result};
use bincode::{Decode, Encode, config};
use std::io::Write;
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub(crate) struct WalWriter {
    fs: File,
}

#[derive(Decode, Encode, PartialEq, Debug)]
struct WalEntry {
    crc: u32,

    op: WalOp,
}

#[derive(Decode, Encode, PartialEq, Debug)]
enum WalOp {
    Put { key: Vec<u8>, value: Vec<u8> },
    Del { key: Vec<u8> },
}

impl WalWriter {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        let fs = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .map_err(|e| PugError::WalOp(e))?;

        Ok(WalWriter { fs })
    }

    pub(crate) fn append(&mut self, entry: Entry) -> Result<()> {
        let wal_entry = Self::build_entry(entry);
        let encoded_entry = bincode::encode_to_vec(wal_entry, config::standard())
            .map_err(|e| PugError::WalEncode(e.to_string()))?;

        self.fs
            .write_all(&encoded_entry)
            .map_err(|e| PugError::WalOp(e))?;
        self.fs.sync_all().map_err(|e| PugError::WalOp(e))?;

        Ok(())
    }

    fn build_entry(entry: Entry) -> WalEntry {
        let (crc, op) = match entry {
            Entry::Put(key, value) => {
                let mut hasher = crc32fast::Hasher::new();
                hasher.update(&key);
                hasher.update(&value);
                (
                    hasher.finalize(),
                    WalOp::Put {
                        key: key,
                        value: value,
                    },
                )
            }
            Entry::Delete(key) => {
                let mut hasher = crc32fast::Hasher::new();
                hasher.update(&key);
                (hasher.finalize(), WalOp::Del { key: key })
            }
        };

        WalEntry { crc, op }
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
        assert_eq!(buf.len(), 16);
    }
}
