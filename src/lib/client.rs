use std::net::SocketAddr;

use tokio::sync::Mutex;

use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct Client {
    pub nick: String,
    pub addr: SocketAddr,
    pub connected_at: DateTime<Utc>,
}

impl Client {
    pub fn new(addr: SocketAddr) -> Self {
        let connected_at = chrono::Utc::now();

        Self { nick: String::new(), addr, connected_at }
    }

    pub fn set_nick(&mut self, nick: &str) {
        self.nick = nick.to_string();
    }
}

#[derive(Debug)]
pub struct Clients {
    pub connections: Mutex<Vec<Client>>,
}

impl Clients {
    pub fn new() -> Self {
        Self { connections: Mutex::new(Vec::new()) }
    }

    pub async fn add(&self, client: Client) {
        let mut connections = self.connections.lock().await;

        connections.push(client);
    }

    pub async fn remove(&self, addr: SocketAddr) {
        let mut connections = self.connections.lock().await;

        connections.retain(|c| c.addr != addr);
    }

    pub async fn clients(&self) -> Vec<String> {
        let connections = self.connections.lock().await;

        connections.iter().map(|c| c.nick.clone()).collect()
    }

    pub async fn check_client(&self, addr: SocketAddr) -> bool {
        let connections = self.connections.lock().await;

        connections.iter().any(|c| c.addr == addr)
    }

    pub async fn check_nick(&self, nick: &str) -> bool {
        let connections = self.connections.lock().await;

        connections.iter().any(|c| c.nick == *nick)
    }
}
