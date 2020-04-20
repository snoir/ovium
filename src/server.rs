use crate::error::{Error, ErrorKind, OviumError};
use crate::types::*;
use crossbeam_utils::thread;
use log::{error, info, warn};
use serde::Deserialize;
use ssh2::Session;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{self, channel};

pub struct Server<'a> {
    socket_path: &'a str,
    config: ServerConfig,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    nodes: HashMap<String, Node>,
}

impl Server<'_> {
    pub fn new(socket_path: &str) -> Result<Server, OviumError> {
        let config_path = Path::new("/home/samir/git/ovium-config");
        let server_config = ServerConfig::new(config_path)?;
        Ok(Server {
            socket_path,
            config: server_config,
        })
    }

    pub fn run(&self) -> Result<(), OviumError> {
        let listener =
            UnixListener::bind(&self.socket_path).map_err(|err| (ErrorKind::Bind, err.into()))?;

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    /* connection succeeded */
                    thread::scope(move |s| {
                        s.spawn::<_, Result<(), OviumError>>(move |_| {
                            self.handle_client(stream)
                                .map_err(|err| (ErrorKind::Handle, err))?;
                            Ok(())
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

    fn handle_client(&self, stream: UnixStream) -> Result<(), Error> {
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
                        let recv_request = Request::from_slice(resp)?;
                        match recv_request {
                            Request::Cmd { nodes, content } => {
                                self.handle_cmd(&stream, nodes, content)?
                            }
                            Request::Ping { .. } => self.handle_ping(&stream)?,
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

    fn handle_ping(&self, stream: &UnixStream) -> Result<(), Error> {
        let mut writer = BufWriter::new(stream);
        info!("Ping received, replying pong!");
        let ping_response = Request::Ping {
            content: "Pong from server!".to_string(),
        };
        writer.write_all(&ping_response.format_bytes()?)?;
        Ok(())
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

    fn handle_cmd(
        &self,
        stream: &UnixStream,
        nodes: Vec<String>,
        cmd: String,
    ) -> Result<(), Error> {
        let (tx, rx) = channel();
        let nodes_nb = nodes.len();
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
                let node_thread = s.spawn(move |_| -> Result<(), mpsc::SendError<_>> {
                    info!("Launching '{}' on node: {}", node_cmd, node_name);
                    let exec_return =
                        self::Server::execute_cmd(&self.config.nodes[&node_name], node_cmd);
                    let ssh_return = match exec_return {
                        Ok(ssh_return) => SshReturn::SshSuccess(ssh_return),
                        Err(err) => SshReturn::SshFailure(err.to_string()),
                    };
                    let cmd_return = CmdReturn {
                        node_name,
                        data: ssh_return,
                    };
                    node_tx.send(cmd_return)?;
                    Ok(())
                });
                threads.push(node_thread);
            }

            // If node_tx.send should failed
            for th in threads {
                if let Err(err) = th.join().unwrap() {
                    warn!("A command execution thread failed with error: {}", err);
                }
            }
        })
        .unwrap();

        let mut cmd_response: CmdResponse = CmdResponse {
            results: Vec::new(),
        };
        for _ in 0..nodes_nb {
            if let Ok(recv) = rx.recv() {
                cmd_response.results.push(recv);
            }
        }
        let mut writer = BufWriter::new(stream);
        writer.write_all(&cmd_response.format_bytes()?)?;
        Ok(())
    }
}

impl ServerConfig {
    pub fn new(config_dir: &Path) -> Result<ServerConfig, OviumError> {
        let node_path = config_dir.join("nodes.toml");
        let nodes_config_string = match read_file(&node_path) {
            Ok(config_string) => config_string,
            Err(err) => {
                error!("Unable to load file {:?}: {}", &node_path, err);
                return Err(OviumError::from((ErrorKind::LoadConfig, err.into())));
            }
        };

        let nodes: ServerConfig = toml::from_str(&nodes_config_string)
            .map_err(|err| (ErrorKind::InvalidConfig, err.into()))?;
        Ok(nodes)
    }
}

fn read_file(file: &Path) -> Result<String, Error> {
    let mut f = File::open(file)?;
    let mut file_string = String::new();
    f.read_to_string(&mut file_string)?;
    Ok(file_string)
}
