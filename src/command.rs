use std::io::{Result, Write};
use std::net::TcpStream;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn parsing_privmsg_works() {
        let test_str = ":Ranmaru!~ranmaru@2a02:908:13b2:5380:6c18:852b:8306:ac33 PRIVMSG ##rantestfoobazinga1337 :Foo! :D";
        let expected = Command::Privmsg("Ranmaru", "##rantestfoobazinga1337", "Foo! :D");
        let as_sent = String::from("PRIVMSG ##rantestfoobazinga1337 :Foo! :D\r\n");
        assert_eq!(Command::from_str(test_str), expected);
        assert_eq!(expected.to_string(), Some(as_sent));
    }

    #[test]
    pub fn parsing_notice_works() {
        let test_str = ":niven.freenode.net NOTICE * :*** Looking up your hostname...";
        let expected =
            Command::Notice("niven.freenode.net", "*", "*** Looking up your hostname...");
        let as_sent = String::from("NOTICE * :*** Looking up your hostname...\r\n");
        assert_eq!(Command::from_str(test_str), expected);
        assert_eq!(expected.to_string(), Some(as_sent));
    }

    #[test]
    pub fn parsing_ping_works() {
        let test_str = ":niven.freenode.net PING :pong me back";
        let expected = Command::Ping(":pong me back");
        let as_sent = String::from("PING :pong me back\r\n");
        assert_eq!(Command::from_str(test_str), expected);
        assert_eq!(expected.to_string(), Some(as_sent));
    }

    #[test]
    pub fn parsing_pong_works() {
        let test_str =
            ":Ranmaru!~ranmaru@2a02:908:13b2:5380:6c18:852b:8306:ac33 PONG :pong pong pong";
        let expected = Command::Pong(":pong pong pong");
        let as_sent = String::from("PONG :pong pong pong\r\n");
        assert_eq!(Command::from_str(test_str), expected);
        assert_eq!(expected.to_string(), Some(as_sent));
    }

    #[test]
    pub fn sending_join_and_part_works() {
        let single = Command::Join(&["##foo"]);
        let multiple = Command::Join(&["##foo", "#bar", "##baz"]);
        let expected = String::from("JOIN ##foo\r\n");
        let expected_mult = String::from("JOIN ##foo,#bar,##baz\r\n");
        assert_eq!(single.to_string(), Some(expected));
        assert_eq!(multiple.to_string(), Some(expected_mult));
    }

    #[test]
    pub fn printing_works() {
        let privmsg = Command::Privmsg("Ranmaru", "##foo", "Hello World!");
        assert_eq!(
            privmsg.to_printable().unwrap(),
            String::from("<Ranmaru> Hello World!")
        );
    }
}

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
    /// Parses a receivable str into a command type.
    pub fn from_str(inp: &'msg str) -> Self {
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

            None | _ => Self::Unknown,
        }
    }

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
