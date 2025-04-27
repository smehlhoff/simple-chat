use std::{net::SocketAddr, sync::Arc};

use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::broadcast::Sender,
};

use crate::lib::{client, commands};

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

        if nick.is_ascii() && !nick.is_empty() {
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

    // drain buffered messages for new client
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

                let line = handle_line(&client.nick, &input).await?;

                if input.len() > 1 {
                    match tx.send(line.to_string()) {
                        Ok(_) => {},
                        Err(e) => println!("Unable to send line: {}", e)
                        }
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

async fn handle_line(nick: &str, line: &str) -> io::Result<String> {
    let tokens: Vec<&str> = line.trim().split(" ").collect();

    match tokens[0] {
        "/help" => commands::help(),
        _ => {}
    }

    // println!("{:?}", tokens);

    let line = format!("{}: {}", nick, line);

    Ok(line)
}
