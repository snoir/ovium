use crate::error::{ConfigError, Error, ErrorKind, OviumError};
use crate::types::*;
use crossbeam_channel::unbounded;
use crossbeam_utils::thread;
use log::{error, info};
use serde::Deserialize;
use signal_hook::{iterator::Signals, SIGINT};
use ssh2::Session;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead, BufReader};
use std::net::TcpStream;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

pub struct Server<'a> {
    socket_path: &'a str,
    config: ServerConfig,
    listener: UnixListener,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub nodes: HashMap<String, Node>,
    pub groups: Option<HashMap<String, Vec<String>>>,
}

impl ServerConfig {
    pub fn is_group(&self, name: &str) -> bool {
        if let Some(groups) = &self.groups {
            groups.contains_key(name)
        } else {
            false
        }
    }

    pub fn group_members(&self, name: &str) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();
        if let Some(groups) = &self.groups {
            result.extend(groups[name].clone());
        }

        result
    }
}

impl Server<'_> {
    pub fn new(socket_path: &str) -> Result<Server, OviumError> {
        let config_path = Path::new("config");
        let server_config = ServerConfig::new(config_path)?;
        let listener =
            UnixListener::bind(socket_path).map_err(|err| (ErrorKind::Bind, err.into()))?;
        listener
            .set_nonblocking(true)
            .map_err(|err| (ErrorKind::Bind, err.into()))?;

        Ok(Server {
            socket_path,
            config: server_config,
            listener,
        })
    }

    pub fn run(&self) -> Result<(), OviumError> {
        thread::scope(|s| -> Result<(), OviumError> {
            let (signal_sender, signal_receiver) = unbounded();
            let signals = Signals::new(&[SIGINT]).unwrap();

            s.spawn(move |_| {
                for sig in signals.forever() {
                    println!("Received signal {:?}", sig);
                    if sig == signal_hook::SIGINT {
                        signal_sender.clone().send(sig).unwrap();
                        break;
                    }
                }
            });

            for stream in self.listener.incoming() {
                if signal_receiver.clone().try_recv().is_ok() {
                    break;
                }

                match stream {
                    Ok(stream) => {
                        /* connection succeeded */
                        let stream_receiver = signal_receiver.clone();
                        s.spawn::<_, Result<(), OviumError>>(move |_| {
                            self.handle_client(stream, stream_receiver)
                                .map_err(|err| (ErrorKind::Handle, err))?;
                            Ok(())
                        });
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(500));
                        continue;
                    }

                    Err(_) => {
                        /* connection failed */
                        break;
                    }
                }
            }
            Ok(())
        })
        .unwrap()?;

        Ok(())
    }

    fn handle_client(
        &self,
        stream: UnixStream,
        _signal_receiver: crossbeam_channel::Receiver<i32>,
    ) -> Result<(), Error> {
        let mut reader = BufReader::new(&stream);

        loop {
            let mut resp = Vec::new();
            let read_bytes = reader.read_until(b'\n', &mut resp);
            match read_bytes {
                Ok(read_bytes) => {
                    if read_bytes == 0 {
                        info!("connection closed by remote");
                    } else {
                        let recv_request = Request::from_slice(&resp)?;
                        let handler = match recv_request {
                            Request::Cmd(inner_req) => {
                                ServerHandler::<CmdRequest>::new(stream, inner_req)
                            }
                        };
                        handler.validate_request(&self.config)?;
                        handler.handle(&self.config)?;
                    };
                    break;
                }
                Err(err) => match err.kind() {
                    io::ErrorKind::Interrupted => continue,
                    _ => break,
                },
            }
        }
        Ok(())
    }

    pub fn execute_cmd(node: &Node, cmd: String) -> Result<SshSuccess, Error> {
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
}

impl Drop for Server<'_> {
    fn drop(&mut self) {
        std::fs::remove_file(&self.socket_path).unwrap();
    }
}

impl ServerConfig {
    pub fn new(config_dir: &Path) -> Result<ServerConfig, OviumError> {
        let nodes_file_path = config_dir.join("nodes.toml");
        let nodes_config_string = match read_file(&nodes_file_path) {
            Ok(config_string) => config_string,
            Err(err) => {
                error!("Unable to load file {:?}: {}", &nodes_file_path, err);
                return Err(OviumError::from((ErrorKind::LoadConfig, err)));
            }
        };

        let config: ServerConfig = toml::from_str(&nodes_config_string)
            .map_err(|err| (ErrorKind::InvalidConfig, ConfigError::Parse(err).into()))?;
        validate_config(&config).map_err(|err| (ErrorKind::InvalidConfig, err.into()))?;

        Ok(config)
    }
}

fn validate_config(config: &ServerConfig) -> Result<(), ConfigError> {
    let mut unknown_nodes: Vec<String> = Vec::new();
    if let Some(groups) = &config.groups {
        for node_group in groups {
            for node in node_group.1 {
                if !config.nodes.contains_key(node) {
                    unknown_nodes.push(node.to_string());
                }
            }
        }
    }

    if !unknown_nodes.is_empty() {
        return Err(ConfigError::UnknownNodes(unknown_nodes));
    }

    Ok(())
}

fn read_file(file: &Path) -> Result<String, Error> {
    let mut f = File::open(file)?;
    let mut file_string = String::new();
    f.read_to_string(&mut file_string)?;

    Ok(file_string)
}
