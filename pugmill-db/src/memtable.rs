use std::collections::BTreeMap;

use crate::Key;
use crate::Result;
use crate::Value;
use crate::memtable::MapEntry::Deleted;
use crate::memtable::MapEntry::Present;

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
}

impl MemTable {
    pub(crate) fn new() -> Self {
        Self {
            store: BTreeMap::new(),
            mem_usage: 0,
        }
    }

    pub(crate) fn put(&mut self, key: Key, value: Value) -> Result<()> {
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
        self.store.insert(key, Deleted);
        Ok(())
    }

    pub(crate) fn mem_usage(&self) -> usize {
        self.mem_usage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_val(key: &str, val: &str) -> (Key, Value) {
        (key.bytes().collect::<_>(), val.bytes().collect::<_>())
    }

    #[test]
    fn it_works() {
        let mut table = MemTable::new();
        let (k, v) = key_val("Azuz", "Is the best!");

        assert!(table.put(k.clone(), v.clone()).is_ok());
        assert_eq!(table.get(&k), Some(&v));

        assert!(table.delete(k.clone()).is_ok());
        assert_eq!(table.get(&k), None);
    }

    #[test]
    fn mem_usage() {
        let mut table = MemTable::new();
        let (k1, v1) = key_val("Azuz", "Is the best!");
        let (k2, v2) = key_val("Mehdi", "Is not!");
        assert!(table.put(k1.clone(), v1.clone()).is_ok());
        assert!(table.put(k2.clone(), v2.clone()).is_ok());

        assert_eq!(table.mem_usage(), k1.len() + v1.len() + k2.len() + v2.len());
    }
}
