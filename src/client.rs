use crate::types::*;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::UnixStream;

pub struct Client {
    pub socket: String,
    pub request: Request,
}

impl Client {
    pub fn run(&self) -> io::Result<()> {
        let stream = UnixStream::connect(&self.socket)?;
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let mut resp = Vec::new();
        writer.write_all(&self.request.format_bytes())?;
        writer.flush().unwrap();
        reader.read_until(b'\n', &mut resp)?;

        match &self.request {
            Request::Cmd { .. } => self.handle_cmd(resp),
            _ => println!("nada"),
        }

        Ok(())
    }

    fn handle_cmd(&self, resp: Vec<u8>) {
        let cmd_response = CmdResponse::from_slice(resp);

        for result in cmd_response.results.iter() {
            println!("{}", result);
        }
    }
}
