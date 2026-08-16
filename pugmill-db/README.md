# Pugmill: An Embedded LSM-Tree Storage Engine

An embedded, fast, and concurrent Key-Value storage engine written in Rust. This project implements a Log-Structured Merge-Tree (LSM-Tree)
architecture featuring write-ahead logging (WAL), memtables, SSTables, and asynchronous background compaction.

## 🚀 Project Roadmap & Implementation Checklist

### Phase 1: Project Setup & Core Types
- [x] Define the core `Key` and `Value` types (e.g., `type Key = Vec<u8>;`, `type Value = Vec<u8>;`).
- [x] Define an `Entry` enum representing a database operation:
  - `Put(Key, Value)`
  - `Delete(Key)` (Tombstone)
- [x] Setup a custom `Error` type for the storage engine using `thiserror` or standard library.

### Phase 2: The MemTable (In-Memory Storage)
The MemTable handles active, incoming writes before they are flushed to disk.
- [x] Implement a sorted, in-memory data structure (e.g., a wrapper around `std::collections::BTreeMap` or a custom `SkipList` for better concurrency).
- [x] Implement `put(key, value)`: Insert or update an entry.
- [x] Implement `get(key)`: Retrieve a value, handling tombstones (return `None` if deleted).
- [x] Implement `delete(key)`: Insert a tombstone record.
- [x] Add an atomic memory tracker to calculate the approximate byte size of the MemTable (to trigger flushes when it gets too large).
    - [ ] Add atomimicity.
- [x] Write unit tests for MemTable operations.

### Phase 3: Write-Ahead Log (WAL)
To prevent data loss during a crash, all operations must be written to disk before modifying the MemTable.
- [ ] Define a binary record format on disk (e.g., `[CRC32 (4 bytes)] [Key Length (2 bytes)] [Key] [Value Length (4 bytes)] [Value]`).
- [ ] Implement `WalWriter`:
  - Open a file in append-only mode.
  - Write encoded operations (`Put` and `Delete`) to the file.
  - Implement a `sync` method to fsync data to disk.
- [ ] Implement `WalReader`:
  - Read sequentially through a WAL file.
  - Verify CRC32 checksums to detect corrupted records.
- [ ] Integrate WAL with MemTable: On every `put` or `delete`, write to the WAL first, then update the MemTable.
- [ ] Implement crash recovery: On startup, read the WAL and reconstruct the MemTable.

### Phase 4: Sorted String Tables (SSTable) - Disk Storage
When the MemTable gets too large, it is flushed to disk as an immutable SSTable.
- [ ] Define the SSTable layout (typically separated into Data Blocks, Index Blocks, and a Meta Block).
- [ ] Implement `SSTableBuilder`:
  - Accept sorted key-value pairs.
  - Write them into fixed-size data blocks (e.g., 4KB).
  - Build a sparse index (storing the first key of every data block and its offset).
- [ ] Implement `SSTable` (Reader):
  - Load the index block into memory upon opening.
  - Implement a binary search on the index to find the correct data block offset for a given key.
  - Read and decode the specific data block from disk to find the exact key.
- [ ] Implement an `SSTableIterator` to sequentially yield `(Key, Value)` pairs (needed for compaction).
- [ ] Write unit tests to verify writing an SSTable and reading specific keys back.

### Phase 5: The Storage State & Background Flushing
Managing the transition of data from RAM to Disk.
- [ ] Define the `LsmState` struct containing:
  - `active_memtable`: The current MemTable.
  - `immutable_memtables`: A list of MemTables waiting to be flushed.
  - `sstables`: A structured collection of SSTables on disk (organized by Levels).
- [ ] Implement the Freeze mechanism: When `active_memtable` hits a size threshold (e.g., 4MB), move it to `immutable_memtables` and create a new active MemTable and WAL.
- [ ] Implement the Flush Task:
  - Create a background worker (using `tokio::spawn` or a dedicated thread).
  - Take the oldest immutable MemTable, write it to an `SSTable` using `SSTableBuilder` (Level 0).
  - Safely update `LsmState` to include the new SSTable and drop the immutable MemTable and its WAL.

### Phase 6: Asynchronous Compaction (The Core Engine)
SSTables must be periodically merged to remove duplicates, clean up tombstones, and maintain read performance.
- [ ] Implement a `MergeIterator`: Takes multiple `SSTableIterator`s and yields the newest version of a key across all of them (handling overlapping keys).
- [ ] Implement the Compaction Loop: A background task that wakes up periodically or when triggered.
- [ ] Implement Size-Tiered or Leveled Compaction logic:
  - Select SSTables to compact (e.g., when Level 0 has too many files, compact them into Level 1).
  - Use `MergeIterator` to stream the selected SSTables.
  - Discard dropped keys and Tombstones (if no longer needed).
  - Write the output to one or more new SSTables.
- [ ] Safely swap the new SSTables into the `LsmState` and delete the old SSTable files from disk.

### Phase 7: Read Optimizations
LSM-Trees are write-heavy; we need these to speed up reads.
- [ ] Implement a **Bloom Filter**:
  - Generate a bloom filter during `SSTableBuilder`.
  - Store it in the SSTable Meta block.
  - When `get(key)` is called, check the bloom filter before performing an expensive disk read.
- [ ] Implement a **Block Cache**:
  - Use an LRU (Least Recently Used) cache (e.g., `moka` crate or a custom implementation).
  - Cache frequently accessed SSTable data blocks in memory.

### Phase 8: Concurrency & The Public API
Tying it all together into a thread-safe engine.
- [ ] Wrap the `LsmState` in `Arc<RwLock<...>>` (or `crossbeam`/`parking_lot` equivalent) to allow concurrent reads while background flushes/compactions update the state.
- [ ] Expose the main `Engine` API:
  - `pub fn open(path: impl AsRef<Path>) -> Result<Self>`
  - `pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>`
  - `pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()>`
  - `pub fn delete(&self, key: &[u8]) -> Result<()>`
- [ ] Ensure the `get` method queries in the correct order:
  1. Active Memtable
  2. Immutable Memtables (newest to oldest)
  3. Level 0 SSTables (newest to oldest)
  4. Deeper Level SSTables.

### Phase 9: Testing & Polish
- [ ] Write integration tests simulating realistic workloads (inserts, overwrites, deletes).
- [ ] Write crash-recovery tests (force panic/drop, reload engine, assert data consistency).
- [ ] Write a highly-concurrent test using `std::thread::spawn` to hammer the database with `put` and `get` requests simultaneously.
- [ ] Create a simple benchmarking suite using `criterion` to measure read and write throughput.

***

