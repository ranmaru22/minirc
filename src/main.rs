#![warn(missing_debug_implementations, rust_2018_idioms)]
const COMMAND_PREFIX: char = '/';

use std::io::prelude::*;
use std::io::{stdin, BufReader, Result};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

#[macro_use]
mod utils;
mod argparse;
mod channel;
mod connection;

use channel::Channel;
use utils::*;

fn main() -> Result<()> {
    let conn = argparse::setup()?;

    if let Ok(mut stream) = TcpStream::connect(&conn.address) {
        println!("Connected to {}", &conn.address);
        send_auth(&conn, &mut stream)?;
        if let Some(passwd) = &conn.password {
            let passwd_cmd = format!("PASS {}", passwd);
            send_cmd!(passwd_cmd => stream);
        }

        let channels = Arc::new(Mutex::new(Vec::<Channel>::new()));
        let active_channel = Arc::new(AtomicUsize::new(0));

        let mut stream_clone = stream.try_clone().expect("Error cloning stream");
        let (tx, rx) = mpsc::channel();
        let chans_clone = Arc::clone(&channels);
        let act_chan_clone = Arc::clone(&active_channel);

        let channel_thread = thread::spawn(move || -> Result<()> {
            loop {
                let mut reader = BufReader::new(&stream_clone);
                let mut message = String::new();
                reader.read_line(&mut message)?;

                match message {
                    m if m.contains("PING") => pong(&m, &mut stream_clone)?,
                    m if m.contains("PRIVMSG") => {
                        print_msg(&m)?;
                        let mut channels = mx_channels.lock().unwrap();
                        let active_channel = mx_active.lock().unwrap();
                        if let Some(channel) = channels.get_mut(*active_channel) {
                            channel.write(&m)?;
                        }
                    }
                    m => println!("{}", &m),
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
