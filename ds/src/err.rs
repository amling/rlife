use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;

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

impl StringError {
    pub fn label(&self, s: impl Display) -> StringError {
        StringError {
            msg: format!("{}: {}", s, self.msg),
        }
    }
}
