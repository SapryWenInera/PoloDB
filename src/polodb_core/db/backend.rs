use crate::Result;

pub trait Backend {
    type ReadTranscation;
    type WriteTransaction;

    fn read_transaction(&self) -> Self::ReadTranscation;

    fn transaction(&self) -> Self::WriteTransaction;

    fn open_path<P>(path: P) -> Result<Self>;
}
