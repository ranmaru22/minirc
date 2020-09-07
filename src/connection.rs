pub struct Connection {
    server: String,
    port: u16,
    pub channel: String,
    pub username: String,
}

impl Connection {
    pub fn new(server: String, port: u16, channel: String, username: String) -> Self {
        Self {
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
