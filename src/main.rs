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

struct Message {
    header: String,
    content: String,
}

impl Message {
    pub fn from_buffer(buf: &[u8; 512]) -> Self {
        let as_utf8 = str::from_utf8(buf).expect("Invalid Message");
        let split = as_utf8.split(':').collect::<Vec<_>>();
        let (header, content) = (split[1].to_owned(), split[2].to_owned());
        Message { header, content }
    }
}

fn format_cmd(cmd: &str, msg: &str) -> Box<[u8]> {
    let bytes = format!("{} {}\r\n", cmd, msg).into_bytes();
    bytes.into_boxed_slice()
}

fn send_auth(conn: &Connection, stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write_all(&*format_cmd("NICK", &conn.username))?;
    stream.write_all(&*format_cmd(
        "USER",
        &format!("{0} * * :{0}", &conn.username),
    ))?;
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

    if let Ok(mut stream) = TcpStream::connect(conn.address()) {
        println!("Connected to {}", &conn.server);
        let mut buf;

        loop {
            buf = [0; 512];
            stream.read(&mut buf)?;
            let message = Message::from_buffer(&buf);

            println!("{}", &message.content);

            match message {
                Message { header, content } if header.contains("PING") => {
                    println!("Recevied a PING");
                    stream.write_all(&*format_cmd("PONG", &content))?;
                    println!("Sent a PONG");
                }
                Message { header, content } if header.contains("MOTD") => {
                    println!("Recevied MOTD");
                    println!("MOTD - {}", content);
                }
                Message { header, content } if header.contains("PRIVMSG") => {
                    println!(
                        "<{}> {}",
                        header.split_whitespace().collect::<Vec<_>>()[0],
                        content
                    );
                }
                Message { content, .. } if content.contains("No Ident response") => {
                    println!("Sending AUTH");
                    send_auth(&conn, &mut stream)?;
                }
                Message { content, .. } if content.contains("376") => {
                    println!("Joining channel");
                    stream.write_all(&*format_cmd("JOIN", &conn.channel))?;
                }
                _ => continue,
            }
        }
    } else {
        println!("Could not connect to {}", &conn.server);
    }
    Ok(())
}
