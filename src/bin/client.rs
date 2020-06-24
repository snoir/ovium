use ovium::client::{Cli, Client};
use ovium::error::{ErrorKind, OviumError};
use ovium::types::*;
use simplelog::{Config, LevelFilter, TermLogger, TerminalMode};
use std::env;

fn main() -> Result<(), OviumError> {
    match TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed) {
        Ok(_) => (),
        Err(err) => eprintln!("Failed while setting up logger: {}", err),
    }

    let args: Vec<String> = env::args().collect();
    let cli = Cli::new(args);
    let (socket_path, request) = cli.parse();
    let response = Client::new(&socket_path)
        .run(request)
        .map_err(|err| (ErrorKind::ClientRun, err.into()))?;
    let handler = ClientHandler::new(response);
    handler.handle().unwrap();

    Ok(())
}
