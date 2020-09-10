use std::io::prelude::*;
use std::io::Result;
use std::net::TcpStream;

use crate::connection::Connection;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn parsing_privmsg_works() {
        let test_str = String::from(":Ranmaru!~ranmaru@2a02:908:13b2:5380:6c18:852b:8306:ac33 PRIVMSG ##rantestfoobazinga1337 :Foo! :D");
        let expected = String::from("<Ranmaru> Foo! :D");
        assert_eq!(parse_msg(&test_str), Some(expected));
    }

    #[test]
    pub fn parsing_notice_works() {
        let test_str =
            String::from(":niven.freenode.net NOTICE * :*** Looking up your hostname...");
        let expected = String::from("NOTICE *** Looking up your hostname...");
        assert_eq!(parse_msg(&test_str), Some(expected));
    }
}

#[macro_export]
macro_rules! send_cmd {
    ($cmd:expr => $to:expr) => {
        let bytes = format!("{}\r\n", $cmd).into_bytes();
        $to.write_all(&bytes.into_boxed_slice())?;
    };
}

pub fn send_auth(conn: &Connection, stream: &mut TcpStream) -> Result<()> {
    let nick_cmd = format!("NICK {}", &conn.username);
    let user_cmd = format!("USER {0} * * :{0}", &conn.username);
    send_cmd!(nick_cmd => stream);
    send_cmd!(user_cmd => stream);
    Ok(())
}

pub fn print_msg(message: &str) -> Result<()> {
    if let Some(msg) = parse_msg(message) {
        println!("{}", &msg);
    }
    Ok(())
}

pub fn parse_msg(message: &str) -> Option<String> {
    match message {
        msg if msg.contains("PRIVMSG") => parse_privmsg(&msg),
        msg if msg.contains("NOTICE") => parse_notice(&msg),
        &_ => None,
    }
}

fn parse_privmsg(message: &str) -> Option<String> {
    let mut split = message.trim().splitn(2, "PRIVMSG");
    if let (Some(name), Some(msg)) = (split.next(), split.next()) {
        let name = &name.split('!').next().unwrap()[1..];
        let msg = msg.splitn(2, ':').last().unwrap();
        return Some(format!("<{}> {}", name, msg));
    }
    None
}

fn parse_notice(message: &str) -> Option<String> {
    let split = message.trim().splitn(2, "NOTICE");
    if let Some(notice) = split.last() {
        let notice = notice.splitn(2, ':').last().unwrap();
        return Some(format!("NOTICE {}", notice));
    }
    None
}

pub fn pong(inp: &str, stream: &mut TcpStream) -> Result<()> {
    if let Some(resp) = inp.splitn(2, ':').last() {
        let pong_cmd = format!("PONG :{}", &resp);
        send_cmd!(pong_cmd => stream);
    }
    Ok(())
}
