use std::collections::{VecDeque};
use std::hash::{Hash, Hasher};
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
    pool: Vec<Arc<Mutex<VecDeque<TcpStream>>>>,
    addresses: Vec<SocketAddr>,
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
    pub addresses: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_connections: 10,
            addresses: vec!["127.0.0.1:8080".to_string()],
        }
    }
}

impl PDBConnectionPool {
    pub async fn new(config: Config) -> Result<Self> {
        todo!("connect to all addresses passed by the config. We assume that each address is a partition db server");
        let mut partition_dbs: Vec<SocketAddr> = Vec::with_capacity(config.addresses.len());
        let addresses = partition_dbs.clone();
    }

    pub async fn get(&self, key: &str) -> Result<Response> {
        let request = Request {
            action_type: ActionType::Get,
            key_value: None,
            route: format!("/keys/{}", key),
        };

        let bytes = serialize(&request).unwrap();

        todo!("hash the key to get the db index. Then get a connection based on the hash.");
        let db_index = 0 as usize;
        let mut conn = self.get_connection(&db_index).await.unwrap();

        conn.write_all(&bytes).await.unwrap();

        let mut response_buf = vec![0; 256];
        conn.read(&mut response_buf).await.unwrap();
        let response: Response = deserialize(&response_buf).unwrap();

        self.return_connection(conn, &db_index).await;

        Ok(response)
    }

    pub async fn update(&self, key_value: KeyValue) -> Result<Response> {
        todo!("hash the key to get the db index. Then get a connection based on the hash.");
        let db_index = 0 as usize;
        let request = Request {
            action_type: ActionType::Update,
            key_value: Some(key_value),
            route: "/keys".to_string(),
        };
        let serialized = serialize(&request).unwrap();

        self.send_get_response(&serialized, &db_index).await
    }

    pub async fn delete(&self, key: &str) -> Result<Response> {
        todo!("hash the key to get the db index. Then get a connection based on the hash.");
        let db_index = 0 as usize;
        let request = Request {
            action_type: ActionType::Delete,
            key_value: None,
            route: format!("/keys/{}", key),
        };

        let bytes = serialize(&request).unwrap();

        self.send_get_response(&bytes, &db_index).await
    }

    async fn get_connection(&self, index: &usize) -> Result<TcpStream> {
        let mut pool = self.pool[*index].lock().await;

        if let Some(conn) = pool.pop_front() {
            return Ok(conn);
        }

        let conn = TcpStream::connect(self.addresses[*index]).await?;
        Ok(conn)
    }
    async fn return_connection(&self, conn: TcpStream, index: &usize) {
        let mut pool = self.pool[*index].lock().await;
        pool.push_back(conn);
    }

    fn get_db_index(&self, key: &str) -> usize {
        let hash = calculate_hash(key);
        todo!("return hash mod n")
    }

    async fn send_get_response(&self, bytes: &[u8], index: &usize) -> Result<Response> {
        let mut conn = self.get_connection(index).await?;

        conn.write_all(bytes).await?;

        let mut response_buf = vec![0; 256];
        conn.read(&mut response_buf).await?;
        let response: Response = deserialize(&response_buf).unwrap();

        self.return_connection(conn, index).await;

        Ok(response)
    }
}

// Calculate the hash of a key or database address
fn calculate_hash(s: &str) -> u32 {
    todo!("hash the str s")
}
