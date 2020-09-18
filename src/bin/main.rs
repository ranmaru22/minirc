#![warn(missing_debug_implementations, rust_2018_idioms)]
const COMMAND_PREFIX: char = ':';

use pancurses::*;
use std::io::{stdin, BufReader, Result};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use libminirc::argparse;
use libminirc::channel::Channel;
use libminirc::command::{send_auth, Command};
use libminirc::interface::Interface;
use libminirc::ui::*;

fn main() -> Result<()> {
    let conn = argparse::setup()?;

    if let Ok(ref mut stream) = TcpStream::connect(&conn.address) {
        let term = initscr();
        cbreak();
        noecho();
        term.timeout(0);
        term.clear();
        term.refresh();
        term.keypad(true);

        let (term_rows, term_cols) = term.get_max_yx();
        let buffers_win = term.subwin(1, term_cols, 0, 0).unwrap();
        let input_win = term.subwin(2, term_cols, term_rows - 2, 0).unwrap();
        let output_win = term.subwin(term_rows - 3, term_cols, 1, 0).unwrap();
        input_win.mv(0, 0);
        input_win.hline('-', term_cols);
        output_win.mv(0, 0);
        output_win.printw(format!("Connected to {}\n", &conn.address));
        buffers_win.refresh();
        input_win.refresh();
        output_win.refresh();

        term.getch();
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
        let mut threads = Vec::with_capacity(3);

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
            }
            Ok(())
        });
        threads.push(stdin_thread);

        let (output_endy, output_endx) = output_win.get_max_yx();
        let output_last_line = output_endy - 2;
        output_win.mv(0, 0);
        // DEBUG: remove this!
        for _ in 1..15 {
            output_win.printw("Foobar\n");
        }
        // Main thread -- handling stdout

        input_win.mv(1, 0);
        let mut inp = String::new();
        loop {
            if interface.should_shutdown() {
                break;
            }

            if let Ok(printable) = stdout_rx.try_recv() {
                let output_endx = output_endx as usize;
                let mut words = printable.split_whitespace();
                let mut line = String::with_capacity(output_endx);
                output_win.refresh();
                while let Some(word) = words.next() {
                    if word.len() + line.len() < output_endx {
                        line.push_str(word);
                        line.push(' ');
                    } else {
                        line.insert(line.len() - 1, '\n');
                        shift_lines_up(&output_win, output_last_line);
                        output_win.printw(&line);
                        output_win.refresh();
                        line = String::with_capacity(output_endx);
                        line.push_str(word);
                        line.push(' ');
                    }
                }
                line.insert(line.len() - 1, '\n');
                shift_lines_up(&output_win, output_last_line);
                output_win.printw(&line);
                output_win.refresh();
            }
            let (y, x) = input_win.get_cur_yx();
            match term.getch() {
                Some(Input::KeyBackspace) => {
                    input_win.mv(y, x - 1);
                    inp.pop();
                    input_win.delch();
                }
                Some(Input::KeyLeft) => {
                    input_win.mv(y, x - 1);
                }
                Some(Input::KeyRight) => {
                    if x < inp.len() as i32 {
                        input_win.mv(y, x + 1);
                    }
                }
                Some(Input::Character(c)) if c == '\n' => {
                    write_tx.send(inp).expect("Could not send to WRITE");
                    input_win.deleteln();
                    input_win.mv(1, 0);
                    inp = String::default();
                }
                Some(Input::Character(c)) => {
                    inp.push(c);
                    input_win.insch(c);
                    input_win.mv(y, x + 1);
                }
                Some(_) | None => (),
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
