use crate::types::*;
use log::info;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::UnixStream;

pub struct Client {
    pub socket: String,
    pub payload: Payload,
}

impl Client {
    pub fn run(&self) {
        let stream = UnixStream::connect(&self.socket).unwrap();
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let mut resp = Vec::new();
        reader.read_until(b'\n', &mut resp).unwrap();
        info!(
            "server thread sent : {}",
            String::from_utf8(resp).unwrap().trim()
        );

        let _ping = Payload::Ping {
            content: "Ping to server".to_string(),
        };

        let cmd = &self.payload;

        writer.write_all(&cmd.format_bytes()).unwrap();
    }
}
