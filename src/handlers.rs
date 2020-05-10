use crate::error::Error;
use crate::server::*;
use crate::types::*;
use crossbeam_utils::thread;
use log::{info, warn};
use std::io::{BufWriter, Write};
use std::sync::mpsc::{self, channel};
use std::sync::Arc;

impl Handle<CmdRequest, CmdReturn> for HandlerRequest<CmdRequest, CmdReturn> {
    fn handle(&self, server_config: &ServerConfig) -> Result<(), Error> {
        let nodes = &self.req.nodes;
        let cmd = &self.req.command;
        let (tx, rx) = channel();
        let nodes_nb = nodes.len();
        info!(
            "Received command '{}' for nodes: [{}]",
            cmd,
            nodes.join(", ")
        );
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

        let cmd_response: Response = Response::Cmd(results);

        let mut writer = BufWriter::new(&self.stream);
        writer.write_all(&cmd_response.format_bytes()?)?;

        Ok(())
    }
}
