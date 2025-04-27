use std::sync::Arc;

use tokio::{
    io::{self, AsyncWriteExt},
    net::tcp::OwnedWriteHalf,
    sync::broadcast::Sender,
};

use crate::lib::{client, utils};

#[allow(unused_variables)]
pub async fn help(writer: &mut OwnedWriteHalf) -> io::Result<()> {
    todo!()
}

pub async fn users(writer: &mut OwnedWriteHalf, clients: &Arc<client::Clients>) -> io::Result<()> {
    let nicks = clients.clients().await.join(", ");

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
    } else {
        writer.write_all("server: too many arguments provided\n".to_string().as_bytes()).await?;
    }

    Ok(())
}
