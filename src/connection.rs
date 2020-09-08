pub struct Connection {
    pub address: String,
    pub username: String,
}

impl Connection {
    pub fn new(server: String, port: String, username: String) -> Self {
        Self {
            address: format!("{}:{}", server, port),
            username,
        }
    }
}
