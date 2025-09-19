use crate::Result;

pub trait Backend {
    type ReadTransaction;
    type WriteTransaction;

    fn read_transaction(&self) -> Self::ReadTransaction;

    fn transaction(&self) -> Self::WriteTransaction;

    fn open_path<P>(path: P) -> Result<Self>;
}
