use std::io::prelude::*;
use std::io::Result;
use std::net::TcpStream;

use crate::connection::Connection;

#[macro_export]
macro_rules! send_cmd {
    ($cmd:literal, $msg:expr => $to:expr) => {
        let bytes = format!("{} {}\r\n", $cmd, $msg).into_bytes();
        $to.write_all(&bytes.into_boxed_slice())?;
    };
}

pub fn send_auth(conn: &Connection, stream: &mut TcpStream) -> Result<()> {
    let user_cmd = format!("{0} * * {0}", &conn.username);
    send_cmd!("NICK", &conn.username => stream);
    send_cmd!("USER", user_cmd => stream);
    Ok(())
}

pub fn print_msg(message: &str) -> Result<()> {
    let resp = message.trim().split(':').collect::<Vec<_>>();
    let name = resp[1].split('!').collect::<Vec<_>>();
    let text = resp.last().unwrap();
    println!("<{}> {}", name[0], text);
    Ok(())
}

pub fn pong(inp: &str, stream: &mut TcpStream) -> Result<()> {
    let resp = inp.split(':').collect::<Vec<_>>().join("");
    let pong_cmd = format!(":{}", resp);
    send_cmd!("PONG", pong_cmd => stream);
    Ok(())
}
