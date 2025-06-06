use std::{env, fmt::Display, io};

use definitions::serde_json;

pub type OpResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    VarError(env::VarError),
    IOError(io::Error),
    DBError(rbatis::Error),
    ParseError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;

        match self {
            VarError(err) => write!(f, "{err}"),
            IOError(err) => write!(f, "{err}"),
            DBError(err) => write!(f, "{err}"),
            ParseError(err) => write!(f, "{err}"),
        }
    }
}

impl From<env::VarError> for Error {
    fn from(value: env::VarError) -> Self {
        Error::VarError(value)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::IOError(value)
    }
}

impl From<rbatis::Error> for Error {
    fn from(value: rbatis::Error) -> Self {
        Error::DBError(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::ParseError(value.to_string())
    }
}
