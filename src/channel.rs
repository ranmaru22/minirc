use crate::CONFIG_PATH;
use std::env;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Channel {
    id: String,
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
        if !fp.exists() {
            File::create(&fp).expect("Error writing buffer file");
        }

        let fp = fp.canonicalize().expect("Error resolving file path");

        Self { id, fp }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn write(&mut self, message: &str) -> std::io::Result<()> {
        let mut file = OpenOptions::new().write(true).append(true).open(&self.fp)?;
        writeln!(file, "{}", message)?;
        Ok(())
    }
}
