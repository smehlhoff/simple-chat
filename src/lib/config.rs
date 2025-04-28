use std::{error, fs};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub loki_address: String,
    pub server_address: String,
}

impl Config {
    pub fn load_config() -> Result<Self, Box<dyn error::Error>> {
        let file = fs::OpenOptions::new().read(true).open("config.json")?;
        let json: Self = serde_json::from_reader(file)?;

        Ok(json)
    }
}
