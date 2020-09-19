use crate::channel::Channel;
use crate::connection::Connection;
use std::io::Result;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct Interface {
    channels: Mutex<Vec<Channel>>,
    conn: Mutex<Connection>,
    active_channel: AtomicUsize,
    shutdown_flag: AtomicBool,
    refresh_buffers_flag: AtomicBool,
}

impl Interface {
    pub fn new(conn: Connection) -> Self {
        let root = vec![Channel::new(&conn.server, &conn.server)];
        let conn = Mutex::new(conn);
        let channels = Mutex::new(root);
        let active_channel = AtomicUsize::new(0);
        let shutdown_flag = AtomicBool::new(false);
        let refresh_buffers_flag = AtomicBool::new(false);

        Self {
            channels,
            conn,
            active_channel,
            shutdown_flag,
            refresh_buffers_flag,
        }
    }

    /// Returns the length of the channels vector
    pub fn channels_len(&self) -> usize {
        let channels = self.channels.lock().unwrap();
        channels.len()
    }

    /// Returns the position channel chan in the vector
    pub fn get_channel_pos(&self, chan: &str) -> Option<usize> {
        let channels = self.channels.lock().unwrap();
        let mut names = channels.iter().map(|c| c.get_id());
        names.position(|c| c == chan)
    }

    /// Returns whether channel chan is the currently active channel
    pub fn is_active(&self, chan: &str) -> bool {
        if let Some(pos) = self.get_channel_pos(chan) {
            pos == self.get_active_channel_pos()
        } else {
            false
        }
    }

    /// Returns the name of the channel at position pos
    pub fn get_channel(&self, pos: usize) -> Option<String> {
        let channels = self.channels.lock().unwrap();
        match channels.get(pos) {
            Some(chan) => Some(chan.get_id().to_owned()),
            None => None,
        }
    }

    /// Returns the name of the currently active channel
    pub fn get_active_channel(&self) -> String {
        let channels = self.channels.lock().unwrap();
        channels
            .get(self.get_active_channel_pos())
            .unwrap()
            .get_id()
            .to_owned()
    }

    /// Pushes a channel to the channel vector
    pub fn push_channel(&self, chan: Channel) {
        let mut channels = self.channels.lock().unwrap();
        channels.push(chan);
    }

    /// Removes the channel at position pos from the vector
    pub fn remove_channel(&self, pos: usize) {
        let mut channels = self.channels.lock().unwrap();
        channels.remove(pos);
    }

    /// Logs a message s to the channel as position pos
    pub fn write_to_chan(&self, pos: usize, s: &str) -> Result<()> {
        let mut channels = self.channels.lock().unwrap();
        if let Some(ref mut chan) = channels.get_mut(pos) {
            chan.write(s)?;
        }
        Ok(())
    }

    /// Gets the position of the currently active channel in the vector
    pub fn get_active_channel_pos(&self) -> usize {
        self.active_channel.load(Ordering::Relaxed)
    }

    /// Changes the currently active channel
    pub fn store_active_channel(&self, n: usize) {
        self.active_channel.store(n, Ordering::Relaxed);
    }

    /// Sets the shutdown flag, causing all threads to terminate in their
    /// next iteration
    pub fn set_shutdown_flag(&self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
    }

    /// Returns whether the shutdown flag is set
    pub fn should_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::Relaxed)
    }

    /// Sets the refresh buffers flag
    pub fn set_refresh_buffers_flag(&self) {
        let arg = self.should_refresh_buffers();
        self.refresh_buffers_flag.store(true, Ordering::Relaxed);
    }

    /// Unsets the refresh buffers flag
    pub fn unset_refresh_buffers_flag(&self) {
        let arg = self.should_refresh_buffers();
        self.refresh_buffers_flag.store(false, Ordering::Relaxed);
    }

    /// Returns whether the refresh buffers flag is set
    pub fn should_refresh_buffers(&self) -> bool {
        self.refresh_buffers_flag.load(Ordering::Relaxed)
    }

    /// Returns the stored username
    pub fn get_username(&self) -> String {
        let conn = self.conn.lock().unwrap();
        conn.username.clone()
    }

    /// Returns the stored server name
    pub fn get_server(&self) -> String {
        let conn = self.conn.lock().unwrap();
        conn.server.clone()
    }
}
