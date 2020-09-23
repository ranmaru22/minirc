use crate::channel::Channel;

pub struct Buffer<'chan> {
    pub chan: &'chan Channel,
    pub log: Vec<String>,
}

impl<'chan> From<&'chan Channel> for Buffer<'chan> {
    fn from(chan: &'chan Channel) -> Self {
        let log_str = chan.read_log().unwrap_or(String::default());
        let log = log_str.split("\r\n").map(|s| s.to_owned()).collect();

        Self { chan, log }
    }
}

impl<'chan> Buffer<'chan> {
    fn push_to_log(&mut self, line: &str) {
        &self.log.push(line.to_owned());
    }
}
