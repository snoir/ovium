use crate::error::Error;
use crate::types::*;
use getopts::Options;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::UnixStream;
use std::process;

pub struct Client<'a> {
    pub socket_path: &'a str,
}

impl Client<'_> {
    pub fn new(socket_path: &str) -> Client {
        Client { socket_path }
    }

    pub fn run(self, request: Request) -> Result<Response, Error> {
        let stream = UnixStream::connect(self.socket_path)?;
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let mut resp = Vec::new();
        writer.write_all(&request.encode()?)?;
        writer.flush()?;
        reader.read_until(b'\n', &mut resp)?;

        let response = Response::decode(&resp)?;

        Ok(response)
    }
}

pub struct Cli {
    opts: getopts::Options,
    args: Vec<String>,
}

impl Cli {
    pub fn new(args: Vec<String>) -> Cli {
        let mut opts = Options::new();
        opts.optopt("s", "", "server socket path", "sock");
        opts.optopt("c", "", "remote command to launch", "command");
        opts.optopt("n", "", "nodes to manage", "nodes");
        opts.optflag("h", "help", "print this help menu");

        Cli { opts, args }
    }

    pub fn parse(&self) -> (String, Request) {
        let program_name = self.args[0].clone();
        let matches = match self.opts.parse(&self.args[1..]) {
            Ok(m) => m,
            Err(f) => panic!("{}", f.to_string()),
        };

        if matches.opt_present("h") || self.args.len() < 2 {
            print_usage(&program_name, &self.opts);
            process::exit(0);
        }

        let socket_path = match matches.opt_str("s") {
            Some(s) => s,
            None => {
                eprintln!("socket path is required!");
                process::exit(1);
            }
        };

        if let Some(c) = matches.opt_str("c") {
            if let Some(n) = matches.opt_str("n") {
                let nodes: Vec<String> = n.split(',').map(String::from).collect();
                let request = Request::Cmd(CmdRequest { nodes, command: c });
                (socket_path, request)
            } else {
                eprintln!("nodes list is required!");
                process::exit(1);
            }
        } else {
            process::exit(1);
        }
    }
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}
