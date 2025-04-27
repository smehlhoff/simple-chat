use std::sync::Arc;

use tokio::{
    io::{self, AsyncWriteExt},
    net::tcp::OwnedWriteHalf,
};

use crate::lib::client;

pub async fn help(writer: &mut OwnedWriteHalf) -> io::Result<()> {
    todo!()
}

pub async fn users(writer: &mut OwnedWriteHalf, clients: &Arc<client::Clients>) -> io::Result<()> {
    let nicks = clients.clients().await.join(", ");

    writer.write_all(format!("server: {}\n", nicks).as_bytes()).await?;

    Ok(())
}

pub async fn nick(writer: &mut OwnedWriteHalf, client: &mut client::Client) -> io::Result<()> {
    todo!();

    Ok(())
}
