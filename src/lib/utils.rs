use std::sync::Arc;

use crate::lib::{client, config};

use tokio::{
    io::{self, AsyncWriteExt},
    net::tcp::OwnedWriteHalf,
    sync::broadcast::Sender,
    time::Duration,
};
use tracing::{error, info};
use tracing_subscriber::{
    EnvFilter, fmt::layer as fmt_layer, layer::SubscriberExt, prelude::*, util::SubscriberInitExt,
};
use url::Url;
use uuid::Uuid;

pub async fn clients_connected_background(clients: Arc<client::Clients>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    let uuid = Uuid::new_v4();

    interval.tick().await;

    loop {
        interval.tick().await;

        let clients = clients.connections.lock().await;

        info!(uuid=?uuid, total=?clients.len(), "clients connected: {:?}", clients);
    }
}

pub fn logger(config: &config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let (loki_layer, loki_task) = tracing_loki::builder()
        .label("service", "simple-chat")?
        .extra_field("environment", "development")?
        .build_url(Url::parse(&config.loki_address)?)?;
    let loki_layer =
        loki_layer.with_filter(EnvFilter::new("info").add_directive("tracing_loki=off".parse()?));
    let console_layer = fmt_layer()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_filter(EnvFilter::new("info").add_directive("tracing_loki=off".parse()?));

    tracing_subscriber::registry().with(loki_layer).with(console_layer).init();

    tokio::spawn(loki_task);

    Ok(())
}

pub async fn set_nick(
    writer: &mut OwnedWriteHalf,
    tx: &Sender<String>,
    clients: &Arc<client::Clients>,
    client: &mut client::Client,
    nick: &str,
) -> io::Result<bool> {
    let nick = nick.trim().to_string();

    if nick.is_ascii() && !nick.is_empty() && !nick.contains('/') {
        if nick.len() < 3 {
            writer.write_all(b"server: nick is too short\n").await?;
            Ok(false)
        } else if nick.len() > 15 {
            writer.write_all(b"server: nick is too long\n").await?;
            Ok(false)
        } else {
            if !clients.check_nick(&nick).await {
                client.set_nick(&nick);

                clients.add(client.clone()).await;

                info!(
                    uuid=?Uuid::new_v4(),
                    ip=?client.addr,
                    nick=?client.nick,
                    "client joined as:  {}", client.nick
                );

                match tx.send(format!("server: {} has joined\n", client.nick)) {
                    Ok(_) => {}
                    Err(e) => error!(uuid=?Uuid::new_v4(), "unable to send line: {}", e),
                }

                writer.write_all(b"server: welcome to chat\n").await?;
                Ok(true)
            } else {
                writer.write_all(b"server: nick is already taken\n").await?;
                Ok(false)
            }
        }
    } else {
        writer.write_all(b"server: please enter a valid nick\n").await?;
        Ok(false)
    }
}

pub async fn change_nick(
    writer: &mut OwnedWriteHalf,
    tx: &Sender<String>,
    clients: &Arc<client::Clients>,
    client: &mut client::Client,
    nick: &str,
) -> io::Result<()> {
    let nick = nick.trim().to_string();

    if nick.is_ascii() && !nick.is_empty() && !nick.contains('/') {
        if nick.len() < 3 {
            writer.write_all(b"server: nick is too short\n").await?;
        } else if nick.len() > 15 {
            writer.write_all(b"server: nick is too long\n").await?;
        } else {
            if !clients.check_nick(&nick).await {
                let old_nick = client.nick.clone();

                client.set_nick(&nick);

                clients.add(client.to_owned()).await;
                clients.remove_by_nick(&old_nick).await;

                match tx.send(format!("server: {} is now {}\n", old_nick, client.nick)) {
                    Ok(_) => {}
                    Err(e) => error!(uuid=?Uuid::new_v4(), "unable to send line: {}", e),
                }
            } else {
                writer.write_all(b"server: nick is already taken\n").await?;
            }
        }
    } else {
        writer.write_all(b"server: please enter a valid nick\n").await?;
    }

    Ok(())
}

pub async fn shutdown(
    tx: &Sender<String>,
    clients: &Arc<client::Clients>,
    client: client::Client,
) -> io::Result<()> {
    if clients.check_client(client.addr).await {
        match tx.send(format!("server: {} has left\n", client.nick)) {
            Ok(_) => {}
            Err(e) => error!(uuid=?Uuid::new_v4(), "unable to send line: {}", e),
        }
    }

    clients.remove_by_addr(client.addr).await;

    if !client.nick.is_empty() {
        info!(uuid=?Uuid::new_v4(), ip=?client.addr, nick=?client.nick, "client disconnected");
    } else {
        info!(uuid=?Uuid::new_v4(), ip=?client.addr, "client disconnected");
    }

    Ok(())
}
