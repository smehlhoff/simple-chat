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
    const fn new(addr: SocketAddr) -> Self {
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

    writer
        .write_all(b"server: please enter your nick below\n")
        .await?;

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
                    client.set_nick(nick);
                    break;
                } else {
                    writer
                        .write_all(b"server: this nick is already taken\n")
                        .await?;
                }
            }
        } else {
            writer
                .write_all(b"server: please enter a valid nick\n")
                .await?;
        }
    }

    clients.add(client.clone()).await;

    match tx.send(format!("server: {} has joined the server!\n", client.nick)) {
        Ok(_) => {}
        Err(e) => println!("Unable to send line: {}", e),
    }

    loop {
        let mut input = String::new();

        tokio::select! {
            bytes = reader.read_line(&mut input) => {
                if bytes.unwrap() == 0 {
                    clients.remove(addr).await;

                    println!("notice: client disconnected from {:?}", addr);
                    println!("{:?}", clients.connections);

                    match tx.send(format!("server: {} has left the server!\n", client.nick)) {
                        Ok(_) => {}
                        Err(e) => println!("Unable to send line: {}", e),
                    }

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
    let (tx, _) = broadcast::channel::<String>(32);
    let clients = Arc::new(Clients::new());

    loop {
        let (socket, addr) = listener.accept().await?;
        let tx = tx.clone();
        let clients = Arc::clone(&clients);

        tokio::spawn(async move { handle_client(socket, addr, tx, clients).await });
    }
}
