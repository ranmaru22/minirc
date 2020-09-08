#![warn(missing_debug_implementations, rust_2018_idioms)]

use std::env;
use std::io::prelude::*;
use std::net::{Shutdown, TcpStream};
use std::str;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

mod connection;
use connection::Connection;

#[macro_use]
mod utils;
use utils::{pong, print_msg, send_auth};

fn main() -> std::io::Result<()> {
    let argv: Vec<String> = env::args().collect();

    let conn = Connection::new(
        argv[1].to_owned(),
        argv[2].parse().expect("Invalid port number"),
        argv[3].to_owned(),
        argv[4].to_owned(),
    );

    #[allow(clippy::unused_io_amount)]
    if let Ok(mut stream) = TcpStream::connect(conn.address()) {
        println!("Connected to {}", &conn.address());

        let joined = loop {
            let mut buf = [0; 512];
            stream.read(&mut buf)?;
            let message = str::from_utf8(&buf).expect("Invalid Message");
            println!("{}", &message);

            if message.contains("PING") {
                pong(&message, &mut stream)?;
            }

            if message.contains("No Ident response") {
                send_auth(&conn, &mut stream)?;
            }

            if message.contains("376") {
                send_cmd!("JOIN", &conn.channel => stream);
            }

            if message.contains("366") {
                break true;
            }
        };

        if !joined {
            println!("Channel join failed");
            stream.shutdown(Shutdown::Both)?;
            return Ok(());
        }

        let mut stream_clone = stream.try_clone().expect("Error cloning stream");
        let (tx, rx) = mpsc::channel();

        let channel_thread = thread::spawn(move || -> std::io::Result<()> {
            loop {
                let mut buf = [0; 512];
                stream_clone.read(&mut buf)?;
                let message = str::from_utf8(&buf).expect("Invalid Message");
                if message.contains("PRIVMSG") {
                    print_msg(message)?;
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
            let mut inp = String::new();
            std::io::stdin().read_line(&mut inp).expect("Invalid input");
            if let Some('\n') = inp.chars().next_back() {
                inp.pop();
            }
            match inp {
                cmd if cmd.starts_with("/QUIT") => {
                    tx.send("QUIT").expect("Error sending QUIT cmd");
                    break;
                }
                cmd if cmd.starts_with("/WHOIS") => {
                    // TODO: This isn't working yet!
                    let target = cmd.split_whitespace().last().unwrap_or_default();
                    send_cmd!("WHOIS", target => stream);
                }
                cmd => {
                    let msg = format!("{} :{}", &conn.channel, &cmd);
                    send_cmd!("PRIVMSG", msg => stream);
                }
            }
            tx.send("OK").expect("Error sending OK cmd");
        }

        let _ = channel_thread.join();
        println!("Closing connection, bye!");
        stream.shutdown(Shutdown::Both)?;
    } else {
        println!("Could not connect to {}", &conn.address());
    }
    Ok(())
}
