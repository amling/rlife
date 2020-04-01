use std::error::Error;
use std::ops::Deref;

pub enum ValidationError {
    Message(String),
    Help(Vec<String>),
}

impl<E: Error> From<E> for ValidationError {
    fn from(e: E) -> ValidationError {
        return ValidationError::Message(format!("{:?}", e));
    }
}

impl ValidationError {
    pub fn message<S: Deref<Target = str>, R>(msg: S) -> ValidationResult<R> {
        return Result::Err(ValidationError::Message(msg.to_string()));
    }

    pub fn help<R>(lines: Vec<String>) -> ValidationResult<R> {
        return Result::Err(ValidationError::Help(lines));
    }

    pub fn label<S: Deref<Target = str>>(self, prefix: S) -> ValidationError {
        return match self {
            ValidationError::Message(s) => ValidationError::Message(format!("{}: {:?}", &*prefix, s)),
            ValidationError::Help(lines) => ValidationError::Help(lines),
        };
    }

    pub fn panic(&self) -> ! {
        match self {
            ValidationError::Message(s) => panic!("{}", s),
            ValidationError::Help(lines) => {
                for line in lines {
                    println!("{}", line);
                }
                std::process::exit(0);
            },
        }
    }
}

pub type ValidationResult<T> = Result<T, ValidationError>;

pub trait Validates {
    type Target;

    fn validate(self) -> ValidationResult<Self::Target>;
}
