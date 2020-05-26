use ovium::client::{Cli, Client};
use ovium::error::Error;
use ovium::types::*;
use simplelog::{Config, LevelFilter, TermLogger, TerminalMode};
use std::env;

fn main() -> Result<(), Error> {
    match TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed) {
        Ok(_) => (),
        Err(err) => eprintln!("Failed while setting up logger: {}", err),
    }

    let args: Vec<String> = env::args().collect();
    let cli = Cli::new(args);
    let (socket_path, request) = cli.parse();
    let response = Client::new(&socket_path).unwrap().run(request).unwrap();
    let handler = ClientHandler::new(response);
    handler.handle().unwrap();

    Ok(())
}
