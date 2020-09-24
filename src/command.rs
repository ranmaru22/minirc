use crate::connection::Connection;
use std::io::{Result, Write};
use std::net::TcpStream;

#[derive(Debug, PartialEq)]
pub enum Command {
    Privmsg(String, String, String), // Sender, target, message
    Notice(String, String, String),  // Sender, target, message
    Ping(String),                    // Payload
    Pong(String),                    // Payload
    Pass(String),                    // Password
    User(String, String),            // Username, realname
    Nick(String),                    // Nickname
    Join(Vec<String>),               // Channels
    Part(Vec<String>),               // Channels
    Quit(String),                    // Quitmsg
    Internal(UiCommand),             // Internal commands
    Unknown,
}

#[derive(Debug, PartialEq)]
pub enum UiCommand {
    PrintBuffers,
    SwitchBuffer(usize),
}

impl Command {
    /// Used to write to stdout or logs.
    /// Returns a printable string from a command type.
    pub fn to_printable(&self) -> Option<String> {
        match self {
            Self::Privmsg(sender, _, msg) => Some(format!("<{}> {}", sender, msg.trim())),
            Self::Notice(.., msg) => Some(format!("-> {}", msg.trim())),
            _ => None,
        }
    }

    /// Sends a sendable command type to a stream.
    pub fn send(&self, stream: &mut TcpStream) -> Result<()> {
        let bytes = self.to_string().into_bytes();
        if !bytes.is_empty() {
            stream.write_all(&bytes)?;
        }
        Ok(())
    }
}

impl ToString for Command {
    fn to_string(&self) -> String {
        match self {
            Self::Privmsg(_, target, msg) => format!("PRIVMSG {} :{}\r\n", target, msg),
            Self::Notice(_, target, msg) => format!("NOTICE {} :{}\r\n", target, msg),
            Self::Ping(payload) => format!("PING {}\r\n", payload),
            Self::Pong(payload) => format!("PONG {}\r\n", payload),
            Self::Pass(passwd) => format!("PASS {}\r\n", passwd),
            Self::User(username, realname) => format!("USER {} * * :{}\r\n", username, realname),
            Self::Nick(nickname) => format!("NICK {}\r\n", nickname),
            Self::Join(channels) => {
                let channels = channels.join(",");
                format!("JOIN {}\r\n", channels)
            }
            Self::Part(channels) => {
                let channels = channels.join(",");
                format!("PART {}\r\n", channels)
            }
            Self::Quit(quitmsg) => format!("QUIT :{}\r\n", quitmsg),
            Self::Internal(_) | Self::Unknown => String::default(),
        }
    }
}

impl From<String> for Command {
    fn from(inp: String) -> Self {
        Self::from(inp.as_ref())
    }
}

impl From<&str> for Command {
    fn from(inp: &str) -> Self {
        let mut split = inp.split_whitespace();

        let sender = match split.next() {
            Some(sender) => sender.split('!').next().unwrap().trim()[1..].to_owned(),
            None => String::default(),
        };

        match split.next() {
            Some("PRIVMSG") => {
                let target = split.next().unwrap_or_default();
                let target = target.split('!').next().unwrap_or_default().to_owned();
                let mut msg = inp
                    .splitn(4, char::is_whitespace)
                    .last()
                    .unwrap_or_default()
                    .to_owned();
                if msg.starts_with(':') {
                    msg.remove(0);
                }
                Self::Privmsg(sender, target, msg)
            }

            Some("NOTICE") => {
                let target = split.next().unwrap_or_default().to_owned();
                let mut msg = inp
                    .splitn(4, char::is_whitespace)
                    .last()
                    .unwrap_or_default()
                    .to_owned();
                if msg.starts_with(':') {
                    msg.remove(0);
                }
                Self::Notice(sender, target, msg)
            }

            Some("PING") => {
                let msg = inp
                    .splitn(3, char::is_whitespace)
                    .last()
                    .unwrap_or_default()
                    .to_owned();
                Self::Ping(msg)
            }

            Some("PONG") => {
                let msg = inp
                    .splitn(3, char::is_whitespace)
                    .last()
                    .unwrap_or_default()
                    .to_owned();
                Self::Pong(msg)
            }

            _ => Self::Unknown,
        }
    }
}

pub fn send_auth(conn: &Connection, stream: &mut TcpStream) -> Result<()> {
    if let Some(passwd) = &conn.password {
        let passwd = passwd.to_owned();
        Command::Pass(passwd).send(stream)?;
    }
    let nickname = conn.username.to_owned();
    let username = conn.username.to_owned();
    let realname = conn.username.to_owned();
    Command::Nick(nickname).send(stream)?;
    Command::User(username, realname).send(stream)?;
    Ok(())
}
