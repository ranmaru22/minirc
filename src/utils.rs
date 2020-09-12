use crate::command::Command;
use crate::connection::Connection;
use std::io::Result;
use std::net::TcpStream;

/// Sends the auth (PASS, USER, NICK) commands to the server.
/// Returns an IO Result to catch errors while sending.
pub fn send_auth(conn: &Connection, stream: &mut TcpStream) -> Result<()> {
    if let Some(ref passwd) = &conn.password {
        Command::Pass(passwd).send(stream)?;
    }
    Command::Nick(&conn.username).send(stream)?;
    Command::User(&conn.username, &conn.username).send(stream)?;
    Ok(())
}
