use crate::error::Error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdReturn {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_status: i32,
}

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
