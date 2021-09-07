use thiserror::Error;
use crate::RbType;

#[derive(Error, Debug)]
pub enum ThurgoodError {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error("Invalid Marshal version")]
    Version(String),
    #[error("Bad symbol reference number {0}")]
    BadSymbolRef(usize),
    #[error("Invalid object reference number {0}")]
    BadObjectRef(usize),
    #[error("Bad instance type")]
    BadInstanceType(char),
    #[error("Unexpected Ruby type")]
    UnexpectedType { expected: RbType, found: RbType },
    #[error("Bad type byte")]
    BadTypeByte(u8),
}

impl ThurgoodError {
    pub fn unexpected_type(expected: RbType, found: RbType) -> Self {
        Self::UnexpectedType { expected, found }
    }
}

pub type TResult<T> = Result<T, ThurgoodError>;
