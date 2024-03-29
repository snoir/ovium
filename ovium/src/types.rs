use crate::error::Error;
use crate::server::ServerConfig;
use serde::{Deserialize, Deserializer, Serialize};
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
    #[serde(deserialize_with = "unescape_new_line")]
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
    Error(ResponseError),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Cmd(CmdRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ResponseError {
    UnknownNodes(Vec<String>),
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

fn unescape_new_line<'de, D>(deserializer: D) -> Result<SshSuccess, D::Error>
where
    D: Deserializer<'de>,
{
    let mut ssh_success: SshSuccess = Deserialize::deserialize(deserializer)?;
    if let Some(stdout) = ssh_success.stdout {
        ssh_success.stdout = Some(stdout.replace("\\n", "\n"));
    }

    if let Some(stderr) = ssh_success.stderr {
        ssh_success.stderr = Some(stderr.replace("\\n", "\n"));
    }

    Ok(ssh_success)
}

pub trait Message: Serialize {
    fn decode<'a>(slice: &'a [u8]) -> Result<Self, Error>
    where
        Self: Sized + Deserialize<'a>,
    {
        Ok(bincode::deserialize(slice)?)
    }

    fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut message = bincode::serialize(&self)?;
        message.extend("\n".as_bytes());
        Ok(message)
    }
}

impl Message for Request {}
impl Message for Response {}

#[derive(Debug)]
pub struct ServerHandler<T> {
    pub stream: UnixStream,
    pub req: T,
}

impl<T> ServerHandler<T> {
    pub fn new(stream: UnixStream, req: T) -> ServerHandler<T> {
        ServerHandler::<T> { stream, req }
    }
}

pub trait ServerActions<T> {
    fn handle(self, server_config: &ServerConfig) -> Result<(), Error>;

    fn validate_request(&self, server_config: &ServerConfig) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct ClientHandler<T> {
    pub response: T,
}

pub trait ClientActions<T> {
    #[allow(clippy::new_ret_no_self)]
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

impl Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", RED)?;
        match &self {
            ResponseError::UnknownNodes(ukn_nodes) => write!(
                f,
                "ERROR: Unknown nodes or groups: [{}]",
                ukn_nodes.join(", ")
            )?,
        };
        write!(f, "{}", NC)
    }
}
