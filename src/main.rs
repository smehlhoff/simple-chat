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

use lib::{client, config, connection};

use tokio::{net::TcpListener, sync::broadcast, time::Duration};
use tracing::info;
use tracing_subscriber::{
    EnvFilter, fmt::layer as fmt_layer, layer::SubscriberExt, prelude::*, util::SubscriberInitExt,
};
use url::Url;
use uuid::Uuid;

fn logger(config: &config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let (loki_layer, task) = tracing_loki::builder()
        .label("service", "simple-chat")?
        .extra_field("environment", "development")?
        .build_url(Url::parse(&config.loki_address)?)?;
    let loki_layer =
        loki_layer.with_filter(EnvFilter::new("info").add_directive("tracing_loki=off".parse()?));
    let console_layer = fmt_layer()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_filter(EnvFilter::new("info").add_directive("tracing_loki=off".parse()?));

    tracing_subscriber::registry().with(loki_layer).with(console_layer).init();

    tokio::spawn(task);

    Ok(())
}

#[tokio::main]
async fn main() {
    let config = config::Config::load_config().unwrap();
    let listener = TcpListener::bind(&config.server_address).await.unwrap();
    let (tx, _) = broadcast::channel::<String>(32);
    let clients = Arc::new(client::Clients::new());
    let cloned_clients = clients.clone();

    logger(&config).unwrap();

    info!(uuid=?Uuid::new_v4(), "TCP server running: {}", config.server_address);

    tokio::spawn(async move { active_clients_background(cloned_clients).await });

    loop {
        let (stream, addr) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let clients = Arc::clone(&clients);

        info!(uuid=?Uuid::new_v4(), ip=?addr, "client connected");

        tokio::spawn(async move { connection::handle_client(stream, addr, tx, clients).await });
    }
}

async fn active_clients_background(clients: Arc<client::Clients>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    let uuid = Uuid::new_v4();

    loop {
        interval.tick().await;

        info!(uuid=?uuid, "clients connected: {:?}", clients.connections);
    }
}
