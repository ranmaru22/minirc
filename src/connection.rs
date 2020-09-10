#[derive(Debug)]
pub struct Connection {
    pub address: String,
    pub server: String,
    pub password: Option<String>,
    pub username: String,
}

impl Connection {
    pub fn new(server: String, port: String, password: String, username: String) -> Self {
        Self {
            address: format!("{}:{}", server, port),
            server,
            password: match password {
                p if p.is_empty() => None,
                p => Some(p),
            },
            username,
        }
    }
}
