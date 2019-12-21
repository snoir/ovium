use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Cmd { hosts: Vec<String>, content: String },
    Ping { content: String },
    Hello { content: String },
}

pub struct ClientOptions {
    pub socket: String,
    pub msg: Message,
}

impl Message {
    pub fn from_slice(slice: Vec<u8>) -> Self {
        let msg: Message = serde_json::from_slice(&slice).unwrap();
        msg
    }

    pub fn format_bytes(&self) -> Vec<u8> {
        let msg_slice = format!("{}\n", serde_json::to_string(&self).unwrap());
        msg_slice.as_bytes().to_vec()
    }
}
