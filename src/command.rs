use std::io::{Result, Write};
use std::net::TcpStream;

#[derive(Debug, PartialEq)]
pub enum Command<'msg> {
    Privmsg(&'msg str, &'msg str, &'msg str), // Sender, target, message
    Notice(&'msg str, &'msg str, &'msg str),  // Sender, target, message
    Ping(&'msg str),                          // Payload
    Pong(&'msg str),                          // Payload
    Pass(&'msg str),                          // Password
    User(&'msg str, &'msg str),               // Username, realname
    Nick(&'msg str),                          // Nick
    Join(&'msg [&'msg str]),                  // Channels
    Part(&'msg [&'msg str]),                  // Channels
    Quit(&'msg str),                          // Quitmsg
    Unknown,
}

impl<'msg> Command<'msg> {
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
            Self::Unknown => None,
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
            Self::Privmsg(sender, _, msg) => Some(format!("<{}> {}", sender, msg)),
            Self::Notice(.., msg) => Some(format!("NOTICE {}", msg)),
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

impl<'msg> From<&'msg str> for Command<'msg> {
    fn from(inp: &'msg str) -> Self {
        let mut split = inp.split_whitespace();

        let sender = match split.next() {
            Some(sender) => &sender.split('!').next().unwrap().trim()[1..],
            None => "",
        };

        match split.next() {
            Some("PRIVMSG") => {
                let target = split.next().unwrap_or_default();
                let target = &target.split('!').next().unwrap_or_default();
                let mut msg = inp
                    .splitn(4, char::is_whitespace)
                    .last()
                    .unwrap_or_default();
                if msg.starts_with(':') {
                    msg = &msg[1..];
                }
                Self::Privmsg(sender, target, msg)
            }

            Some("NOTICE") => {
                let target = split.next().unwrap_or_default();
                let mut msg = inp
                    .splitn(4, char::is_whitespace)
                    .last()
                    .unwrap_or_default();
                if msg.starts_with(':') {
                    msg = &msg[1..];
                }
                Self::Notice(sender, target, msg)
            }

            Some("PING") => {
                let msg = inp
                    .splitn(3, char::is_whitespace)
                    .last()
                    .unwrap_or_default();
                Self::Ping(msg)
            }

            Some("PONG") => {
                let msg = inp
                    .splitn(3, char::is_whitespace)
                    .last()
                    .unwrap_or_default();
                Self::Pong(msg)
            }

            _ => Self::Unknown,
        }
    }
}
