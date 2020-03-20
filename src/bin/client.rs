use getopts::Options;
use ovium::client::Client;
use ovium::types::*;
use simplelog::{Config, LevelFilter, TermLogger, TerminalMode};
use std::{env, process};

fn main() {
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed).unwrap();
    let args: Vec<_> = env::args().collect();
    let program_name = args[0].clone();
    let mut opts = Options::new();
    opts.optopt("s", "", "server socket path", "sock");
    opts.optopt("c", "", "remote command to launch", "command");
    opts.optopt("n", "", "nodes to manage", "nodes");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_usage(&program_name, opts);
        process::exit(0);
    }

    let socket_path = if let Some(s) = matches.opt_str("s") {
        s
    } else {
        println!("socket path is required!");
        process::exit(0);
    };

    if let Some(c) = matches.opt_str("c") {
        if let Some(n) = matches.opt_str("n") {
            let nodes: Vec<String> = n.split(',').map(String::from).collect();
            let client = Client {
                socket: socket_path,
                payload: Payload::Cmd {
                    hosts: nodes,
                    content: c,
                },
            };
            client.run();
        } else {
            println!("nodes list is required!");
            process::exit(0);
        }
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}
