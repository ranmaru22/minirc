#![warn(missing_debug_implementations, rust_2018_idioms)]

// DEFAULT PARAMETERS
const DEFAULT_SERVER: &str = "chat.freenode.net";
const DEFAULT_PORT: &str = "6667";
const DEFAULT_USERNAME: &str = "minirc_user";

use std::io::prelude::*;
use std::io::{stdin, Result};
use std::net::{Shutdown, TcpStream};
use std::str;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

use argparse::{ArgumentParser, Store};

mod connection;
use connection::Connection;

#[macro_use]
mod utils;
use utils::*;

fn setup() -> Result<Connection> {
    let mut server = String::from(DEFAULT_SERVER);
    let mut port = String::from(DEFAULT_PORT);
    let mut passwd = String::new();
    let mut uname = String::from(DEFAULT_USERNAME);

    {
        let mut parser = ArgumentParser::new();
        parser.set_description("Simple IRC client written in Rust.");
        parser
            .refer(&mut server)
            .add_option(&["-s", "--server"], Store, "Server to connect to");
        parser
            .refer(&mut port)
            .add_option(&["-p", "--port"], Store, "Port to connect to");
        parser
            .refer(&mut passwd)
            .add_option(&["-k", "--key"], Store, "Server password");
        parser
            .refer(&mut uname)
            .add_option(&["-n", "--name"], Store, "User handle to use");
        parser.parse_args_or_exit();
    }

    Ok(Connection::new(server, port, passwd, uname))
}

fn main() -> Result<()> {
    let conn = setup()?;

    #[allow(clippy::unused_io_amount)]
    if let Ok(mut stream) = TcpStream::connect(&conn.address) {
        println!("Connected to {}", &conn.address);
        send_cmd!("CAP LS 302" => stream);
        send_auth(&conn, &mut stream)?;
        if let Some(passwd) = &conn.password {
            let passwd_cmd = format!("PASS {}", passwd);
            send_cmd!(passwd_cmd => stream);
        }

        let mut stream_clone = stream.try_clone().expect("Error cloning stream");
        let (tx, rx) = mpsc::channel();
        let channel_thread = thread::spawn(move || -> Result<()> {
            loop {
                let mut buf = [0; 512];
                stream_clone.read(&mut buf)?;
                let message = str::from_utf8(&buf).expect("Invalid Message");

                match message {
                    m if m.contains("PING") => pong(&message, &mut stream_clone)?,
                    m if m.contains("PRIVMSG") => print_msg(message)?,
                    m => println!("{}", m),
                }

                match rx.try_recv() {
                    Ok("QUIT") | Err(TryRecvError::Disconnected) => {
                        break;
                    }
                    Ok(_) | Err(TryRecvError::Empty) => continue,
                }
            }
            stream_clone.shutdown(Shutdown::Both)?;
            Ok(())
        });

        loop {
            // main loop
            let mut inp = String::new();
            stdin().read_line(&mut inp).expect("Invalid input");
            if let Some('\n') = inp.chars().next_back() {
                inp.pop();
            }
            match inp {
                cmd if cmd.to_ascii_uppercase().starts_with("/QUIT") => {
                    tx.send("QUIT").expect("Error sending QUIT cmd");
                    break;
                }
                cmd if cmd.to_ascii_uppercase().starts_with("/JOIN") => {
                    let args: Vec<_> = cmd.split("JOIN").collect();
                    let join_cmd = format!("JOIN {}", args.get(1).unwrap_or(&""));
                    send_cmd!(join_cmd => stream);
                    break;
                }
                cmd if cmd.to_ascii_uppercase().starts_with("/WHOIS") => {
                    // TODO: This isn't working yet!
                    let args: Vec<_> = cmd.split("WHOIS").collect();
                    let whois_cmd = format!("WHOIS {}", args.get(1).unwrap_or(&""));
                    send_cmd!(whois_cmd => stream);
                }
                _ => {}
            }
            tx.send("OK").expect("Error sending OK cmd");
        }

        let _ = channel_thread.join();
        println!("Closing connection, bye!");
        stream.shutdown(Shutdown::Both)?;
    } else {
        println!("Could not connect to {}", &conn.address);
    }
    Ok(())
}
