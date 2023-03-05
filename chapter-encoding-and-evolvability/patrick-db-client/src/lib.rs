use std::collections::{VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct PDBConnectionPool {
    addr: SocketAddr,
    pool: Arc<Mutex<VecDeque<TcpStream>>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyValue {
    pub key: String,
    pub value: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
enum ActionType {
    Get = 1,
    Update = 2,
    Delete = 3,
}

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    action_type: ActionType,
    key_value: Option<KeyValue>,
    route: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub status: u16,
    pub key_value: Option<KeyValue>,
}

pub struct Config {
    pub min_connections: usize,
    pub addr: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_connections: 10,
            addr: "127.0.0.1:8080".to_string(),
        }
    }
}

impl PDBConnectionPool {
    pub async fn new(config: Config) -> Result<Self> {
        let mut connections = VecDeque::new();

        todo!("push config.min_connections tcp streams to connections");

        Ok(Self {
            pool: Arc::new(Mutex::new(connections)),
            addr: config.addr.parse().unwrap(),
        })
    }

    pub async fn get(&self, key: &str) -> Result<Response> {
        let request = Request {
            action_type: ActionType::Get,
            key_value: None,
            route: format!("/keys/{}", key),
        };

        todo!("get connection with self.get_connection(). \
        Serialize request and send it to the server.\
        Deserialize the response by creating a buffer and reading into it.\
        Return the connection to the pool with self.return_connection(conn).");
    }

    pub async fn update(&self, key_value: KeyValue) -> Result<Response> {
        let request = Request {
            action_type: ActionType::Update,
            key_value: Some(key_value),
            route: "/keys".to_string(),
        };

        todo!("get connection with self.get_connection(). \
        Serialize request and send it to the server.\
        Deserialize the response by creating a buffer and reading into it.\
        Return the connection to the pool with self.return_connection(conn).");
    }

    pub async fn delete(&self, key: &str) -> Result<Response> {
        let request = Request {
            action_type: ActionType::Delete,
            key_value: None,
            route: format!("/keys/{}", key),
        };

        todo!("get connection with self.get_connection(). \
        Serialize request and send it to the server.\
        Deserialize the response by creating a buffer and reading into it.\
        Return the connection to the pool with self.return_connection(conn).");
    }

    async fn get_connection(&self) -> Result<TcpStream> {
        if let Some(conn) = self.pool.lock().await.pop_front() {
            return Ok(conn);
        }

        let conn = TcpStream::connect(self.addr).await?;
        Ok(conn)
    }
    async fn return_connection(&self, conn: TcpStream) {
        let mut pool = self.pool.lock().await;
        pool.push_back(conn);
    }
}
