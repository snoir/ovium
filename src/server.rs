use crate::error::Error;
use crate::types::*;
use crossbeam_utils::thread;
use log::{info, warn};
use serde::Deserialize;
use ssh2::Session;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::channel;

pub struct Server<'a> {
    socket_path: &'a str,
    config: ServerConfig,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    nodes: HashMap<String, HashMap<String, String>>,
}

impl Server<'_> {
    pub fn new(socket_path: &str) -> Result<Server, io::Error> {
        let config_path = Path::new("/home/samir/git/ovium-config");
        let server_config = ServerConfig::new(config_path)?;
        Ok(Server {
            socket_path: socket_path,
            config: server_config,
        })
    }

    pub fn run(&self) -> io::Result<()> {
        let listener = UnixListener::bind(&self.socket_path)?;

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    /* connection succeeded */
                    thread::scope(move |s| {
                        s.spawn(move |_| {
                            self.handle_client(stream).unwrap();
                        });
                    })
                    .unwrap();
                }
                Err(_err) => {
                    /* connection failed */
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_client(&self, stream: UnixStream) -> io::Result<()> {
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let hello_payload = Payload::Hello {
            content: "Hello from Ovium server!".to_string(),
        };

        writer.write_all(&hello_payload.format_bytes())?;
        writer.flush()?;

        loop {
            let mut resp = Vec::new();
            let read_bytes = reader.read_until(b'\n', &mut resp);
            match read_bytes {
                Ok(read_bytes) => {
                    if read_bytes == 0 {
                        info!("connection closed by remote");
                        break;
                    } else {
                        let recv_payload = Payload::from_slice(resp);
                        match recv_payload {
                            Payload::Cmd { nodes, content } => {
                                self.handle_cmd(&stream, nodes, content)
                            }
                            Payload::Hello { .. } => info!("Hello"),
                            Payload::Ping { .. } => self.handle_ping(&stream),
                            _ => warn!("Unhandled type!"),
                        }
                        break;
                    };
                }
                Err(err) => match err.kind() {
                    io::ErrorKind::Interrupted => continue,
                    _ => break,
                },
            }
        }
        Ok(())
    }

    fn handle_ping(&self, stream: &UnixStream) {
        let mut writer = BufWriter::new(stream);
        info!("Ping received, replying pong!");
        let hello_payload = Payload::Hello {
            content: "Pong from server!".to_string(),
        };
        writer.write_all(&hello_payload.format_bytes()).unwrap();
    }

    fn execute_cmd(node_addr: String, cmd: String) -> Result<CmdReturn, Error> {
        let tcp = TcpStream::connect(node_addr)?;
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_agent("root")?;
        let mut channel = sess.channel_session()?;
        channel.exec(&cmd)?;
        let mut stdout_string = String::new();
        let mut stderr_string = String::new();
        channel.read_to_string(&mut stdout_string)?;
        channel.stderr().read_to_string(&mut stderr_string)?;
        channel.wait_close()?;

        let stderr = if stderr_string.is_empty() {
            None
        } else {
            Some(stderr_string)
        };

        let stdout = if stdout_string.is_empty() {
            None
        } else {
            Some(stdout_string)
        };

        let exit_status = channel.exit_status()?;

        Ok(CmdReturn {
            stdout,
            stderr,
            exit_status,
        })
    }

    fn handle_cmd(&self, stream: &UnixStream, nodes: Vec<String>, cmd: String) {
        let (tx, rx) = channel();
        thread::scope(move |s| {
            let mut threads = Vec::new();
            for node in nodes {
                let node_tx = tx.clone();
                let node_cmd = cmd.clone();
                let node_thread = s.spawn(move |_| {
                    let node_addr = format!(
                        "{}:{}",
                        &self.config.nodes[&node]["ip"], &self.config.nodes[&node]["port"]
                    );
                    let cmd_return = self::Server::execute_cmd(node_addr, node_cmd);
                    node_tx.send(cmd_return).unwrap();
                });
                threads.push(node_thread);
            }
            let mut results: Vec<Result<CmdReturn, Error>> = Vec::new();
            for _ in 0..threads.len().clone() {
                results.push(rx.recv().unwrap());
            }
            dbg!(&results);
        })
        .unwrap();
    }
}

impl ServerConfig {
    pub fn new(config_dir: &Path) -> Result<ServerConfig, io::Error> {
        let mut config_string = String::new();
        let node_path = config_dir.join("nodes.toml");

        let mut f = File::open(node_path)?;
        f.read_to_string(&mut config_string)?;
        let nodes: ServerConfig = toml::from_str(&config_string)?;
        Ok(nodes)
    }
}
