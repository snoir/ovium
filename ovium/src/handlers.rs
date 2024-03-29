use crate::error::{Error, RequestError};
use crate::server::*;
use crate::types::*;
use crossbeam_utils::thread;
use log::{error, info, warn};
use std::io::{BufWriter, Write};
use std::sync::mpsc::{self, channel};
use std::sync::Arc;

impl ServerActions<CmdRequest> for ServerHandler<CmdRequest> {
    fn handle(self, server_config: &ServerConfig) -> Result<(), Error> {
        let mut nodes: Vec<&String> = Vec::new();
        for node in &self.req.nodes {
            if server_config.is_group(node) {
                if let Some(members) = server_config.groups.get(node) {
                    nodes.extend(members);
                }
            } else {
                nodes.push(node);
            }
        }
        nodes.sort();
        nodes.dedup();

        let cmd = &self.req.command;
        let (tx, rx) = channel();
        let nodes_nb = nodes.len();
        // Can't use join() on Vec<&String>
        // Might be a bug: https://github.com/rust-lang/rust/issues/82910
        info!("Received command '{}' for nodes: {:?}", cmd, nodes);
        let server_config = Arc::new(&server_config);

        thread::scope(move |s| {
            let mut threads = Vec::new();

            for node_name in nodes {
                let node_tx = tx.clone();
                let node_cmd = cmd.clone();
                let node_server_config = Arc::clone(&server_config);
                let node_thread = s.spawn(move |_| -> Result<(), mpsc::SendError<_>> {
                    info!("Launching '{}' on node: {}", node_cmd, node_name);
                    let exec_return = Server::execute_cmd(
                        &node_server_config.nodes[&node_name.to_owned()],
                        node_cmd,
                    );
                    let ssh_return = match exec_return {
                        Ok(ssh_return) => SshReturn::SshSuccess(ssh_return),
                        Err(err) => SshReturn::SshFailure(err.to_string()),
                    };
                    let cmd_return = CmdReturn {
                        node_name: node_name.clone(),
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

        let mut results = Vec::new();
        for _ in 0..nodes_nb {
            if let Ok(recv) = rx.recv() {
                results.push(recv);
            }
        }

        let cmd_response = Response::Cmd(results);

        let mut writer = BufWriter::new(&self.stream);
        writer.write_all(&cmd_response.encode()?)?;

        Ok(())
    }

    fn validate_request(&self, server_config: &ServerConfig) -> Result<(), Error> {
        let available_names = &mut server_config
            .nodes
            .keys()
            .into_iter()
            .collect::<Vec<&String>>();

        available_names.extend(
            server_config
                .groups
                .keys()
                .into_iter()
                .collect::<Vec<&String>>(),
        );

        let not_in_config: Vec<String> = self
            .req
            .nodes
            .iter()
            .cloned()
            .filter(|n| !available_names.contains(&n))
            .collect();

        if !not_in_config.is_empty() {
            error!(
                "Some nodes or groups are unknown (not in config): [{}]",
                not_in_config.join(", ")
            );

            let error_response =
                Response::Error(ResponseError::UnknownNodes(not_in_config.clone()));
            let mut writer = BufWriter::new(&self.stream);
            writer.write_all(&error_response.encode()?)?;

            return Err(Error::from(RequestError::UnknownNodes(not_in_config)));
        }

        Ok(())
    }
}

impl ClientActions<Response> for ClientHandler<Response> {
    fn handle(self) -> Result<(), Error> {
        match self.response {
            Response::Cmd(inner_resp) => ClientHandler::<Vec<CmdReturn>>::new(inner_resp).handle(),
            Response::Error(inner_resp) => ClientHandler::<ResponseError>::new(inner_resp).handle(),
        }
    }
}

impl ClientActions<Vec<CmdReturn>> for ClientHandler<Vec<CmdReturn>> {
    fn handle(self) -> Result<(), Error> {
        for cmd_return in self.response {
            println!("{}", cmd_return);
        }

        Ok(())
    }
}

impl ClientActions<ResponseError> for ClientHandler<ResponseError> {
    fn handle(self) -> Result<(), Error> {
        println!("{}", &self.response);

        Ok(())
    }
}
