use std::fmt;
use std::io;

#[derive(Debug)]
pub enum CmdError {
    Ssh(ssh2::Error),
    Io(io::Error),
}

impl From<io::Error> for CmdError {
    fn from(error: io::Error) -> Self {
        CmdError::Io(error)
    }
}

impl From<ssh2::Error> for CmdError {
    fn from(error: ssh2::Error) -> Self {
        CmdError::Ssh(error)
    }
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err_msg = match self {
            CmdError::Ssh(value) => format!("Ssh error: {}", value.to_string()),
            CmdError::Io(value) => format!("I/O error: {}", value.to_string()),
        };

        write!(f, "{}", err_msg)
    }
}
