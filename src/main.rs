#![warn(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]

mod lib {
    pub mod client;
    pub mod commands;
    pub mod connection;
    pub mod utils;
}

use std::sync::Arc;

use tokio::{
    io::{self},
    net::TcpListener,
    sync::broadcast,
};

use lib::{client, connection};

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:6667").await?;
    let (tx, _) = broadcast::channel::<String>(32);
    let clients = Arc::new(client::Clients::new());

    loop {
        let (stream, addr) = listener.accept().await?;
        let tx = tx.clone();
        let clients = Arc::clone(&clients);

        tokio::spawn(async move { connection::handle_client(stream, addr, tx, clients).await });
    }
}
