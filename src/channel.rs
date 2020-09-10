use crate::CONFIG_PATH;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Channel {
    id: String,
    file: File,
    fp: PathBuf,
}

impl Channel {
    pub fn new(id: String) -> Self {
        let path = match env::var("HOME") {
            Ok(home) => Path::new(&home).join(CONFIG_PATH).join("logs"),
            Err(e) => panic!("Error reading HOME: {}", e),
        };

        if !path.exists() {
            create_dir_all(&path).expect("Error creating logs directory");
        }

        let mut fp = path.join(&id);
        fp.set_extension("txt");

        let file = if fp.exists() {
            File::open(&fp).expect("Error opening buffer file")
        } else {
            File::create(&fp).expect("Error writing buffer file")
        };

        let fp = fp.canonicalize().expect("Error resolving file path");
        Self { id, file, fp }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn write(&mut self, message: &str) -> std::io::Result<()> {
        self.file.write(message.as_bytes())?;
        Ok(())
    }

    pub fn get_fp(&self) -> Option<&str> {
        self.fp.to_str()
    }
}
