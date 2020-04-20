use crate::error::Error;
use serde::{Deserialize, Serialize};
//use serde_json::Result;
use std::fmt::{self, Display};

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const NC: &str = "\x1b[0m";

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdReturn {
    pub node_name: String,
    pub data: SshReturn,
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
pub struct CmdResponse {
    pub results: Vec<CmdReturn>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Cmd { nodes: Vec<String>, content: String },
    Ping { content: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub ip: String,
    pub port: i32,
    #[serde(default = "default_user")]
    pub user: String,
}

fn default_user() -> String {
    "root".to_string()
}

pub trait Transport: serde::Serialize {
    fn from_slice(slice: Vec<u8>) -> Result<Self, Error>
    where
        Self: Sized;

    fn format_bytes(&self) -> Result<Vec<u8>, Error> {
        let slice = format!("{}\n", serde_json::to_string(&self)?);
        Ok(slice.as_bytes().to_vec())
    }
}

impl Transport for Request {
    fn from_slice(slice: Vec<u8>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let request: Request = serde_json::from_slice(&slice)?;
        Ok(request)
    }
}

impl Transport for CmdResponse {
    fn from_slice(slice: Vec<u8>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let response: CmdResponse = serde_json::from_slice(&slice)?;
        Ok(response)
    }
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
