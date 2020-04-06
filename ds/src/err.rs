use std::error::Error;
use std::fmt::Debug;

#[derive(Debug)]
pub struct StringError {
    pub msg: String,
}

impl<E: Error> From<E> for StringError {
    fn from(e: E) -> StringError {
        StringError {
            msg: format!("{:?}", e),
        }
    }
}
