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
        &_ => None,
    }
}

fn parse_privmsg(message: &str) -> Option<String> {
    let split: Vec<_> = message.trim().split("PRIVMSG").collect();
    if let Some((left, right)) = split.split_first() {
        let name = left.split('!').collect::<Vec<_>>().first().unwrap()[1..].to_owned();
        let mut msg = right[0].split(':').collect::<Vec<_>>();
        let _ = msg.remove(0);
        return Some(format!("<{}> {}", name, msg.join(":")));
    }
    None
}

pub fn pong(inp: &str, stream: &mut TcpStream) -> Result<()> {
    let resp = inp.split(':').collect::<Vec<_>>().join("");
    let pong_cmd = format!("PONG :{}", resp);
    send_cmd!(pong_cmd => stream);
    Ok(())
}
