#![warn(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]

use std::{net::SocketAddr, sync::Arc};

use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Mutex, broadcast, broadcast::Sender},
};

use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
struct Client {
    nick: String,
    addr: SocketAddr,
    connected_at: DateTime<Utc>,
}

impl Client {
    fn new(addr: SocketAddr) -> Self {
        let connected_at = chrono::Utc::now();

        Self { nick: String::new(), addr, connected_at }
    }

    fn set_nick(&mut self, nick: &str) {
        self.nick = nick.to_string();
    }
}

#[derive(Debug)]
struct Clients {
    connections: Mutex<Vec<Client>>,
}

impl Clients {
    fn new() -> Self {
        Self { connections: Mutex::new(Vec::new()) }
    }

    async fn add(&self, client: Client) {
        let mut connections = self.connections.lock().await;

        connections.push(client);
    }

    async fn remove(&self, addr: SocketAddr) {
        let mut connections = self.connections.lock().await;

        connections.retain(|c| c.addr != addr);
    }

    async fn check_client(&self, addr: SocketAddr) -> bool {
        let connections = self.connections.lock().await;

        connections.iter().any(|c| c.addr == addr)
    }

    async fn check_nick(&self, nick: &str) -> bool {
        let connections = self.connections.lock().await;

        connections.iter().any(|c| c.nick == *nick)
    }
}

async fn handle_line(nick: &str, line: &str) -> io::Result<String> {
    // let tokens: Vec<&str> = line.trim().split(" ").collect();

    // println!("{:?}", tokens);

    let line = format!("{}: {}", nick, line);

    Ok(line)
}

async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    tx: Sender<String>,
    clients: Arc<Clients>,
) -> io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut rx = tx.subscribe();
    let mut reader = BufReader::new(reader);
    let mut client = Client::new(addr);

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

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:6667").await?;
    let (tx, _) = broadcast::channel::<String>(16);
    let clients = Arc::new(Clients::new());

    loop {
        let (stream, addr) = listener.accept().await?;
        let tx = tx.clone();
        let clients = Arc::clone(&clients);

        tokio::spawn(async move { handle_client(stream, addr, tx, clients).await });
    }
}
