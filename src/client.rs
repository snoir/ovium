use crate::types::*;
use log::info;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::UnixStream;

pub struct Client {
    pub socket: String,
    pub payload: Payload,
}

impl Client {
    pub fn run(&self) -> io::Result<()> {
        let stream = UnixStream::connect(&self.socket)?;
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let mut resp = Vec::new();
        reader.read_until(b'\n', &mut resp)?;
        info!(
            "server thread sent : {}",
            String::from_utf8(resp).unwrap().trim()
        );

        writer.write_all(&self.payload.format_bytes())?;
        Ok(())
    }
}
