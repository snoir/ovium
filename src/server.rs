use crate::types::*;
use crossbeam_utils::thread;
use log::info;
use serde_json::Result;
use ssh2::Session;
use std::io::prelude::*;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::os::unix::net::{UnixListener, UnixStream};

pub struct Server<'a> {
    path: &'a str,
}

impl Server<'_> {
    pub fn new(path: &str) -> Result<Server> {
        Ok(Server { path })
    }

    pub fn run(&self) -> io::Result<()> {
        let listener = UnixListener::bind(&self.path).unwrap();

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
