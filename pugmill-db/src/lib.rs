use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;

type Key = Bytes;
type Value = Bytes;

#[derive(Debug, PartialEq)]
enum Entry {
    Put(Key, Value),
    Delete(Key),
}

impl Entry {
    fn payload_capacity(&self) -> usize {
        match self {
            Entry::Put(key, value) => 1 + 4 + key.len() + 4 + value.len(),
            Entry::Delete(key) => 1 + 4 + key.len(),
        }
    }

    pub(crate) fn compute_crc(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        match self {
            Entry::Put(key, value) => {
                hasher.update(&[0u8]);
                hasher.update(&(key.len() as u32).to_be_bytes());
                hasher.update(key);
                hasher.update(&(value.len() as u32).to_be_bytes());
                hasher.update(value);
            }
            Entry::Delete(key) => {
                hasher.update(&[1u8]);
                hasher.update(&(key.len() as u32).to_be_bytes());
                hasher.update(key);
            }
        };
        hasher.finalize()
    }
}

impl From<&Entry> for Bytes {
    fn from(entry: &Entry) -> Self {
        let capacity = entry.payload_capacity();
        let mut payload = BytesMut::with_capacity(capacity);

        match entry {
            Entry::Put(key, value) => {
                payload.put_u8(0u8);
                payload.put_u32(key.len() as u32);
                payload.put_slice(key);
                payload.put_u32(value.len() as u32);
                payload.put_slice(value);
            }
            Entry::Delete(key) => {
                payload.put_u8(1u8);
                payload.put_u32(key.len() as u32);
                payload.put_slice(key);
            }
        };

        let crc = crc32fast::hash(&payload[..]);
        let mut final_buf = BytesMut::with_capacity(4 + payload.len());
        final_buf.put_u32(crc);
        final_buf.put_slice(&payload);
        final_buf.into()
    }
}

impl TryFrom<&mut Bytes> for Entry {
    type Error = PugError;

    fn try_from(buf: &mut Bytes) -> Result<Self> {
        if buf.remaining() < 5 {
            return Err(PugError::WalEncode("buffer too short for header".into()));
        }
        let expected_crc = buf.get_u32();
        let op = buf.get_u8();

        let entry = match op {
            0u8 => {
                if buf.remaining() < 4 {
                    return Err(PugError::WalEncode("buffer too short for key size".into()));
                }
                let ksize = buf.get_u32() as usize;
                if buf.remaining() < ksize + 4 {
                    return Err(PugError::WalEncode("buffer too short for key data".into()));
                }
                let key = buf.copy_to_bytes(ksize);
                let vsize = buf.get_u32() as usize;
                if buf.remaining() < vsize {
                    return Err(PugError::WalEncode("buffer too short for value data".into()));
                }
                let value = buf.copy_to_bytes(vsize);
                Entry::Put(key, value)
            }
            1u8 => {
                if buf.remaining() < 4 {
                    return Err(PugError::WalEncode("buffer too short for key size".into()));
                }
                let ksize = buf.get_u32() as usize;
                if buf.remaining() < ksize {
                    return Err(PugError::WalEncode("buffer too short for key data".into()));
                }
                let key = buf.copy_to_bytes(ksize);
                Entry::Delete(key)
            }
            _ => return Err(PugError::UnexpectedOperation(op)),
        };

        if entry.compute_crc() != expected_crc {
            return Err(PugError::InvalidCRC);
        }
        Ok(entry)
    }
}

#[derive(Error, Debug)]
enum PugError {
    #[error("unknown error!")]
    Unknown,

    #[error("failed to perform io operation on the WAL: {0}")]
    WalOp(#[from] std::io::Error),

    #[error("failed to encode/decode wal entry: {0}")]
    WalEncode(String),

    #[error("invalid CRC when parsing entry")]
    InvalidCRC,

    #[error("unexpected operation: {0}")]
    UnexpectedOperation(u8),
}

type Result<T> = std::result::Result<T, PugError>;

pub(crate) mod memtable;
pub(crate) mod wal;
