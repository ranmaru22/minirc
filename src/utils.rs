use std::io::prelude::*;
use std::io::Result;
use std::net::TcpStream;

use crate::connection::Connection;

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
    let resp: Vec<_> = message.trim().split(':').collect();
    let name: Vec<_> = resp[1].split('!').collect();
    let text = resp.last().unwrap();
    println!("<{}> {}", name[0], text);
    Ok(())
}

pub fn pong(inp: &str, stream: &mut TcpStream) -> Result<()> {
    let resp = inp.split(':').collect::<Vec<_>>().join("");
    let pong_cmd = format!("PONG :{}", resp);
    send_cmd!(pong_cmd => stream);
    Ok(())
}
