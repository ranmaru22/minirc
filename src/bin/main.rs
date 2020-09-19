#![warn(missing_debug_implementations, rust_2018_idioms)]
const COMMAND_PREFIX: char = ':';

use std::io::{prelude::*, stdout, BufReader, Result, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use termion::async_stdin;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor};

use libminirc::argparse;
use libminirc::command::{send_auth, Command, UiCommand::*};
use libminirc::interface::Interface;
use libminirc::thread_tools::*;
use libminirc::ui::*;

fn main() -> Result<()> {
    let conn = argparse::setup()?;

    if let Ok(ref mut stream) = TcpStream::connect(&conn.address) {
        let mut stdin = async_stdin();
        let mut stdout = stdout().into_raw_mode()?;
        write!(
            stdout,
            "{}{}Conntecting to {}...\r",
            termion::clear::All,
            termion::cursor::Goto(1, 1),
            &conn.server
        )?;
        stdout.flush()?;

        send_auth(&conn, stream)?;

        // Interface clones
        let interface = Arc::new(Interface::new(conn));
        let interface_read = interface.clone();
        let interface_write = interface.clone();
        let interface_input = interface.clone();

        // Stream clones
        let stream_read = stream.try_clone().expect("Error cloning stream");
        let mut stream_write = stream.try_clone().expect("Error cloning stream");

        // Channels
        let (write_tx, write_rx): (Sender<Command>, Receiver<_>) = mpsc::channel();
        let (stdout_tx, stdout_rx): (Sender<String>, Receiver<_>) = mpsc::channel();
        let stdout_tx_c = stdout_tx.clone();

        // Set up threads
        let mut threads: Vec<JoinHandle<_>> = Vec::with_capacity(3);

        // Reading incoming data from TcpStream
        let read_thread = thread::spawn(move || -> Result<()> {
            let stream = stream_read;
            let interface = interface_read;

            while !interface.should_shutdown() {
                let mut reader = BufReader::new(&stream);
                let mut message = String::new();
                reader.read_line(&mut message)?;
                let command = Command::from(message);
                if let Some(printable) = parse_incoming_cmd(command, &interface)? {
                    stdout_tx.send(printable).expect("Could not send to stdout");
                }
            }
            Ok(())
        });
        threads.push(read_thread);

        // Sending data to TcpStream
        let write_thread = thread::spawn(move || -> Result<()> {
            let stream = &mut stream_write;
            let stdout_tx = stdout_tx_c;
            let interface = interface_write;

            while !interface.should_shutdown() {
                if let Ok(command) = write_rx.try_recv() {
                    command.send(stream)?;
                    if interface.get_active_channel_pos() != 0 {
                        if let Some(printable) = command.to_printable() {
                            stdout_tx.send(printable).expect("Could not send to stdout");
                        }
                    }
                }
            }
            Ok(())
        });
        threads.push(write_thread);

        let input_thread = thread::spawn(move || -> Result<()> {
            let interface = interface_input;

            while !interface.should_shutdown() {}
            Ok(())
        });
        threads.push(input_thread);

        // Main thread -- handling stdout & UI
        let mut inp = String::new();
        let mut bufline = String::new();
        interface.set_refresh_buffers_flag();

        while !interface.should_shutdown() {
            if interface.should_refresh_buffers() {
                bufline.clear();
                for i in 0..interface.channels_len() {
                    let name = interface.get_channel(i).unwrap();
                    bufline.push_str(&format!("[{}]{} ", i, name));
                }
                write!(
                    stdout,
                    "{}{}{}",
                    cursor::Goto(1, 1),
                    clear::CurrentLine,
                    &bufline
                )?;
            }

            if let Ok(printable) = stdout_rx.try_recv() {
                let lines = split_line(&printable, 50);
                for line in lines {
                    write!(stdout, "{}\r", line)?;
                    stdout.flush()?;
                }
            }

            handle_input(&mut inp, &mut stdin)?;
            let command = if inp.starts_with(COMMAND_PREFIX) {
                parse_user_input(&inp, &interface)
            } else {
                let active_channel = interface.get_active_channel();
                let username = interface.get_username();
                Command::Privmsg(username, active_channel, inp.to_owned())
            };

            if let Command::Internal(icmd) = command {
                match icmd {
                    PrintBuffers => write!(stdout, "Buffers: {}\r", &bufline)?,
                }
            } else {
                write_tx.send(command).expect("Could not send to Write");
            }

            inp.clear();
            stdout.flush()?;
        }

        for thread in threads {
            thread.join().unwrap()?;
        }

        write!(stdout, "Shutting down. Bye!")?;
        stdout.flush()?;
        stream.shutdown(Shutdown::Both)?;
    } else {
        println!("Could not connect to {}", &conn.address);
    }

    Ok(())
}
