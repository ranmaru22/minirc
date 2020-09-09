use crate::CONFIG_PATH;
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Channel {
    id: String,
    file: File,
    fp: PathBuf,
}

impl Channel {
    pub fn new(id: String) -> Self {
        let mut path = match env::var("HOME") {
            Ok(home) => Path::new(&home).join(CONFIG_PATH),
            Err(e) => panic!("Error reading HOME: {}", e),
        };

        if !path.exists() {
            Command::new("mkdir")
                .arg("-r")
                .arg(&path)
                .status()
                .expect("Error creating config dir");
        }
        path.set_file_name(&id);
        path.set_extension("txt");

        println!("{:?}", &path);
        let file = if fs::metadata(&path).is_ok() {
            File::open(&path).expect("Error writing buffer file")
        } else {
            File::create(&path).expect("Error opening buffer file")
        };

        let fp = path.canonicalize().expect("Error resolving file path");

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
        if let Some(fp) = self.fp.to_str() {
            fp
        } else {
            ""
        }
    }
}
