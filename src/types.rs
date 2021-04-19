use crate::error::Error;
use crate::server::ServerConfig;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::os::unix::net::UnixStream;

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const NC: &str = "\x1b[0m";

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdReturn {
    pub node_name: String,
    pub data: SshReturn,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdRequest {
    pub nodes: Vec<String>,
    pub command: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SshReturn {
    SshSuccess(SshSuccess),
    SshFailure(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SshSuccess {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_status: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Cmd(Vec<CmdReturn>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Cmd(CmdRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub ip: String,
    #[serde(default = "default_port")]
    pub port: u32,
    #[serde(default = "default_user")]
    pub user: String,
}

fn default_user() -> String {
    "root".to_string()
}

fn default_port() -> u32 {
    22
}

pub trait Message: Serialize {
    fn from_slice<'a>(slice: &'a [u8]) -> Result<Self, Error>
    where
        Self: Sized + Deserialize<'a>,
    {
        let response = serde_json::from_slice(&slice)?;
        Ok(response)
    }

    fn format_bytes(&self) -> Result<Vec<u8>, Error> {
        let slice = format!("{}\n", serde_json::to_string(&self)?);
        Ok(slice.as_bytes().to_vec())
    }
}

impl Message for Request {}
impl Message for Response {}

#[derive(Debug)]
pub struct ServerHandler<T> {
    pub stream: UnixStream,
    pub req: T,
}

pub trait ServerHandle<T> {
    fn new(stream: UnixStream, req: T) -> ServerHandler<T> {
        ServerHandler { stream, req }
    }

    fn handle(self, server_config: &ServerConfig) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct ClientHandler<T> {
    pub response: T,
}

pub trait ClientHandle<T> {
    fn new(response: T) -> ClientHandler<T> {
        ClientHandler { response }
    }

    fn handle(self) -> Result<(), Error>;
}

impl Display for CmdReturn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.data {
            SshReturn::SshSuccess(success) => {
                if success.exit_status == 0 {
                    write!(f, "{}", GREEN)?;
                    write!(f, "{} | SUCCESS:", self.node_name)?;
                } else {
                    write!(f, "{}", RED)?;
                    write!(f, "{} | FAILED:", self.node_name)?;
                }
                write!(f, "\n  exit_status: {}", success.exit_status)?;
                if let Some(stdout) = &success.stdout {
                    write!(f, "\n  stdout:\n")?;
                    for line in stdout.trim().lines() {
                        writeln!(f, "    {}", line)?;
                    }
                }
                if let Some(stderr) = &success.stderr {
                    write!(f, "\n  stderr:\n")?;
                    for line in stderr.trim().lines() {
                        writeln!(f, "    {}", line)?;
                    }
                }
                write!(f, "{}", NC)
            }
            SshReturn::SshFailure(failure) => {
                write!(f, "{}", RED)?;
                writeln!(f, "{} | TRANSPORT FAILURE:", self.node_name)?;
                writeln!(f, "  {}", failure)?;
                write!(f, "{}", NC)
            }
        }
    }
}
