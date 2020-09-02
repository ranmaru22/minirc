use regex::Regex;
use std::env;
use std::io::prelude::*;
use std::net::TcpStream;
use std::str;
// use std::thread;

#[derive(Debug)]
struct Connection {
    server: String,
    port: u16,
    channel: String,
    username: String,
}

impl Connection {
    pub fn new(server: String, port: u16, channel: String, username: String) -> Self {
        Connection {
            server,
            port,
            channel: Connection::parse_channel(channel),
            username,
        }
    }

    fn parse_channel(channel: String) -> String {
        match channel {
            c if c.starts_with("#") => c,
            c => format!("#{}", c),
        }
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.server, self.port)
    }
}

enum MsgCmd {
    PRIVMSG,
    MOTD,
    PING,
    Other,
}

struct Message {
    cmd: MsgCmd,
    source: String,
    target: String,
    content: String,
}

impl Message {
    pub fn from_buffer(buf: &[u8; 512]) -> Option<Self> {
        let as_utf8 = str::from_utf8(buf).expect("Invalid Message");
        let mut split_iter = as_utf8.split_whitespace();

        let source = match split_iter.next() {
            Some(src) => src.split("!").collect::<Vec<_>>()[0].to_owned(),
            None => return None,
        };

        let message = match split_iter.next() {
            Some("PING") => Message {
                cmd: MsgCmd::PING,
                source,
                target: match split_iter.next() {
                    Some(tg) => tg.to_owned(),
                    None => String::new(),
                },
                content: split_iter.collect::<Vec<&str>>().join(" "),
            },
            Some("MOTD") => Message {
                cmd: MsgCmd::MOTD,
                source,
                target: match split_iter.next() {
                    Some(tg) => tg.to_owned(),
                    None => String::new(),
                },
                content: split_iter.collect::<Vec<&str>>().join(" "),
            },
            Some("PRIVMSG") => Message {
                cmd: MsgCmd::PRIVMSG,
                source,
                target: match split_iter.next() {
                    Some(tg) => tg.to_owned(),
                    None => String::new(),
                },
                content: split_iter.collect::<Vec<&str>>().join(" "),
            },
            Some(_) => Message {
                cmd: MsgCmd::Other,
                source,
                target: match split_iter.next() {
                    Some(tg) => tg.to_owned(),
                    None => String::new(),
                },
                content: split_iter.collect::<Vec<&str>>().join(" "),
            },
            None => return None,
        };
        Some(message)
    }
}

fn format_cmd(cmd: &str, msg: &str) -> Box<[u8]> {
    let bytes = format!("{} {}\r\n", cmd, msg).into_bytes();
    bytes.into_boxed_slice()
}

fn send_auth(conn: &Connection, stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write(&*format_cmd("NICK", &conn.username))?;
    stream.write(&*format_cmd(
        "USER",
        &format!("{0} * * :{0}", &conn.username),
    ))?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let argv: Vec<String> = env::args().collect();

    let conn = Connection::new(
        argv[1].to_owned(),
        argv[2].parse().expect("Invalid port number"),
        argv[3].to_owned(),
        argv[4].to_owned(),
    );

    if let Ok(mut stream) = TcpStream::connect(conn.address()) {
        println!("Connected to {}", &conn.server);
        let mut buf;

        // Welcome loop
        loop {
            buf = [0; 512];
            stream.read(&mut buf)?;
            // let message = str::from_utf8(&buf).expect("Invalid message");
            if let Some(message) = Message::from_buffer(&buf) {
                match message.cmd {
                    MsgCmd::PING => {
                        println!("Recevied a PING");
                        stream.write(&*format_cmd("PONG", &message.content))?;
                        println!("Sent a PONG");
                    }
                    MsgCmd::MOTD => {
                        println!("MOTD - {}", &message.content);
                    }
                    MsgCmd::PRIVMSG => {
                        println!("{} -> {}", &message.source, &message.content);
                    }
                    MsgCmd::Other => {
                        if message.content.contains("No Ident response") {
                            send_auth(&conn, &mut stream)?;
                        } else if message.content.contains("376") {
                            stream.write(&*format_cmd("JOIN", &conn.channel))?;
                        }
                    }
                }
            }
        }
    } else {
        println!("Could not connect to {}", &conn.server);
    }

    Ok(())
}
