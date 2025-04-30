use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct Client {
    pub nick: String,
    pub addr: SocketAddr,
    #[allow(dead_code)]
    pub connected_at: DateTime<Utc>,
    pub last_seen: Option<DateTime<Utc>>,
}

impl Client {
    pub fn new(addr: SocketAddr) -> Self {
        Self { nick: String::new(), addr, connected_at: chrono::Utc::now(), last_seen: None }
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
        self.connections.lock().await.push(client);
    }

    pub async fn remove_by_addr(&self, addr: SocketAddr) {
        self.connections.lock().await.retain(|c| c.addr != addr);
    }

    pub async fn remove_by_nick(&self, nick: &str) {
        self.connections.lock().await.retain(|c| c.nick != nick);
    }

    pub async fn retrieve_by_nick(&self, target: &str) -> Option<Client> {
        self.connections.lock().await.iter().find(|c| c.nick == target).cloned()
    }

    pub async fn list_clients(&self) -> Vec<String> {
        self.connections.lock().await.iter().map(|c| c.nick.to_string()).collect()
    }

    pub async fn check_by_addr(&self, addr: SocketAddr) -> bool {
        self.connections.lock().await.iter().any(|c| c.addr == addr)
    }

    pub async fn check_by_nick(&self, nick: &str) -> bool {
        self.connections.lock().await.iter().any(|c| c.nick == nick)
    }

    pub async fn set_last_seen(&self, addr: SocketAddr) {
        if let Some(client) = self.connections.lock().await.iter_mut().find(|c| c.addr == addr) {
            client.last_seen = Some(chrono::Utc::now());
        }
    }
}
