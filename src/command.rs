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
}

impl Command {
    /// Returns a sendable string from a command type.
    pub fn to_string(&self) -> Option<String> {
        match self {
            Self::Privmsg(_, target, msg) => Some(format!("PRIVMSG {} :{}\r\n", target, msg)),
            Self::Notice(_, target, msg) => Some(format!("NOTICE {} :{}\r\n", target, msg)),
            Self::Ping(payload) => Some(format!("PING {}\r\n", payload)),
            Self::Pong(payload) => Some(format!("PONG {}\r\n", payload)),
            Self::Pass(passwd) => Some(format!("PASS {}\r\n", passwd)),
            Self::User(username, realname) => {
                Some(format!("USER {} * * :{}\r\n", username, realname))
            }
            Self::Nick(nickname) => Some(format!("NICK {}\r\n", nickname)),
            Self::Join(channels) => {
                let channels = channels.join(",");
                Some(format!("JOIN {}\r\n", channels))
            }
            Self::Part(channels) => {
                let channels = channels.join(",");
                Some(format!("PART {}\r\n", channels))
            }
            Self::Quit(quitmsg) => Some(format!("QUIT :{}\r\n", quitmsg)),
            Self::Internal(_) | Self::Unknown => None,
        }
    }

    /// Returns a raw sendable string from a command type.
    /// Returns an empty string on unknown commands.
    pub fn to_unwrapped_string(&self) -> String {
        self.to_string().unwrap_or_default()
    }

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
        if let Some(cmd) = self.to_string() {
            let bytes = cmd.into_bytes();
            stream.write_all(&bytes)?;
        }
        Ok(())
    }
}

impl From<String> for Command {
    fn from(inp: String) -> Self {
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
