use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process;

const SOCK_PATH: &str = "/run/stratsup.sock";

enum Command {
    Update,
    Shutdown,
    Status,
}

impl Command {
    fn parse(arg: &str) -> Result<Self, String> {
        match arg {
            "update" => Ok(Self::Update),
            "shutdown" => Ok(Self::Shutdown),
            "status" => Ok(Self::Status),
            _ => Err(format!("unknown command: {}", arg)),
        }
    }

    fn opcode(&self) -> u8 {
        match self {
            Self::Update => 0x01,
            Self::Shutdown => 0xFF,
            Self::Status => 0x02,
        }
    }
}

fn usage() {
    eprintln!("usage: strat-ctl <update|shutdown|status>");
}

fn main() {
    let mut args = env::args().skip(1);
    let command_arg = match args.next() {
        Some(arg) => arg,
        None => {
            usage();
            process::exit(2);
        }
    };
    if args.next().is_some() {
        usage();
        process::exit(2);
    }

    let command = match Command::parse(&command_arg) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("strat-ctl: {}", err);
            usage();
            process::exit(2);
        }
    };

    let mut stream = match UnixStream::connect(SOCK_PATH) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("strat-ctl: failed to connect to {}: {}", SOCK_PATH, err);
            process::exit(1);
        }
    };

    if let Err(err) = stream.write_all(&[command.opcode()]) {
        eprintln!("strat-ctl: failed to send command byte: {}", err);
        process::exit(1);
    }

    match command {
        Command::Status => {
            let mut response = [0u8; 4];
            if let Err(err) = stream.read_exact(&mut response) {
                eprintln!("strat-ctl: failed to read status response: {}", err);
                process::exit(1);
            }

            println!(
                "active={} slot_a={} slot_b={} slot_c={}",
                response[0], response[1], response[2], response[3]
            );
        }
        Command::Update | Command::Shutdown => {}
    }
}
