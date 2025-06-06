use std::sync::Arc;

use crate::lib::{client, utils};

use tokio::{
    io::{self, AsyncWriteExt},
    net::tcp::OwnedWriteHalf,
    sync::broadcast::Sender,
};

pub async fn time(writer: &mut OwnedWriteHalf) -> io::Result<()> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M");

    writer.write_all(format!("server: current time is {}\n", now).as_bytes()).await?;

    Ok(())
}

pub async fn users(writer: &mut OwnedWriteHalf, clients: &Arc<client::Clients>) -> io::Result<()> {
    let nicks = clients.list_clients().await.join(", ");

    writer.write_all(format!("server: {}\n", nicks).as_bytes()).await?;

    Ok(())
}

pub async fn nick(
    writer: &mut OwnedWriteHalf,
    tx: &Sender<String>,
    clients: &Arc<client::Clients>,
    client: &mut client::Client,
    tokens: Vec<&str>,
) -> io::Result<()> {
    if tokens.len() == 2 {
        utils::change_nick(writer, tx, clients, client, tokens[1]).await?;
    } else if tokens.len() == 1 {
        writer.write_all("server: not enough arguments provided\n".to_string().as_bytes()).await?;
    } else {
        writer.write_all("server: too many arguments provided\n".to_string().as_bytes()).await?;
    }

    Ok(())
}

pub async fn seen(
    writer: &mut OwnedWriteHalf,
    clients: &Arc<client::Clients>,
    tokens: Vec<&str>,
) -> io::Result<()> {
    if tokens.len() == 2 {
        if let Some(target) = clients.retrieve_by_nick(tokens[1]).await {
            match target.last_seen {
                Some(last_seen) => {
                    let last_seen = last_seen.format("%Y-%m-%d %H:%M").to_string();
                    writer
                        .write_all(
                            format!("server: {} was seen at {}\n", target.nick, last_seen)
                                .as_bytes(),
                        )
                        .await?;
                }
                None => {
                    writer
                        .write_all(format!("server: {} was not seen yet\n", target.nick).as_bytes())
                        .await?;
                }
            }
        } else {
            writer.write_all("server: nick not found\n".to_string().as_bytes()).await?;
        }
    } else if tokens.len() == 1 {
        writer.write_all("server: not enough arguments provided\n".to_string().as_bytes()).await?;
    } else {
        writer.write_all("server: too many arguments provided\n".to_string().as_bytes()).await?;
    }

    Ok(())
}
