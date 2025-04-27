#![warn(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]

mod lib {
    pub mod client;
    pub mod commands;
    pub mod config;
    pub mod connection;
    pub mod utils;
}

use std::sync::Arc;

use tokio::{
    io::{self},
    net::TcpListener,
    sync::broadcast,
};

use tracing::{error, info};

use lib::{client, config, connection};

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let config = match config::Config::load_config() {
        Ok(config) => config,
        Err(e) => {
            error!("error with config file: {}", e);
            panic!("error with config file: {}", e)
        }
    };
    let listener = TcpListener::bind(&config.server_address).await?;
    let (tx, _) = broadcast::channel::<String>(32);
    let clients = Arc::new(client::Clients::new());

    info!("TCP server running: {}", config.server_address);

    loop {
        let (stream, addr) = listener.accept().await?;
        let tx = tx.clone();
        let clients = Arc::clone(&clients);

        tokio::spawn(async move { connection::handle_client(stream, addr, tx, clients).await });
    }
}
