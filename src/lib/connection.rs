use std::{net::SocketAddr, sync::Arc};

use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, tcp::OwnedWriteHalf},
    sync::broadcast::Sender,
};

use crate::lib::{client, commands, utils};

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

    println!("notice: client connected from {:?}", addr);

    writer.write_all(b"server: please enter your nick below\n").await?;

    loop {
        let mut nick = String::new();
        let bytes = reader.read_line(&mut nick).await?;

        if bytes == 0 {
            utils::shutdown(&tx, &clients, client).await?;

            return Ok(());
        }

        if utils::set_nick(&mut writer, &tx, &clients, &mut client, &nick).await? {
            break;
        }
    }

    // drain buffered messages for new client, which will prevent returning old join messages
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
                            Err(e) => println!("Unable to send line: {}", e)
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
                    Err(e) => println!("Unable to read line: {}", e)
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
    let tokens: Vec<&str> = line.trim().split(' ').collect();

    if let Some(token) = tokens.first() {
        if token.starts_with('/') {
            if token.to_lowercase() == "/help" {
                commands::help(writer).await?;
                Ok(LineResult::NoBroadcast)
            } else if token.to_lowercase() == "/users" {
                commands::users(writer, clients).await?;
                Ok(LineResult::NoBroadcast)
            } else if token.to_lowercase() == "/nick" {
                commands::nick(writer, tx, clients, client, tokens).await?;
                Ok(LineResult::NoBroadcast)
            } else if token.to_lowercase() == "/quit" {
                Ok(LineResult::Shutdown)
            } else {
                writer.write_all(b"server: invalid command\n").await?;
                Ok(LineResult::NoBroadcast)
            }
        } else {
            Ok(LineResult::Broadcast(format!("{}: {}", client.nick, line)))
        }
    } else {
        Ok(LineResult::NoBroadcast)
    }
}
