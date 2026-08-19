use crate::{Entry, Result};
use bytes::{Buf, BufMut, Bytes};
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
        let wal_entry: Bytes = (&entry).into();

        self.fs.write_all(&wal_entry)?;
        self.fs.sync_all()?;

        Ok(())
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
        let mut entries_buf = Bytes::from(buf);
        let mut entries = vec![];

        while entries_buf.has_remaining() {
            let entry: Entry = (&mut entries_buf).try_into()?;
            entries.push(entry);
        }

        Ok(entries)
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
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn it_works() {
        let file = NamedTempFile::new().expect("should create temporary file");
        let mut wal_writer = WalWriter::new(file.path()).expect("should create WalWriter");

        let put_entry = Entry::Put(Bytes::from("key"), Bytes::from("value"));
        let del_entry = Entry::Delete(Bytes::from("key"));

        wal_writer
            .append(put_entry)
            .expect("should append PUT entry");
        wal_writer
            .append(del_entry)
            .expect("should append DELETE entry");

        // drop(wal_writer) is intentionally omitted; sync_all in WalWriter::append
        // ensures data is flushed before reading.

        let mut wal_reader = WalReader::new(file.path()).expect("should create WalReader");
        let entries = wal_reader.all_entries().expect("should read all entries");

        assert_eq!(entries.len(), 2);

        match &entries[0] {
            Entry::Put(k, v) => {
                assert_eq!(k, &Bytes::from("key"));
                assert_eq!(v, &Bytes::from("value"));
            }
            Entry::Delete(_) => {
                panic!("PUT entry expected.")
            }
        }

        match &entries[1] {
            Entry::Put(_, _) => panic!("unexpected entry"),
            Entry::Delete(k) => {
                assert_eq!(k, &Bytes::from("key"));
            }
        }
    }
}
