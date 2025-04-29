use std::{net::SocketAddr, sync::Arc};

use crate::lib::{client, commands, utils};

use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, tcp::OwnedWriteHalf},
    sync::broadcast::Sender,
};
use tracing::{error, info};
use uuid::Uuid;

enum LineResult {
    Broadcast(String),
    Shutdown,
    NoBroadcast,
}

pub async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    tx: Sender<String>,
    clients: Arc<client::Clients>,
) -> io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut rx = tx.subscribe();
    let mut reader = BufReader::new(reader);
    let mut client = client::Client::new(addr);

    writer.write_all(b"server: please enter your nick below\n").await?;

    loop {
        let mut nick = String::new();
        let bytes = reader.read_line(&mut nick).await?;

        if bytes == 0 {
            utils::shutdown(&tx, &clients, client).await?;

            return Ok(());
        }

        info!(
            uuid=?Uuid::new_v4(),
            ip=?client.addr,
            nick=?nick.trim(),
            "client entered nick as: {}", nick
        );

        if utils::set_nick(&mut writer, &tx, &clients, &mut client, &nick).await? {
            break;
        }
    }

    // drain buffered messages for new client, which will prevent returning old messages
    while rx.try_recv().is_ok() {}

    loop {
        let mut input = String::new();

        tokio::select! {
            bytes = reader.read_line(&mut input) => {
                if bytes.unwrap() == 0 {
                    utils::shutdown(&tx, &clients, client).await?;

                    return Ok(());
                }

                let line = handle_line(&mut writer, &tx, &clients, &mut client, &input).await?;

                match line {
                    LineResult::Broadcast(line) => {
                        match tx.send(line) {
                            Ok(_) => {},
                            Err(e) => error!(uuid=?Uuid::new_v4(), "unable to send line: {}", e)
                        }
                    }
                    LineResult::Shutdown => {
                        utils::shutdown(&tx, &clients, client).await?;

                        return Ok(());
                    }
                    LineResult::NoBroadcast => {}
                }
            }

            msg = rx.recv() => {
                match msg {
                    Ok(msg) => writer.write_all(msg.as_bytes()).await?,
                    Err(e) => error!(uuid=?Uuid::new_v4(), "unable to send line: {}", e)
                }
            }
        }
    }
}

async fn handle_line(
    writer: &mut OwnedWriteHalf,
    tx: &Sender<String>,
    clients: &Arc<client::Clients>,
    client: &mut client::Client,
    line: &str,
) -> io::Result<LineResult> {
    info!(uuid=?Uuid::new_v4(), ip=?client.addr, nick=?client.nick, "client sent: {}", line);

    let tokens: Vec<&str> = line.trim().split(' ').collect();

    if let Some(token) = tokens.first() {
        let token = token.to_lowercase();

        if token.is_empty() {
            Ok(LineResult::NoBroadcast)
        } else if token.starts_with('/') {
            if token == "/time" {
                commands::time(writer).await?;
                Ok(LineResult::NoBroadcast)
            } else if token == "/users" {
                commands::users(writer, clients).await?;
                Ok(LineResult::NoBroadcast)
            } else if token == "/nick" {
                commands::nick(writer, tx, clients, client, tokens).await?;
                Ok(LineResult::NoBroadcast)
            } else if token == "/seen" {
                commands::seen(writer, clients, tokens).await?;
                Ok(LineResult::NoBroadcast)
            } else if token == "/quit" {
                Ok(LineResult::Shutdown)
            } else {
                writer.write_all(b"server: invalid command\n").await?;
                Ok(LineResult::NoBroadcast)
            }
        } else {
            clients.set_last_seen(client.addr).await;

            Ok(LineResult::Broadcast(format!("{}: {}", client.nick, line)))
        }
    } else {
        Ok(LineResult::NoBroadcast)
    }
}
