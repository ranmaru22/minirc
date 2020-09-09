use crate::CONFIG_PATH;
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;

pub struct Channel {
    id: String,
    file: File,
    fp: String,
}

impl Channel {
    pub fn new(id: String) -> Self {
        let path = match env::var("HOME") {
            Ok(home) => format!("{}{}", home, CONFIG_PATH),
            Err(e) => panic!("Error reading HOME: {}", e),
        };

        let fp = format!("{}/{}.txt", path, id);
        let file = if fs::metadata(&fp).is_ok() {
            File::open(&fp).expect("Error writing buffer file")
        } else {
            File::create(&fp).expect("Error opening buffer file")
        };

        Self { id, file, fp }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn write(&mut self, message: &str) -> std::io::Result<()> {
        self.file.write(message.as_bytes())?;
        Ok(())
    }

    pub fn get_fp(&self) -> &str {
        &self.fp
    }
}
