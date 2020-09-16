#![warn(missing_debug_implementations, rust_2018_idioms)]
const COMMAND_PREFIX: char = ':';

use std::io::prelude::*;
use std::io::{stdin, stdout, BufReader, Result, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use termion::raw::IntoRawMode;

#[macro_use]
mod utils;
mod argparse;
mod channel;
mod command;
mod connection;

use channel::Channel;
use command::Command;
use utils::*;

fn main() -> Result<()> {
    let conn = Arc::new(argparse::setup()?);
    // let (cin, mut cout) = (stdin(), stdout().into_raw_mode().unwrap());

    if let Ok(ref mut stream) = TcpStream::connect(&conn.address) {
        // write!(cout, "{}", termion::clear::All).unwrap();
        // cout.flush().unwrap();
        println!("Connected to {}", &conn.address);
        send_auth(&conn, stream)?;

        let channels = Arc::new(Mutex::new(Vec::<Channel>::new()));
        let active_channel = Arc::new(AtomicUsize::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));

        let stream_c = stream.try_clone().expect("Error cloning stream");
        let shutdown_c = shutdown.clone();
        // let (tx, rx) = mpsc::channel();

        let channel_thread = thread::spawn(move || -> Result<()> {
            let stream = stream_c;
            let shutdown = shutdown_c;
            loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
                let mut reader = BufReader::new(&stream);
                let mut message = String::new();
                reader.read_line(&mut message)?;
                let command = Command::from_str(&message);

                match command {
                    _ => println!("{}", command.to_printable()),
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

            if inp.starts_with(COMMAND_PREFIX) {
                let cmd = &inp[1..2];
                match cmd {
                    "q" => {
                        let args = &inp[2..].trim();
                        let quitmsg = if args.is_empty() {
                            "Quitting ..."
                        } else {
                            args
                        };
                        Command::Quit(quitmsg).send(stream)?;
                        shutdown.store(true, Ordering::Relaxed);
                        break;
                    }

                    "j" => {
                        let args: Vec<_> = inp[2..].split_whitespace().collect();
                        Command::Join(&args).send(stream)?;
                        // TODO: check whether join is successful
                        let mut channels = channels.lock().unwrap();
                        for channel in args {
                            channels.push(Channel::new(channel, &conn.server));
                        }
                        let index = channels.len() - 1;
                        active_channel.store(index, Ordering::Relaxed);
                    }

                    "p" => {
                        let args: Vec<_> = inp[2..].split_whitespace().collect();
                        Command::Part(&args).send(stream)?;
                        let mut channels = channels.lock().unwrap();
                        for channel in args {
                            if let Some(index) = channels.iter().position(|c| c.get_id() == channel)
                            {
                                channels.remove(index);
                                active_channel.compare_and_swap(
                                    index,
                                    channels.len() - 1,
                                    Ordering::Relaxed,
                                );
                            }
                        }
                    }

                    "c" => {
                        let channels = channels.lock().unwrap();
                        if let Ok(target) = &inp[2..].trim().parse::<usize>() {
                            if channels.get(*target).is_some() {
                                active_channel.store(*target, Ordering::Relaxed);
                            }
                        } else {
                            let buffers = channels.iter().map(|c| c.get_id());
                            print!("Buffers: ");
                            for (i, elem) in buffers.enumerate() {
                                print!("[{}]{} ", i, elem);
                            }
                            println!();
                        }
                    }

                    &_ => println!("Unknown command"),
                }
            } else {
                let mut channels = channels.lock().unwrap();
                if let Some(channel) = channels.get_mut(active_channel.load(Ordering::Relaxed)) {
                    let target = channel.get_id().to_owned();
                    let privmsg = Command::Privmsg(&conn.username, &target, &inp);
                    privmsg.send(stream)?;
                    channel.write(&privmsg.to_printable())?;
                }
            }
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
