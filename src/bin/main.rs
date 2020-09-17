#![warn(missing_debug_implementations, rust_2018_idioms)]
const COMMAND_PREFIX: char = ':';

use std::io::{stdin, BufReader, Result};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use libminirc::argparse;
use libminirc::channel::Channel;
use libminirc::command::{send_auth, Command};
use libminirc::interface::Interface;

fn main() -> Result<()> {
    let conn = argparse::setup()?;

    if let Ok(ref mut stream) = TcpStream::connect(&conn.address) {
        println!("Connected to {}", &conn.address);
        send_auth(&conn, stream)?;

        // Interface clones
        let interface = Arc::new(Interface::new(conn));
        let interface_read = interface.clone();
        let interface_write = interface.clone();
        let interface_stdin = interface.clone();

        // Stream clones
        let stream_read = stream.try_clone().expect("Error cloning stream");
        let mut stream_write = stream.try_clone().expect("Error cloning stream");

        // Channels
        let (write_tx, write_rx): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (stdout_tx, stdout_rx) = mpsc::channel();
        let stdout_tx_c = stdout_tx.clone();

        // Set up threads
        let mut threads = Vec::with_capacity(2);

        let read_thread = thread::spawn(move || -> Result<()> {
            // Reading incoming data from TcpStream
            let stream = stream_read;
            let interface = interface_read;

            loop {
                if interface.should_shutdown() {
                    break;
                }

                let mut reader = BufReader::new(&stream);
                let mut message = String::new();
                // reader.read_line(&mut message)?;
                // BufRead::read_line conflicts with termion::read_line
                std::io::BufRead::read_line(&mut reader, &mut message)?;
                let command = Command::from(message.as_str());

                match command {
                    Command::Privmsg(sender, target, _) => {
                        let printable = command.to_printable().unwrap();

                        let log_target = match target {
                            t if t == interface.get_username() => sender,
                            _ => target,
                        };

                        if let Some(pos) = interface.get_channel_pos(&log_target) {
                            interface.write_to_chan(pos, &printable)?;
                            if interface.is_active(&log_target) {
                                stdout_tx.send(printable).expect("Could not send to stdout");
                            }
                        } else {
                            let server = interface.get_server();
                            let mut c = Channel::new(log_target, &server);
                            c.write(&printable)?;
                            interface.push_channel(c);
                        }
                    }

                    _ => {
                        if let Some(printable) = command.to_printable() {
                            interface.write_to_chan(0, &printable)?;
                            if interface.get_active_channel_pos() == 0 {
                                stdout_tx.send(printable).expect("Could not send to stdout");
                            }
                        }
                    }
                }
            }
            Ok(())
        });
        threads.push(read_thread);

        let write_thread = thread::spawn(move || -> Result<()> {
            // Sending data to TcpStream
            let stream = &mut stream_write;
            let stdout_tx = stdout_tx_c;
            let interface = interface_write;

            loop {
                if interface.should_shutdown() {
                    break;
                }

                if let Ok(ref inp) = write_rx.try_recv() {
                    let argv: Vec<_>;
                    let active_channel = interface.get_active_channel();
                    let username = interface.get_username();
                    let command = if inp.starts_with(COMMAND_PREFIX) {
                        argv = inp[2..].split_whitespace().collect();
                        match &inp[1..2] {
                            "q" => {
                                let quitmsg = if argv.is_empty() {
                                    "Quitting ..."
                                } else {
                                    &inp[2..]
                                };
                                interface.set_shutdown_flag();
                                Command::Quit(quitmsg)
                            }

                            "j" => {
                                // TODO: check whether join is successful
                                for channel in &argv {
                                    interface.push_channel(Channel::new(
                                        channel,
                                        &interface.get_server(),
                                    ));
                                }
                                interface.store_active_channel(interface.channels_len() - 1);
                                Command::Join(&argv)
                            }

                            "p" => {
                                for channel in &argv {
                                    if let Some(index) = interface.get_channel_pos(*channel) {
                                        interface.remove_channel(index);
                                        if interface.is_active(*channel) {
                                            interface
                                                .store_active_channel(interface.channels_len() - 1);
                                        }
                                    }
                                }
                                Command::Part(&argv)
                            }

                            "c" => {
                                if let Ok(target) = &inp[2..].trim().parse::<usize>() {
                                    if interface.get_channel(*target).is_some() {
                                        interface.store_active_channel(*target);
                                    }
                                } else {
                                    let mut printable = String::from("Buffers: ");
                                    for i in 0..interface.channels_len() {
                                        let name = interface.get_channel(i).unwrap();
                                        printable.push_str(&format!("[{}]{} ", i, name));
                                    }
                                    stdout_tx.send(printable).expect("Could not send to stdout");
                                }
                                Command::Unknown
                            }

                            &_ => Command::Unknown,
                        }
                    } else {
                        Command::Privmsg(&username, &active_channel, &inp)
                    };

                    command.send(stream)?;
                    if let Some(printable) = command.to_printable() {
                        if interface.get_active_channel_pos() != 0 {
                            stdout_tx.send(printable).expect("Could not send to stdout");
                        }
                    }
                }
            }
            Ok(())
        });
        threads.push(write_thread);

        let stdin_thread = thread::spawn(move || -> Result<()> {
            // Reading input vom stdin
            let interface = interface_stdin;

            loop {
                if interface.should_shutdown() {
                    break;
                }

                let mut inp = String::new();
                stdin().read_line(&mut inp).expect("Invalid input");
                write_tx.send(inp).expect("Could not send input");
            }
            Ok(())
        });
        threads.push(stdin_thread);

        // Main threads -- handling stdout
        loop {
            if interface.should_shutdown() {
                break;
            }

            if let Ok(printable) = stdout_rx.try_recv() {
                print!("{}", printable);
            }
        }

        for thread in threads {
            thread.join().unwrap()?;
        }
        println!("Shutting down. Bye!.");
        stream.shutdown(Shutdown::Both)?;
    } else {
        println!("Could not connect to {}", &conn.address);
    }

    Ok(())
}