#![warn(missing_debug_implementations, rust_2018_idioms)]
const COMMAND_PREFIX: char = ':';

use std::io::prelude::*;
use std::io::{stdin, stdout, BufReader, Result, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
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

    if let Ok(ref mut stream) = TcpStream::connect(&conn.address) {
        println!("Connected to {}", &conn.address);
        send_auth(&conn, stream)?;

        let channels = vec![Channel::new(&conn.server, &conn.server)];
        let channels = Arc::new(Mutex::new(channels));
        let active_channel = Arc::new(AtomicUsize::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));

        let stream_read = stream.try_clone().expect("Error cloning stream");
        let mut stream_write = stream.try_clone().expect("Error cloning stream");
        let conn_c = conn.clone();
        let channels_read = channels.clone();
        let channels_write = channels.clone();
        let active_channel_read = active_channel.clone();
        let active_channel_write = active_channel.clone();
        let shutdown_read = shutdown.clone();
        let shutdown_write = shutdown.clone();
        // let (read_tx, read_rx) = mpsc::channel();
        // let (write_tx, write_rx) = mpsc::channel();

        let mut threads = Vec::with_capacity(2);

        let read_thread = thread::spawn(move || -> Result<()> {
            let stream = stream_read;
            let channels = channels_read;
            let active_channel = active_channel_read;
            let shutdown = shutdown_read;

            loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                let mut reader = BufReader::new(&stream);
                let mut message = String::new();
                reader.read_line(&mut message)?;
                let command = Command::from_str(&message);

                let mut channels = channels.lock().unwrap();
                let index = active_channel.load(Ordering::Relaxed);
                match command {
                    Command::Privmsg(sender, target, _) => {
                        let mut channel_names = channels.iter().map(|c| c.get_id());
                        let printable = command.to_printable().unwrap();

                        let log_target = match target {
                            t if t == &conn.username => sender,
                            _ => target,
                        };

                        if let Some(pos) = channel_names.position(|c| c == log_target) {
                            channels[pos].write(&printable)?;
                            if pos == index {
                                print!("{}", &printable);
                            }
                        } else {
                            let mut c = Channel::new(log_target, &conn.server);
                            c.write(&printable)?;
                            channels.push(c);
                        }
                    }

                    _ => {
                        if let Some(printable) = command.to_printable() {
                            channels[0].write(&printable)?;
                            if index == 0 {
                                print!("{}", &printable);
                            }
                        }
                    }
                }
            }
            Ok(())
        });
        threads.push(read_thread);

        let write_thread = thread::spawn(move || -> Result<()> {
            let stream = &mut stream_write;
            let conn = conn_c;
            let channels = channels_write;
            let active_channel = active_channel_write;
            let shutdown = shutdown_write;

            loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                let mut inp = String::new();
                stdin().read_line(&mut inp).expect("Invalid input");
                if inp.starts_with(COMMAND_PREFIX) {
                    let cmd = &inp[1..2];
                    let args: Vec<_> = inp[2..].split_whitespace().collect();

                    let command = match cmd {
                        "q" => {
                            let quitmsg = "Quitting ...";
                            shutdown.store(true, Ordering::Relaxed);
                            Command::Quit(quitmsg)
                        }

                        "j" => {
                            // TODO: check whether join is successful
                            let mut channels = channels.lock().unwrap();
                            for channel in &args {
                                channels.push(Channel::new(channel, &conn.server));
                            }
                            let index = channels.len() - 1;
                            active_channel.store(index, Ordering::Relaxed);
                            Command::Join(&args)
                        }

                        "p" => {
                            let mut channels = channels.lock().unwrap();
                            for channel in &args {
                                if let Some(index) =
                                    channels.iter().position(|c| c.get_id() == *channel)
                                {
                                    channels.remove(index);
                                    active_channel.compare_and_swap(
                                        index,
                                        channels.len() - 1,
                                        Ordering::Relaxed,
                                    );
                                }
                            }
                            Command::Part(&args)
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
                            Command::Unknown
                        }

                        &_ => Command::Unknown,
                    };

                    command.send(stream)?;
                    if let Some(printable) = command.to_printable() {
                        print!("{}", &printable);
                    }
                } else {
                    let mut channels = channels.lock().unwrap();
                    if let Some(channel) = channels.get_mut(active_channel.load(Ordering::Relaxed))
                    {
                        let target = channel.get_id().to_owned();
                        let privmsg = Command::Privmsg(&conn.username, &target, &inp);
                        let printable = privmsg.to_printable();
                        privmsg.send(stream)?;
                        if let Some(msg) = printable {
                            channel.write(&msg)?;
                            print!("{}", &msg);
                        }
                    }
                }
            }
            Ok(())
        });
        threads.push(write_thread);

        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
        }

        for thread in threads {
            thread.join().unwrap()?;
        }
        println!("Closing connection, bye!");
        stream.shutdown(Shutdown::Both)?;
    } else {
        println!("Could not connect to {}", &conn.address);
    }

    Ok(())
}
