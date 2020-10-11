const CONFIG_PATH: &str = ".config/minirc/";
const MAX_HIST: usize = 1000;

use std::collections::VecDeque;
use std::env;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{prelude::*, Result, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Channel {
    id: String,
    server: String,
    buffer: VecDeque<String>,
    fp: PathBuf,
}

impl Channel {
    pub fn new(id: &str, server: &str) -> Self {
        let path = match env::var("HOME") {
            Ok(home) => Path::new(&home)
                .join(CONFIG_PATH)
                .join("logs")
                .join(&server),
            Err(e) => panic!("Error reading HOME: {}", e),
        };

        if !path.exists() {
            create_dir_all(&path).expect("Error creating logs directory");
        }

        let mut fp = path.join(&id);
        fp.set_extension("txt");
        if !fp.exists() {
            File::create(&fp).expect("Error creating buffer file");
        }

        let mut buffer = VecDeque::with_capacity(MAX_HIST);
        let mut file = File::open(&fp).expect("Error opening buffer file");
        let mut file_buf = String::new();
        let file_size = file.metadata().unwrap().len();
        let chunk_size = 512 * MAX_HIST as u64;
        let start_pos = if file_size < chunk_size {
            0
        } else {
            file_size - chunk_size
        };
        file.seek(SeekFrom::Start(start_pos)).unwrap();
        file.take(chunk_size).read_to_string(&mut file_buf).unwrap();

        for (i, line) in file_buf.lines().rev().enumerate() {
            if i > MAX_HIST {
                break;
            }
            buffer.push_front(format!("{}\r\n", line));
        }

        // Remove oldest line as it's likely cut off in the middle.
        buffer.pop_front();

        let fp = fp.canonicalize().expect("Error resolving file path");
        Self {
            id: id.to_owned(),
            server: server.to_owned(),
            buffer,
            fp,
        }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_buffer(&self) -> &VecDeque<String> {
        &self.buffer
    }

    pub fn write(&mut self, message: &str) -> Result<()> {
        let mut file = OpenOptions::new().write(true).append(true).open(&self.fp)?;
        write!(file, "{}\r\n", message)?;
        if self.buffer.len() > MAX_HIST {
            self.buffer.pop_front();
        }
        self.buffer.push_back(format!("{}\r\n", message));
        Ok(())
    }
}
