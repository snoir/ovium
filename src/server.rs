use crate::error::Error;
use crate::types::*;
use crossbeam_utils::thread;
use log::info;
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
    nodes: HashMap<String, Node>,
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
        let _writer = BufWriter::new(&stream);

        loop {
            let mut resp = Vec::new();
            let read_bytes = reader.read_until(b'\n', &mut resp);
            match read_bytes {
                Ok(read_bytes) => {
                    if read_bytes == 0 {
                        info!("connection closed by remote");
                        break;
                    } else {
                        let recv_request = Request::from_slice(resp);
                        match recv_request {
                            Request::Cmd { nodes, content } => {
                                self.handle_cmd(&stream, nodes, content)
                            }
                            Request::Ping { .. } => self.handle_ping(&stream),
                            //_ => warn!("Unhandled type!"),
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
        let ping_response = Request::Ping {
            content: "Pong from server!".to_string(),
        };
        writer.write_all(&ping_response.format_bytes()).unwrap();
    }

    fn execute_cmd(node: &Node, cmd: String) -> Result<SshSuccess, Error> {
        let node_addr = format!("{}:{}", node.ip, node.port);
        let tcp = TcpStream::connect(node_addr)?;
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_agent(&node.user)?;
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

        Ok(SshSuccess {
            stdout,
            stderr,
            exit_status,
        })
    }

    fn handle_cmd(&self, stream: &UnixStream, nodes: Vec<String>, cmd: String) {
        let (tx, rx) = channel();
        thread::scope(move |s| {
            let mut threads = Vec::new();
            info!(
                "Received command '{}' for nodes: [{}]",
                cmd,
                nodes.join(", ")
            );
            for node_name in nodes {
                let node_tx = tx.clone();
                let node_cmd = cmd.clone();
                let node_thread = s.spawn(move |_| {
                    info!("Launching '{}' on node: {}", node_cmd, node_name);
                    let exec_return =
                        self::Server::execute_cmd(&self.config.nodes[&node_name], node_cmd);
                    let ssh_return = match exec_return {
                        Ok(ssh_return) => SshReturn::SshSuccess(ssh_return),
                        Err(err) => SshReturn::SshFailure(err),
                    };
                    let cmd_return = CmdReturn {
                        node_name: node_name,
                        data: ssh_return,
                    };
                    node_tx.send(cmd_return).unwrap();
                });
                threads.push(node_thread);
            }
            let mut cmd_response: CmdResponse = CmdResponse {
                results: Vec::new(),
            };
            for _ in 0..threads.len().clone() {
                cmd_response.results.push(rx.recv().unwrap());
            }

            let mut writer = BufWriter::new(stream);
            writer.write(&cmd_response.format_bytes()).unwrap();
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
