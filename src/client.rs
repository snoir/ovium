use crate::error::Error;
use crate::types::*;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::UnixStream;

pub struct Client {
    pub socket: String,
    pub request: Request,
}

impl Client {
    pub fn run(&self) -> Result<(), Error> {
        let stream = UnixStream::connect(&self.socket)?;
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let mut resp = Vec::new();
        writer.write_all(&self.request.format_bytes().unwrap())?;
        writer.flush()?;
        reader.read_until(b'\n', &mut resp)?;

        match &self.request {
            Request::Cmd { .. } => self.handle_cmd(resp),
            _ => println!("nada"),
        }

        Ok(())
    }

    fn handle_cmd(&self, resp: Vec<u8>) {
        let cmd_response = CmdResponse::from_slice(resp).unwrap();

        for result in cmd_response.results.iter() {
            println!("{}", result);
        }
    }
}
