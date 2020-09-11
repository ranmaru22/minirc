use crate::connection::Connection;
use std::io::prelude::*;
use std::io::Result;
use std::net::TcpStream;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn parsing_privmsg_works() {
        let test_str =
            String::from(":Ranmaru!~ranmaru@2a02:908:13b2:5380:6c18:852b:8306:ac33 PRIVMSG ##rantestfoobazinga1337 :Foo! :D");
        let expected = String::from("<Ranmaru> Foo! :D");
        let expected_with_target = (
            Origin::User("Ranmaru"),
            Target::Single("##rantestfoobazinga1337"),
            String::from("<Ranmaru> Foo! :D"),
        );
        assert_eq!(parse_msg(&test_str), Some(expected));
        assert_eq!(parse_msg_with_target(&test_str), Some(expected_with_target));
    }

    #[test]
    pub fn parsing_notice_works() {
        let test_str =
            String::from(":niven.freenode.net NOTICE * :*** Looking up your hostname...");
        let expected = String::from("NOTICE *** Looking up your hostname...");
        assert_eq!(parse_msg(&test_str), Some(expected));
    }
}

#[derive(Debug, PartialEq)]
pub enum Target<'msg> {
    Single(&'msg str),
    All,
}

#[derive(Debug, PartialEq)]
pub enum Origin<'msg> {
    User(&'msg str),
    Server,
}

#[macro_export]
macro_rules! send_cmd {
    ($cmd:expr => $to:expr) => {
        let bytes = format!("{}\r\n", $cmd).into_bytes();
        $to.write_all(&bytes.into_boxed_slice())?;
    };
}

pub fn send_auth(conn: &Connection, stream: &mut TcpStream) -> Result<()> {
    // returns a Result type to catch errors in the marco
    let nick_cmd = format!("NICK {}", &conn.username);
    let user_cmd = format!("USER {0} * * :{0}", &conn.username);
    send_cmd!(nick_cmd => stream);
    send_cmd!(user_cmd => stream);
    Ok(())
}

pub fn print_msg(message: &str) {
    if let Some(msg) = parse_msg(message) {
        println!("{}", &msg);
    }
}

pub fn parse_msg(message: &str) -> Option<String> {
    if let Some((.., msg)) = parse_msg_with_target(message) {
        return Some(msg);
    }
    None
}

pub fn parse_msg_with_target(message: &str) -> Option<(Origin<'_>, Target<'_>, String)> {
    match message {
        msg if msg.contains("PRIVMSG") => parse_privmsg(&msg),
        msg if msg.contains("NOTICE") => parse_notice(&msg),
        &_ => None,
    }
}

fn parse_privmsg(message: &str) -> Option<(Origin<'_>, Target<'_>, String)> {
    let mut split = message.trim().splitn(2, "PRIVMSG");
    if let (Some(name), Some(msg)) = (split.next(), split.next()) {
        let name = &name.split('!').next().unwrap()[1..];
        let mut msg_split = msg.splitn(2, ':');
        let target = msg_split.next().unwrap().trim();
        let msg = msg_split.next().unwrap().trim();
        return Some((
            Origin::User(name),
            Target::Single(target),
            format!("<{}> {}", name, msg),
        ));
    }
    None
}

fn parse_notice(message: &str) -> Option<(Origin<'_>, Target<'_>, String)> {
    let split = message.trim().splitn(2, "NOTICE");
    if let Some(notice) = split.last() {
        let notice = notice.splitn(2, ':').last().unwrap();
        return Some((Origin::Server, Target::All, format!("NOTICE {}", notice)));
    }
    None
}

pub fn pong(inp: &str, stream: &mut TcpStream) -> Result<()> {
    // returns a Result type to catch errors in the marco
    if let Some(resp) = inp.splitn(2, ':').last() {
        let pong_cmd = format!("PONG :{}", &resp);
        send_cmd!(pong_cmd => stream);
    }
    Ok(())
}
