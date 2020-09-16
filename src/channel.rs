const CONFIG_PATH: &str = ".config/minirc/";

use std::env;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Channel {
    id: String,
    server: String,
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
            File::create(&fp).expect("Error writing buffer file");
        }

        let fp = fp.canonicalize().expect("Error resolving file path");
        let (id, server) = (id.to_owned(), server.to_owned());
        Self { id, server, fp }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn write(&mut self, message: &str) -> std::io::Result<()> {
        let mut file = OpenOptions::new().write(true).append(true).open(&self.fp)?;
        write!(file, "{}", message)?;
        Ok(())
    }
}
