#![warn(missing_debug_implementations, rust_2018_idioms)]

// DEFAULT PARAMETERS
const DEFAULT_SERVER: &str = "chat.freenode.net";
const DEFAULT_PORT: &str = "6667";
const DEFAULT_USERNAME: &str = "minirc_user";
const COMMAND_PREFIX: char = '/';
const CONFIG_PATH: &str = ".config/minirc/";

use std::io::prelude::*;
use std::io::{stdin, Result};
use std::net::{Shutdown, TcpStream};
use std::str;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

use argparse::{ArgumentParser, Store};

mod connection;
use connection::Connection;

mod channel;
use channel::Channel;

#[macro_use]
mod utils;
use utils::*;

fn setup() -> Result<Connection> {
    let mut server = String::from(DEFAULT_SERVER);
    let mut port = String::from(DEFAULT_PORT);
    let mut passwd = String::new();
    let mut uname = String::from(DEFAULT_USERNAME);

    {
        // arg parse block
        // borrows go out of scope after parsing
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
        // send_cmd!("CAP LS 302" => stream);
        send_auth(&conn, &mut stream)?;
        if let Some(passwd) = &conn.password {
            let passwd_cmd = format!("PASS {}", passwd);
            send_cmd!(passwd_cmd => stream);
        }

        let channels = Arc::new(Mutex::new(Vec::<Channel>::new()));
        let active_channel = Arc::new(Mutex::new(0));

        let mut stream_clone = stream.try_clone().expect("Error cloning stream");
        let (tx, rx) = mpsc::channel();
        let (mx_channels, mx_active) = (Arc::clone(&channels), Arc::clone(&active_channel));
        let channel_thread = thread::spawn(move || -> Result<()> {
            loop {
                let mut buf = [0; 512];
                stream_clone.read(&mut buf)?;
                let message = str::from_utf8(&buf).expect("Invalid Message");

                match message {
                    m if m.contains("PING") => pong(&message, &mut stream_clone)?,
                    m if m.contains("PRIVMSG") => {
                        print_msg(message)?;
                        let mut channels = mx_channels.lock().unwrap();
                        let active_channel = mx_active.lock().unwrap();
                        if let Some(channel) = channels.get_mut(*active_channel) {
                            channel.write(&message)?;
                        }
                    }
                    m => println!("{}", m),
                }

                match rx.try_recv() {
                    Ok("QUIT") | Err(TryRecvError::Disconnected) => break,
                    Ok(_) | Err(TryRecvError::Empty) => {}
                }
            }
            Ok(())
        });

        loop {
            // main loop
            let mut inp = String::new();
            stdin().read_line(&mut inp).expect("Invalid input");
            if let Some('\n') = inp.chars().next_back() {
                inp.pop();
            }

            if let Some(COMMAND_PREFIX) = inp.chars().next() {
                let cmd = &inp[1..2];
                match cmd {
                    "q" => {
                        tx.send("QUIT").expect("Error sending QUIT cmd");
                        break;
                    }
                    "j" => {
                        let args = &inp[2..];
                        let join_cmd = format!("JOIN {}", args);
                        send_cmd!(join_cmd => stream);
                        // TODO: check whether join is successful
                        let joined_chan = Channel::new(args.trim().to_owned());
                        let mut channels = channels.lock().unwrap();
                        let mut active_channel = active_channel.lock().unwrap();
                        println!("CHAN FILE: {}", joined_chan.get_fp());
                        channels.push(joined_chan);
                        *active_channel = channels.len();
                    }
                    "c" => {
                        if let Ok(target) = &inp[2..].parse::<usize>() {
                            let mut active_channel = active_channel.lock().unwrap();
                            *active_channel = *target;
                        } else {
                            println!("Invalid buffer");
                        }
                    }
                    &_ => println!("Unknown command"),
                }
            } else {
                let mut channels = channels.lock().unwrap();
                let active_channel = active_channel.lock().unwrap();
                if let Some(channel) = channels.get_mut(*active_channel) {
                    let privmsg = format!("PRIVMSG {} :{}", channel.get_id(), &inp);
                    send_cmd!(privmsg => stream);
                    let log = format!("<{}> {}", &conn.username, &inp);
                    channel.write(&log)?;
                }
            }

            tx.send("OK").expect("Error sending OK cmd");
        }

        match channel_thread.join().unwrap() {
            Ok(_) => {
                println!("Closing connection, bye!");
                stream.shutdown(Shutdown::Both)?;
            }
            Err(e) => {
                eprintln!("Error shutting down: {}", e);
            }
        }
    } else {
        println!("Could not connect to {}", &conn.address);
    }

    Ok(())
}
