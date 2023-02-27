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
    // addr: SocketAddr,
    // pool: Arc<Mutex<VecDeque<TcpStream>>>,
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
        let mut partition_dbs: Vec<SocketAddr> = Vec::with_capacity(config.addresses.len());
        for db_address in config.addresses.iter() {
            partition_dbs.push(db_address.parse().unwrap());
        }
        let addresses = partition_dbs.clone();

        let mut pool: Vec<Arc<Mutex<VecDeque<TcpStream>>>> = Vec::new();
        for partition in partition_dbs {
            let mut connections = VecDeque::new();
            for _ in 0..config.min_connections {
                connections.push_back(TcpStream::connect(partition).await?);
            }
            pool.push(Arc::new(Mutex::new(connections)));
        }

        Ok(Self {
            pool,
            addresses,
        })
    }

    pub async fn get(&self, key: &str) -> Result<Response> {
        let request = Request {
            action_type: ActionType::Get,
            key_value: None,
            route: format!("/keys/{}", key),
        };

        let bytes = serialize(&request).unwrap();

        let db_index = &self.get_db_index(key);

        let mut conn = self.get_connection(db_index).await.unwrap();

        conn.write_all(&bytes).await.unwrap();

        let mut response_buf = vec![0; 256];
        conn.read(&mut response_buf).await.unwrap();
        let response: Response = deserialize(&response_buf).unwrap();

        self.return_connection(conn, db_index).await;

        Ok(response)
    }

    pub async fn update(&self, key_value: KeyValue) -> Result<Response> {
        let db_index = &self.get_db_index(&key_value.key);
        let request = Request {
            action_type: ActionType::Update,
            key_value: Some(key_value),
            route: "/keys".to_string(),
        };
        let serialized = serialize(&request).unwrap();

        self.send_get_response(&serialized, db_index).await
    }

    pub async fn delete(&self, key: &str) -> Result<Response> {
        let db_index = &self.get_db_index(key);
        let request = Request {
            action_type: ActionType::Delete,
            key_value: None,
            route: format!("/keys/{}", key),
        };

        let bytes = serialize(&request).unwrap();

        self.send_get_response(&bytes, db_index).await
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
        
        hash as usize % self.addresses.len()
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
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish() as u32
}
