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
}

type Result<T> = std::result::Result<T, PugError>;

pub(crate) mod memtable;
