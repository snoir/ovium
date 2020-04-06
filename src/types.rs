use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Payload {
    Cmd { nodes: Vec<String>, content: String },
    Ping { content: String },
    Hello { content: String },
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
