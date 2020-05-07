use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Ssh(ssh2::Error),
    Io(io::Error),
    Json(serde_json::Error),
    TomlDe(toml::de::Error),
    ConfigParse,
}

#[derive(Debug)]
pub struct OviumError {
    kind: ErrorKind,
    source: Error,
    detail: Option<String>,
}

#[derive(Debug)]
pub enum ErrorKind {
    InvalidConfig,
    LoadConfig,
    Handle,
    Bind,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "I/O error: {}", err),
            Error::Ssh(err) => write!(f, "Ssh error: {}", err),
            Error::Json(err) => write!(f, "Json error: {}", err),
            Error::TomlDe(err) => write!(f, "Toml deserialization error: {}", err),
            Error::ConfigParse => write!(f, "Configuration parsing error"),
        }
    }
}

impl fmt::Display for OviumError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            ErrorKind::InvalidConfig => writeln!(f, "Invalid configuration"),
            ErrorKind::LoadConfig => writeln!(f, "Failed to load configuration"),
            ErrorKind::Handle => writeln!(f, "Handle error"),
            ErrorKind::Bind => writeln!(f, "Error while binding socket"),
        }?;

        write!(f, "  Caused by: {}", &self.source)
    }
}

impl std::error::Error for Error {}

impl std::error::Error for OviumError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<ssh2::Error> for Error {
    fn from(error: ssh2::Error) -> Self {
        Error::Ssh(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Json(error)
    }
}

impl From<toml::de::Error> for Error {
    fn from(error: toml::de::Error) -> Self {
        Error::TomlDe(error)
    }
}

impl From<(ErrorKind, Error)> for OviumError {
    fn from((kind, source): (ErrorKind, Error)) -> Self {
        OviumError {
            kind,
            source,
            detail: None,
        }
    }
}

impl From<(ErrorKind, Error, String)> for OviumError {
    fn from((kind, source, detail): (ErrorKind, Error, String)) -> Self {
        OviumError {
            kind,
            source,
            detail: Some(detail),
        }
    }
}
