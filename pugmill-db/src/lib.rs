use thiserror::Error;

type Key = Vec<u8>;
type Value = Vec<u8>;

enum Entry {
    Put(Key, Value),
    Delete(Key),
}

#[derive(Error, Debug)]
enum PugError {
    #[error("unknown error!")]
    Unknown,

    #[error("failed to perform io operation on the WAL: {0}")]
    WalOp(#[from] std::io::Error),

    #[error("failed to encode/decode wal entry: {0}")]
    WalEncode(String),
}

type Result<T> = std::result::Result<T, PugError>;

pub(crate) mod memtable;
pub(crate) mod wal;
