use crate::channel::Channel;
use std::io::Result;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct Interface {
    channels: Mutex<Vec<Channel>>,
    active_channel: AtomicUsize,
    shutdown_flag: AtomicBool,
}

impl Interface {
    pub fn new(server: &str) -> Self {
        let root = vec![Channel::new(server, server)];
        let channels = Mutex::new(root);
        let active_channel = AtomicUsize::new(0);
        let shutdown_flag = AtomicBool::new(false);

        Self {
            channels,
            active_channel,
            shutdown_flag,
        }
    }

    pub fn channels_len(&self) -> usize {
        let channels = self.channels.lock().unwrap();
        channels.len()
    }

    pub fn get_channel_pos(&self, chan: &str) -> Option<usize> {
        let channels = self.channels.lock().unwrap();
        let mut names = channels.iter().map(|c| c.get_id());
        names.position(|c| c == chan)
    }

    pub fn is_active(&self, chan: &str) -> bool {
        if let Some(pos) = self.get_channel_pos(chan) {
            pos == self.get_active_channel_pos()
        } else {
            false
        }
    }

    pub fn get_channel(&self, pos: usize) -> Option<String> {
        let channels = self.channels.lock().unwrap();
        match channels.get(pos) {
            Some(chan) => Some(chan.get_id().to_owned()),
            None => None,
        }
    }

    pub fn get_active_channel(&self) -> String {
        let channels = self.channels.lock().unwrap();
        channels
            .get(self.get_active_channel_pos())
            .unwrap()
            .get_id()
            .to_owned()
    }

    pub fn push_channel(&self, chan: Channel) {
        let mut channels = self.channels.lock().unwrap();
        channels.push(chan);
    }

    pub fn remove_channel(&self, pos: usize) {
        let mut channels = self.channels.lock().unwrap();
        channels.remove(pos);
    }

    pub fn write_to_chan(&self, pos: usize, s: &str) -> Result<()> {
        let mut channels = self.channels.lock().unwrap();
        if let Some(ref mut chan) = channels.get_mut(pos) {
            chan.write(s)?;
        }
        Ok(())
    }

    pub fn get_active_channel_pos(&self) -> usize {
        self.active_channel.load(Ordering::Relaxed)
    }

    pub fn store_active_channel(&self, n: usize) {
        self.active_channel.store(n, Ordering::Relaxed);
    }

    pub fn set_shutdown_flag(&self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
    }

    pub fn should_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::Relaxed)
    }
}
