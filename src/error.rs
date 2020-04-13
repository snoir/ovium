use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;

#[derive(Serialize, Deserialize, Debug)]
pub enum Error {
    Ssh(String),
    Io(String),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error.to_string())
    }
}

impl From<ssh2::Error> for Error {
    fn from(error: ssh2::Error) -> Self {
        Error::Ssh(error.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err_msg = match self {
            Error::Ssh(value) => format!("Ssh error: {}", value),
            Error::Io(value) => format!("I/O error: {}", value),
        };

        write!(f, "{}", err_msg)
    }
}
