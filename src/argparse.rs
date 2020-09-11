// DEFAULT PARAMETERS
const DEFAULT_SERVER: &str = "chat.freenode.net";
const DEFAULT_PORT: &str = "6667";
const DEFAULT_USERNAME: &str = "minirc_user";

use crate::connection::Connection;
use argparse::{ArgumentParser, Store};
use std::io::Result;

pub fn setup() -> Result<Connection> {
    let mut server = String::from(DEFAULT_SERVER);
    let mut port = String::from(DEFAULT_PORT);
    let mut passwd = String::new();
    let mut uname = String::from(DEFAULT_USERNAME);

    {
        // blocked so borrows go out of scope after parsing
        let mut parser = ArgumentParser::new();
        parser.set_description("Simple IRC client written in Rust.");
        parser
            .refer(&mut server)
            .add_option(&["-s", "--server"], Store, "Server to connect to");
        parser
            .refer(&mut port)
            .add_option(&["-p", "--port"], Store, "Port to connect to");
        parser
            .refer(&mut passwd)
            .add_option(&["-k", "--key"], Store, "Server password");
        parser
            .refer(&mut uname)
            .add_option(&["-n", "--name"], Store, "User handle to use");
        parser.parse_args_or_exit();
    }

    Ok(Connection::new(server, port, passwd, uname))
}
