use crate::interface::Interface;

pub struct Buffer {
    pub name: &str,
    pub log: Vec<String>,
}

impl Buffer {
    pub fn new(name: &str, itf: &interface) -> Self {
        if let Some(pos) = itf.get_channel_pos(name) {
            let log_str = itf.read_channel(pos);
        }
    }
}
