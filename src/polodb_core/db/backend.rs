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

    fn read_transaction(&self) -> Self::ReadTransaction;

    fn transaction(&self) -> Self::WriteTransaction;

    fn open_path<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>;
}
