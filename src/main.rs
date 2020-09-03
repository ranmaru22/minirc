use std::env;
use std::io::prelude::*;
use std::net::TcpStream;
use std::str;

struct Connection {
    server: String,
    port: u16,
    channel: String,
    username: String,
}

impl Connection {
    pub fn new(server: String, port: u16, channel: String, username: String) -> Self {
        Connection {
            server,
            port,
            channel: Connection::parse_channel(channel),
            username,
        }
    }

    fn parse_channel(channel: String) -> String {
        match channel {
            c if c.starts_with('#') => c,
            c => format!("#{}", c),
        }
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.server, self.port)
    }
}

macro_rules! send_cmd {
    ($cmd:literal, $msg:expr => $to:expr) => {
        let bytes = format!("{} {}\r\n", $cmd, $msg).into_bytes();
        $to.write_all(&bytes.into_boxed_slice())?;
    };
}

fn send_auth(conn: &Connection, stream: &mut TcpStream) -> std::io::Result<()> {
    let user_cmd = format!("{0} * * {0}", &conn.username);
    send_cmd!("NICK", &conn.username => stream);
    send_cmd!("USER", user_cmd => stream);
    Ok(())
}

fn print_msg(message: &str) -> std::io::Result<()> {
    let resp = message.trim().split(':').collect::<Vec<_>>();
    let name = resp[1].split('!').collect::<Vec<_>>();
    let text = resp.last().unwrap();
    println!("<{}> {}", name[0], text);
    Ok(())
}

fn main() -> std::io::Result<()> {
    let argv: Vec<String> = env::args().collect();

    let conn = Connection::new(
        argv[1].to_owned(),
        argv[2].parse().expect("Invalid port number"),
        argv[3].to_owned(),
        argv[4].to_owned(),
    );

    #[allow(clippy::unused_io_amount)]
    if let Ok(mut stream) = TcpStream::connect(conn.address()) {
        println!("Connected to {}", &conn.server);

        let joined = loop {
            let mut buf = [0; 512];
            stream.read(&mut buf)?;
            let message = str::from_utf8(&buf).expect("Invalid Message");
            println!("{}", &message);

            if message.contains("PING") {
                let resp = message.split(':').collect::<Vec<_>>().join("");
                let pong_cmd = format!(":{}", resp);
                send_cmd!("PONG", pong_cmd => stream);
            }

            if message.contains("No Ident response") {
                send_auth(&conn, &mut stream)?;
            }

            if message.contains("376") {
                send_cmd!("JOIN", &conn.channel => stream);
            }

            if message.contains("366") {
                break true;
            }
        };

        while joined {
            let mut buf = [0; 512];
            stream.read(&mut buf)?;
            let message = str::from_utf8(&buf).expect("Invalid Message");
            println!("{}", &message);

            if message.contains("PRIVMSG") {
                print_msg(message)?;
            }
        }
    } else {
        println!("Could not connect to {}", &conn.server);
    }
    Ok(())
}
