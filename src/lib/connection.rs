use std::{net::SocketAddr, sync::Arc};

use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, tcp::OwnedWriteHalf},
    sync::broadcast::Sender,
};

use crate::lib::{client, commands};

enum LineResult {
    Broadcast(String),
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
            clients.remove(addr).await;

            println!("notice: client disconnected from {:?}", addr);
            println!("{:?}", clients.connections);

            break;
        }

        nick = nick.trim().to_string();

        if nick.is_ascii() && !nick.is_empty() && !nick.contains('/') {
            if nick.len() < 3 {
                writer.write_all(b"server: nick is too short\n").await?;
            } else if nick.len() > 15 {
                writer.write_all(b"server: nick is too long\n").await?;
            } else {
                if clients.check_nick(&nick).await == false {
                    client.set_nick(&nick);

                    clients.add(client.clone()).await;

                    match tx.send(format!("server: {} has joined\n", client.nick)) {
                        Ok(_) => {}
                        Err(e) => println!("Unable to send line: {}", e),
                    }

                    writer.write_all(b"server: welcome to chat\n").await?;

                    break;
                } else {
                    writer.write_all(b"server: nick is already taken\n").await?;
                }
            }
        } else {
            writer.write_all(b"server: please enter a valid nick\n").await?;
        }
    }

    // drain buffered messages for new client, which will prevent returning old join messages
    while let Ok(_) = rx.try_recv() {}

    loop {
        let mut input = String::new();

        tokio::select! {
            bytes = reader.read_line(&mut input) => {
                if bytes.unwrap() == 0 {
                    if clients.check_client(client.addr).await == true {
                        match tx.send(format!("server: {} has left\n", client.nick)) {
                            Ok(_) => {}
                            Err(e) => println!("Unable to send line: {}", e),
                        }

                    }

                    clients.remove(addr).await;

                    println!("notice: client disconnected from {:?}", addr);
                    println!("{:?}", clients.connections);

                    break Ok(());
                }

                let line = handle_line(&mut writer, &clients, &mut client, &input).await?;

                match line {
                    LineResult::Broadcast(line) => {
                        match tx.send(line) {
                            Ok(_) => {},
                            Err(e) => println!("Unable to send line: {}", e)
                        }
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
    clients: &Arc<client::Clients>,
    client: &mut client::Client,
    line: &str,
) -> io::Result<LineResult> {
    let tokens: Vec<&str> = line.trim().split(' ').collect();

    if let Some(token) = tokens.first() {
        if token.starts_with('/') {
            if token.to_lowercase().contains("/help") {
                commands::help(writer).await?;
            } else if token.to_lowercase().contains("/users") {
                commands::users(writer, clients).await?;
            } else if token.to_lowercase().contains("/nick") {
                commands::nick(writer, client).await?;
            } else {
                writer.write_all(b"server: not a valid command\n").await?;
            }
            Ok(LineResult::NoBroadcast)
        } else {
            Ok(LineResult::Broadcast(format!("{}: {}", client.nick, line)))
        }
    } else {
        Ok(LineResult::NoBroadcast)
    }
}
