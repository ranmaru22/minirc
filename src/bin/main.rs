#![warn(missing_debug_implementations, rust_2018_idioms)]
const COMMAND_PREFIX: char = ':';

use std::io::{prelude::*, stdout, BufReader, Result, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::{str, thread};

use minirc::command::{send_auth, Command, UiCommand::*};
use minirc::interface::Interface;
use minirc::{argparse, thread_tools::*, ui::*};
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor};

fn main() -> Result<()> {
    let conn = argparse::setup()?;

    if let Ok(ref mut stream) = TcpStream::connect(&conn.address) {
        let mut stdin = termion::async_stdin();
        let mut stdout = stdout().into_raw_mode()?;
        write!(
            stdout,
            "{}{}Conntecting to {}...\r",
            termion::clear::All,
            termion::cursor::Goto(1, 1),
            &conn.server
        )?;
        stdout.flush()?;
        let (cols, rows) = termion::terminal_size()?;

        send_auth(&conn, stream)?;

        // Interface clones
        let interface = Arc::new(Interface::new(conn));
        let interface_read = interface.clone();
        let interface_send = interface.clone();

        // Stream clones
        let stream_read = stream.try_clone().expect("Error cloning stream");
        let mut stream_write = stream.try_clone().expect("Error cloning stream");

        // Channels
        let (send_tx, send_rx): (Sender<Command>, Receiver<_>) = mpsc::channel();
        let (stdout_tx, stdout_rx): (Sender<String>, Receiver<_>) = mpsc::channel();
        let stdout_tx_c = stdout_tx.clone();

        // Reading incoming data from TcpStream
        let read_thread = thread::spawn(move || -> Result<()> {
            let stream = stream_read;
            let interface = interface_read;
            let mut reader = BufReader::with_capacity(512, &stream);
            let mut bytes = Vec::with_capacity(512);

            while !interface.should_shutdown() {
                bytes.clear();
                reader.read_until(b'\n', &mut bytes)?;
                if let Ok(message) = str::from_utf8(&bytes) {
                    let command = Command::from(message);
                    if let Some(printable) = parse_incoming_cmd(command, &interface)? {
                        stdout_tx.send(printable).expect("Could not send to stdout");
                    }
                }
            }
            Ok(())
        });

        // Sending data to TcpStream
        let write_thread = thread::spawn(move || -> Result<()> {
            let stream = &mut stream_write;
            let stdout_tx = stdout_tx_c;
            let interface = interface_send;

            while !interface.should_shutdown() {
                if let Ok(command) = send_rx.try_recv() {
                    command.send(stream)?;
                    if let Some(printable) = command.to_printable() {
                        let active_channel = interface.get_active_channel_pos();
                        if active_channel != 0 {
                            interface.write_to_chan(active_channel, &printable)?;
                            stdout_tx.send(printable).expect("Could not send to stdout");
                        }
                    }
                }
            }

            Ok(())
        });

        // Main thread -- handling stdout & UI
        write!(stdout, "{}", cursor::Goto(1, 2))?;
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
                    "{}{}{}{}{}",
                    cursor::Save,
                    cursor::Goto(1, 1),
                    clear::CurrentLine,
                    &bufline,
                    cursor::Restore
                )?;

                interface.unset_refresh_buffers_flag();
            }

            if let Ok(printable) = stdout_rx.try_recv() {
                for line in split_line(&printable, cols as usize) {
                    write!(stdout, "{}\r", line)?;
                    stdout.flush()?;
                }
            }

            handle_input(&mut inp, &mut stdin)?;

            if inp.ends_with('\n') {
                let command = if inp.starts_with(COMMAND_PREFIX) {
                    parse_user_input(&inp, &interface)
                } else {
                    let active_channel = interface.get_active_channel();
                    let username = interface.get_username();
                    Command::Privmsg(username, active_channel, inp.to_owned())
                };

                match command {
                    Command::Internal(icmd) => match icmd {
                        PrintBuffers => write!(stdout, "Buffers: {}\r", &bufline)?,
                        SwitchBuffer(c) => {
                            write!(stdout, "{}{}", cursor::Goto(1, 2), clear::AfterCursor)?;
                            let print = |line: &'_ str| write!(stdout, "{}\r", line);
                            interface.apply_to_buffer(c, print)?;
                        }
                    },
                    Command::Quit(_) => {
                        send_tx.send(command).expect("Could not send to Write");
                        break;
                    }
                    _ => send_tx.send(command).expect("Could not send to Write"),
                }

                inp.clear();
            }

            write!(
                stdout,
                "{}{}{}>>> {}{}",
                cursor::Save,
                cursor::Goto(1, rows),
                clear::CurrentLine,
                &inp,
                cursor::Restore
            )?;

            stdout.flush()?;
        }

        write!(stdout, "Shutting down. Bye!")?;
        stdout.flush()?;
        stream.shutdown(Shutdown::Both)?;
        read_thread.join().unwrap()?;
        write_thread.join().unwrap()?;
    } else {
        println!("Could not connect to {}", &conn.address);
    }

    Ok(())
}
