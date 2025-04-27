use std::sync::Arc;

use tokio::{
    io::{self, AsyncWriteExt},
    net::tcp::OwnedWriteHalf,
    sync::broadcast::Sender,
};

use crate::lib::client;

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

                match tx.send(format!("server: {} has joined\n", client.nick)) {
                    Ok(_) => {}
                    Err(e) => println!("Unable to send line: {}", e),
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
                    Err(e) => println!("Unable to send line: {}", e),
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
            Err(e) => println!("Unable to send line: {}", e),
        }
    }

    clients.remove_by_addr(client.addr).await;

    println!("notice: client disconnected from {:?}", client.addr);
    println!("{:?}", clients.connections);

    Ok(())
}
