pub struct Connection {
    pub address: String,
    pub channel: Option<String>,
    pub username: String,
}

impl Connection {
    pub fn new(server: String, port: String, channel: String, username: String) -> Self {
        Self {
            address: format!("{}:{}", server, port),
            channel: Connection::parse_channel(channel),
            username,
        }
    }

    fn parse_channel(channel: String) -> Option<String> {
        match channel {
            c if c.is_empty() => None,
            c if c.starts_with('#') => Some(c),
            c => Some(format!("#{}", c)),
        }
    }
}
