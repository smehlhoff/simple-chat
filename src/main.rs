#![warn(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, broadcast, broadcast::Sender};

#[derive(Clone, Debug)]
struct Client {
    nick: String,
    addr: SocketAddr,
}

impl Client {
    fn new(addr: SocketAddr) -> Self {
        Self {
            nick: String::new(),
            addr,
        }
    }

    fn set_nick(&mut self, nick: String) {
        self.nick = nick;
    }
}

#[derive(Debug)]
struct Clients {
    connections: Mutex<Vec<Client>>,
}

impl Clients {
    fn new() -> Self {
        Self {
            connections: Mutex::new(Vec::new()),
        }
    }

    async fn add(&self, client: Client) {
        let mut connections = self.connections.lock().await;

        connections.push(client);
    }

    async fn remove(&self, addr: SocketAddr) {
        let mut connections = self.connections.lock().await;

        connections.retain(|c| c.addr != addr);
    }

    async fn check_nick(&self, nick: &String) -> bool {
        let connections = self.connections.lock().await;

        connections.iter().any(|x| x.nick == *nick)
    }
}

async fn handle_line(nick: String, line: String) -> io::Result<String> {
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

    println!("Client connected from {:?}", addr);

    writer
        .write_all(b"Welcome! Please enter your nick below\n")
        .await?;

    loop {
        let mut nick = String::new();
        let bytes = reader.read_line(&mut nick).await?;

        nick = nick.trim().to_string();

        if nick.is_ascii() && !nick.is_empty() {
            if clients.check_nick(&nick).await == false {
                client.set_nick(nick);
                break;
            }
            writer.write_all(b"This nick is already taken\n").await?;
        } else {
            writer.write_all(b"Please enter a valid nick\n").await?;
        }

        if bytes == 0 {
            clients.remove(addr).await;

            println!("Client disconnected from {:?}", addr);
            println!("{:?}", clients.connections);

            break;
        }
    }

    clients.add(client.clone()).await;

    loop {
        let mut input = String::new();

        tokio::select! {
            bytes = reader.read_line(&mut input) => {
                let line = handle_line(client.nick.clone(), input).await?;

                match tx.send(line) {
                    Ok(_) => {},
                    Err(e) => println!("Unable to send line: {}", e)
                }

                if bytes.unwrap() == 0 {
                    clients.remove(addr).await;

                    println!("Client disconnected from {:?}", addr);
                    println!("{:?}", clients.connections);

                    break Ok(());
                }
            }

            msg = rx.recv() => {
                writer.write_all(msg.unwrap().as_bytes()).await?;
            }
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:6667").await?;
    let (tx, _) = broadcast::channel::<String>(32);
    let clients = Arc::new(Clients::new());

    loop {
        let (socket, addr) = listener.accept().await?;
        let tx = tx.clone();
        let clients = Arc::clone(&clients);

        tokio::spawn(async move { handle_client(socket, addr, tx, clients).await });
    }
}
