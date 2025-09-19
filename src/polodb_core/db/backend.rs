use crate::{
    db::{rocksdb_wrapper::RocksDBWrapper, RocksDBTransaction},
    Result,
};
use std::path::Path;

pub trait Backend
where
    Self: Sized,
{
    type ReadTransaction;
    type WriteTransaction;

    fn transaction(&self) -> Result<Self::ReadTransaction>;

    fn write_transaction(&self) -> Result<Self::WriteTransaction>;

    fn open_path<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>;
}
