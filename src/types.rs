use crate::error::Error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdReturn {
    pub node_name: String,
    pub data: SshReturn,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SshReturn {
    SshSuccess(SshSuccess),
    SshFailure(Error),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SshSuccess {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_status: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdResponse {
    pub success: Vec<CmdReturn>,
    pub error: Vec<Error>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Payload {
    Cmd { nodes: Vec<String>, content: String },
    Ping { content: String },
    Hello { content: String },
    CmdReturn(CmdReturn),
    Error(Error),
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

impl Payload {
    pub fn from_slice(slice: Vec<u8>) -> Self {
        let payload: Payload = serde_json::from_slice(&slice).unwrap();
        payload
    }

    pub fn format_bytes(&self) -> Vec<u8> {
        let payload_slice = format!("{}\n", serde_json::to_string(&self).unwrap());
        payload_slice.as_bytes().to_vec()
    }
}
