use crate::types::*;
use crossbeam_utils::thread;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Result;
use ssh2::Session;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use toml::Value;

#[derive(Deserialize, Debug)]
pub struct Server<'a> {
    socket_path: &'a str,
    config: ServerConfig,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    nodes: HashMap<String, HashMap<String, String>>,
}

impl Server<'_> {
    pub fn new(socket_path: &str) -> Result<Server> {
        let config_path = Path::new("/home/samir/git/ovium-config");
        let server_config = ServerConfig::new(config_path).unwrap();
        Ok(Server {
            socket_path: socket_path,
            config: server_config,
        })
    }

    pub fn run(&self) -> io::Result<()> {
        let listener = UnixListener::bind(&self.socket_path).unwrap();

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    /* connection succeeded */
                    thread::scope(move |s| {
                        s.spawn(|_| {
                            self::Server::handle_client(stream);
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

    fn handle_client(stream: UnixStream) {
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let hello_payload = Payload::Hello {
            content: "Hello from Ovium server!".to_string(),
        };

        writer.write_all(&hello_payload.format_bytes()).unwrap();
        writer.flush().unwrap();

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
                            Payload::Cmd { hosts, content } => {
                                self::Server::handle_cmd(&stream, hosts, content)
                            }
                            Payload::Hello { .. } => info!("Hello"),
                            Payload::Ping { .. } => self::Server::handle_ping(&stream),
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
    }

    fn handle_ping(stream: &UnixStream) {
        let mut writer = BufWriter::new(stream);
        info!("Ping received, replying pong!");
        let hello_payload = Payload::Hello {
            content: "Pong from server!".to_string(),
        };
        writer.write_all(&hello_payload.format_bytes()).unwrap();
    }

    fn handle_cmd(stream: &UnixStream, hosts: Vec<String>, content: String) {
        info!("Sending command {} over ssh to node list", &content);
        for host in hosts {
            thread::scope(|s| {
                s.spawn(|_| {
                    let tcp = TcpStream::connect(host).unwrap();
                    let mut sess = Session::new().unwrap();
                    sess.set_tcp_stream(tcp);
                    sess.handshake().expect("fail");
                    sess.userauth_agent("root").expect("fail");
                    let mut channel = sess.channel_session().expect("fail");
                    channel.exec(&content).expect("fail");
                    let mut s = String::new();
                    channel.read_to_string(&mut s).unwrap();
                    println!("{}", s);
                    channel.wait_close().unwrap();
                    println!("{}", channel.exit_status().unwrap());
                });
            })
            .unwrap();
        }
    }
}

impl ServerConfig {
    pub fn new(config_dir: &Path) -> Result<ServerConfig> {
        let mut config_string = String::new();
        let node_path = config_dir.join("nodes.toml");

        let mut f = File::open(node_path).unwrap();
        f.read_to_string(&mut config_string).unwrap();
        let nodes: ServerConfig = toml::from_str(&config_string).unwrap();
        Ok(nodes)
    }
}
