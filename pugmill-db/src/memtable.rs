use std::collections::BTreeMap;
use std::path::Path;

use crate::memtable::MapEntry::Deleted;
use crate::memtable::MapEntry::Present;
use crate::wal::WalReader;
use crate::wal::WalWriter;
use crate::{Entry, Key, Result, Value};

enum MapEntry {
    Present(Value),
    Deleted,
}

pub(crate) struct MemTable {
    store: BTreeMap<Key, MapEntry>,

    // The memeory usage field is an approxmative tracking metric, it deliberatly does not track
    // deletions and updates of already existing keys.
    // This is supposed to be inline with the size of the log.
    mem_usage: usize,

    wal_writer: WalWriter,
}

impl MemTable {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        let wal_writer = WalWriter::new(path)?;
        Ok(Self {
            store: BTreeMap::new(),
            mem_usage: 0,
            wal_writer,
        })
    }

    pub(crate) fn put(&mut self, key: Key, value: Value) -> Result<()> {
        self.wal_writer
            .append(Entry::Put(key.clone(), value.clone()))?;
        self.mem_usage += key.len() + value.len();
        self.store.insert(key, MapEntry::Present(value));
        Ok(())
    }

    pub(crate) fn get(&self, key: &Key) -> Option<&Value> {
        if let Some(entry) = self.store.get(key) {
            match entry {
                Deleted => None,
                Present(val) => Some(val),
            }
        } else {
            None
        }
    }

    pub(crate) fn delete(&mut self, key: Key) -> Result<()> {
        self.wal_writer.append(Entry::Delete(key.clone()))?;
        self.store.insert(key, Deleted);
        Ok(())
    }

    pub(crate) fn mem_usage(&self) -> usize {
        self.mem_usage
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use tempfile::NamedTempFile;

    use crate::wal::WalReader;

    use super::*;

    fn key_val(key: &str, val: &str) -> (Key, Value) {
        (key.bytes().collect::<_>(), val.bytes().collect::<_>())
    }

    #[test]
    fn it_works() {
        let file = NamedTempFile::new().expect("should create temporary file");
        let mut table = MemTable::new(file.path()).expect("should create memtable");
        let (k, v) = key_val("key", "value");

        assert!(table.put(k.clone(), v.clone()).is_ok());
        assert_eq!(table.get(&k), Some(&v));

        assert!(table.delete(k.clone()).is_ok());
        assert_eq!(table.get(&k), None);

        let mut wal_reader = WalReader::new(file.path()).expect("should create wal reader");
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
            Entry::Put(_, _) => panic!("unepected entry"),
            Entry::Delete(k) => {
                assert_eq!(k, &Bytes::from("key"));
            }
        }
    }

    #[test]
    fn mem_usage() {
        let file = NamedTempFile::new().expect("sjhould create temporary file");
        let mut table = MemTable::new(file.path()).expect("should create memtable");
        let (k1, v1) = key_val("Azuz", "Is the best!");
        let (k2, v2) = key_val("Mehdi", "Is not!");
        assert!(table.put(k1.clone(), v1.clone()).is_ok());
        assert!(table.put(k2.clone(), v2.clone()).is_ok());

        assert_eq!(table.mem_usage(), k1.len() + v1.len() + k2.len() + v2.len());
    }
}
