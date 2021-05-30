use std::io;

pub type ReadResult<T> = Result<T, ReadError>;

#[derive(Debug)]
pub enum ReadError {
    IO(io::Error),
    Signature,
    Decoding(&'static str),
}

impl From<io::Error> for ReadError {
    fn from(value: io::Error) -> Self {
        Self::IO(value)
    }
}
