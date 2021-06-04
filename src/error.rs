use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Ssh(ssh2::Error),
    Io(io::Error),
    Json(serde_json::Error),
    ConfigError(ConfigError),
    RequestError(RequestError),
}

#[derive(Debug)]
pub struct OviumError {
    kind: ErrorKind,
    source: Error,
    detail: Option<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    UnknownNodes(Vec<String>),
    Parse(toml::de::Error),
}

#[derive(Debug)]
pub enum RequestError {
    UnknownNodes(Vec<String>),
}

#[derive(Debug)]
pub enum ErrorKind {
    InvalidConfig,
    LoadConfig,
    Handle,
    Bind,
    ClientRun,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "I/O error: {}", err),
            Error::Ssh(err) => write!(f, "Ssh error: {}", err),
            Error::Json(err) => write!(f, "Json error: {}", err),
            Error::ConfigError(err) => write!(f, "{}", err),
            Error::RequestError(err) => write!(f, "{}", err),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::Parse(err) => write!(f, "Parsing error: {}", err),
            ConfigError::UnknownNodes(err) => write!(f, "Unknown nodes: '{}'", err.join(", ")),
        }
    }
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RequestError::UnknownNodes(err) => write!(f, "Unknown nodes: '{}'", err.join(", ")),
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
            ErrorKind::ClientRun => writeln!(f, "Error running Ovium client"),
        }?;

        if let Some(detail) = &self.detail {
            write!(f, "  Caused by: {}", &self.source)?;
            write!(f, "  Detail: {}", detail)
        } else {
            write!(f, "  Caused by: {}", &self.source)
        }
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

impl From<ConfigError> for Error {
    fn from(error: ConfigError) -> Self {
        Error::ConfigError(error)
    }
}

impl From<RequestError> for Error {
    fn from(error: RequestError) -> Self {
        Error::RequestError(error)
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
