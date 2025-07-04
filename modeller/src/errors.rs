use std::{env, fmt::Display, io};

use definitions::bincode;

#[derive(Debug)]
pub enum Error {
    VarError(env::VarError),
    IOError(io::Error),
    DBError(rbatis::Error),
    ParseError(String),
    InternalError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;

        match self {
            VarError(err) => write!(f, "{err}"),
            IOError(err) => write!(f, "{err}"),
            DBError(err) => write!(f, "{err}"),
            ParseError(msg) => write!(f, "{msg}"),
            InternalError(msg) => write!(f, "{msg}"),
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

impl From<bincode::error::DecodeError> for Error {
    fn from(value: bincode::error::DecodeError) -> Self {
        Error::ParseError(value.to_string())
    }
}
