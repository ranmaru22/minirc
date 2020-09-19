#![warn(missing_debug_implementations, rust_2018_idioms)]
const COMMAND_PREFIX: char = ':';
const DEBUG_MODE: bool = true;

use pancurses::*;
use std::io::{prelude::*, BufReader, Result};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use libminirc::command::{send_auth, Command};
use libminirc::interface::Interface;
use libminirc::thread_tools::*;
use libminirc::ui::*;
use libminirc::{argparse, refresh_all};

fn main() -> Result<()> {
    let conn = argparse::setup()?;

    if let Ok(ref mut stream) = TcpStream::connect(&conn.address) {
        let term = init_curses(DEBUG_MODE);
        let (term_rows, term_cols) = term.get_max_yx();

        let buffers_win = term.subwin(1, term_cols, 0, 0).unwrap();
        let input_win = term.subwin(1, term_cols, term_rows - 1, 0).unwrap();
        let output_win = term.subwin(term_rows - 2, term_cols, 1, 0).unwrap();

        output_win.printw(format!("Connected to {}\n", &conn.address));
        refresh_all![buffers_win, input_win, output_win];

        send_auth(&conn, stream)?;

        // Interface clones
        let interface = Arc::new(Interface::new(conn));
        let interface_read = interface.clone();
        let interface_write = interface.clone();

        // Stream clones
        let stream_read = stream.try_clone().expect("Error cloning stream");
        let mut stream_write = stream.try_clone().expect("Error cloning stream");

        // Channels
        let (write_tx, write_rx): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (stdout_tx, stdout_rx) = mpsc::channel();
        let stdout_tx_c = stdout_tx.clone();

        // Set up threads
        let mut threads = Vec::with_capacity(3);

        // Reading incoming data from TcpStream
        let read_thread = thread::spawn(move || -> Result<()> {
            let stream = stream_read;
            let interface = interface_read;

            loop {
                let mut reader = BufReader::new(&stream);
                let mut message = String::new();
                reader.read_line(&mut message)?;
                let command = Command::from(message.as_str());
                parse_incoming_cmd(command, &interface, &stdout_tx)?;

                if interface.should_shutdown() {
                    break;
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
                        parse_user_cmd(&inp, &interface, &stdout_tx, &argv)
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

        // Main thread -- handling stdout & UI
        let (output_endy, output_endx) = output_win.get_max_yx();
        let output_last_line = output_endy - 2;
        let mut inp = String::new();
        interface.toggle_refresh_buffers_flag();

        loop {
            if interface.should_shutdown() {
                break;
            }

            refresh_buffers(&buffers_win, &interface);

            if let Ok(printable) = stdout_rx.try_recv() {
                let max_len = output_endx as usize;
                let lines = split_line(&printable, max_len);
                for line in lines {
                    shift_lines_up(&output_win, output_last_line);
                    output_win.printw(&line);
                    output_win.refresh();
                }
            }

            if handle_input(&mut inp, &input_win, &term) {
                inp.pop();
                write_tx.send(inp).expect("Could not send to Write");
                input_win.deleteln();
                input_win.mv(0, 0);
                inp = String::new();
            }

            if input_win.is_touched() {
                input_win.refresh();
            }
        }

        for thread in threads {
            thread.join().unwrap()?;
        }

        output_win.printw("Shutting down. Bye!");
        stream.shutdown(Shutdown::Both)?;
        endwin();
    } else {
        println!("Could not connect to {}", &conn.address);
    }

    Ok(())
}
