#![warn(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]

use std::sync::Arc;

mod lib {
    pub mod client;
    pub mod commands;
    pub mod config;
    pub mod connection;
    pub mod utils;
}

use lib::{client, config, connection, utils};

use tokio::{net::TcpListener, sync::broadcast};
use tracing::info;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let config = config::Config::load_config().unwrap();
    let listener = TcpListener::bind(&config.server_address).await.unwrap();
    let (tx, _) = broadcast::channel::<String>(32);
    let clients = Arc::new(client::Clients::new());
    let cloned_clients = clients.clone();

    utils::logger(&config).unwrap();

    info!(uuid=?Uuid::new_v4(), "TCP server running: {}", config.server_address);

    tokio::spawn(async move { utils::clients_connected_background(cloned_clients).await });

    loop {
        let (stream, addr) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let clients = Arc::clone(&clients);

        info!(uuid=?Uuid::new_v4(), ip=?addr, "client connected");

        tokio::spawn(async move { connection::handle_client(stream, addr, tx, clients).await });
    }
}
