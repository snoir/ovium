use getopts::Options;
use ovium::server::Server;
use simplelog::{Config, LevelFilter, TermLogger, TerminalMode};
use std::{env, process};

fn main() {
    match TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed) {
        Ok(_) => (),
        Err(err) => eprintln!("Failed while setting up logger: {}", err),
    }

    let args: Vec<_> = env::args().collect();
    let program_name = &args[0];
    let mut opts = Options::new();
    opts.optopt("s", "", "socket path to listen on", "SOCK");
    opts.optopt("c", "", "config files directory", "CONFIG-DIR");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("{}", f.to_string()),
    };

    if matches.opt_present("h") || args.len() < 2 {
        print_usage(program_name, &opts);
        process::exit(0);
    }

    let config_path = match matches.opt_str("c") {
        Some(c) => c,
        None => {
            println!("config directory option is required!");
            process::exit(1);
        }
    };

    if let Some(s) = matches.opt_str("s") {
        let server = match Server::new(&s, &config_path) {
            Ok(server) => server,
            Err(err) => {
                eprintln!("{}", err);
                process::exit(1);
            }
        };
        if let Err(err) = server.run() {
            eprintln!("{}", err);
            process::exit(1);
        }
    } else {
        println!("socket option is required!");
    }
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}
