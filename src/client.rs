use crate::error::Error;
use crate::types::*;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::UnixStream;

pub struct Client<'a> {
    pub socket_path: &'a str,
    pub request: Request,
}

impl Client<'_> {
    pub fn run(&self) -> Result<(), Error> {
        let stream = UnixStream::connect(&self.socket_path)?;
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let mut resp = Vec::new();
        writer.write_all(&self.request.format_bytes()?)?;
        writer.flush()?;
        reader.read_until(b'\n', &mut resp)?;

        match &self.request {
            Request::Cmd { .. } => self.handle_cmd(resp)?,
        }

        Ok(())
    }

    fn handle_cmd(&self, resp: Vec<u8>) -> Result<(), Error> {
        let cmd_response = Response::from_slice(&resp)?;
        match cmd_response {
            Response::Cmd(results) => {
                for result in results.iter() {
                    println!("{}", result);
                }
            }
        }

        Ok(())
    }
}
