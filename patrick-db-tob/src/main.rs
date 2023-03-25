use std::env;
use bincode::{deserialize};
use clap::{Parser};
use tokio::time;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:7979")]
    sequencer: String,
    #[arg(short, long, default_value = "127.0.0.1:8000,127.0.0.1:8001,127.0.0.1:8002")]
    replicas: String,
    #[arg(short, long, default_value = "test,test2,test3,test4,test5,test6,test7,test8,test9,test10")]
    messages: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Response {
    counter: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ActionType {
    Get = 1,
    Update = 2,
    Delete = 3,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    pub action_type: ActionType,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ResponseSequencer {
    counter: u64,
}

#[derive(Clone)]
pub struct Replica {
    pub sequencer: Arc<Mutex<TcpStream>>,
    pub peers: Arc<RwLock<Vec<Arc<Mutex<TcpStream>>>>>,
    pub peer_addresses: Vec<String>,
    pub addr: String,
    pub counter: u64,
}

impl Replica {
    pub async fn new(addr: String, sequencer_addr: String, peer_addresses: Vec<String>) -> Self {
        let sequencer = TcpStream::connect(sequencer_addr).await.unwrap();
        let peers = Arc::new(RwLock::new(Vec::new()));

        Replica {
            sequencer: Arc::new(Mutex::new(sequencer)),
            peers,
            peer_addresses,
            addr,
            counter: 0,
        }
    }

    pub async fn start_server(&mut self) -> Result<()> {
        log::info!("Starting server");
        let listener = TcpListener::bind(&self.addr).await.unwrap();
        let self_arc = Arc::new(Mutex::new(self.clone()));

        let self_arc2 = self_arc.clone();
        tokio::spawn(async move {
            let arc = self_arc2.clone();
            let locked_self = arc.lock().await;
            let mut replicas = Vec::new();
            for addr in &locked_self.peer_addresses {
                let stream = connect_with_retry(addr, 20, Duration::from_secs(2)).await;
                replicas.push(Arc::new(Mutex::new(stream)));
            }

            locked_self.peers.write().await.extend(replicas);
        });

        let self_arc3 = self_arc.clone();
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            log::info!("Accepted connection from: {}", stream.peer_addr().unwrap());

            let self_arc4 = self_arc3.clone();
            tokio::spawn(async move {
                let arc = self_arc4.clone();
                let mut locked_self = arc.lock().await;
                locked_self.handle_client_connection(stream).await.unwrap();
            });
        }
    }

    pub async fn handle_client_connection(&mut self, mut stream: tokio::net::TcpStream) -> Result<()> {
        log::info!("Handling client: {}", stream.peer_addr()?);

        // Read the message type from the stream
        let mut request_buf = [0; 256];
        let n = stream.read(&mut request_buf).await.unwrap();
        if n == 0 {
            log::info!("Connection closed by client: {}", stream.peer_addr()?);

            return Ok(());
        }

        let request: Request = deserialize(&request_buf).unwrap();
        log::debug!("Received request: {:?}", &request);

        self.handle_request(&mut stream, request).await?;

        Ok(())
    }

    async fn handle_request(&mut self, stream: &mut tokio::net::TcpStream, request: Request) -> Result<()> {
        match request.action_type {
            ActionType::Get => {
                self.handle_update_key(stream, request).await?;
            }
            ActionType::Update => {
                self.handle_update_key(stream, request).await?;
            }
            ActionType::Delete => {
                self.handle_update_key(stream, request).await?;
            }
        }

        Ok(())
    }

    async fn handle_update_key(&mut self, _stream: &mut tokio::net::TcpStream, _request: Request) -> Result<()> {
        let mut request_buf = [0; 256];
        let r = vec![1];
        let mut seq = self.sequencer.lock().await;
        seq.write_all(&r).await.unwrap();
        seq.read(&mut request_buf).await.unwrap();
        let _response: ResponseSequencer = deserialize(&request_buf).unwrap();

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let args = Args::parse();
    let sequencer = env::var("SEQUENCER").ok().unwrap_or(args.sequencer);
    let replica_args = env::var("REPLICAS").ok().unwrap_or(args.replicas);
    let replica_addrs: Vec<String> = replica_args.split(',').map(|s| s.to_string()).collect();
    let message = env::var("MESSAGES").ok().unwrap_or(args.messages);
    let _messages: Vec<String> = message.split(',').map(|s| s.to_string()).collect();

    let mut replicas = Vec::new();
    for addr in replica_addrs.clone() {
        replicas.push(Replica::new(addr.clone(), sequencer.clone(), replica_addrs.clone()).await);
    }

    let replicas: Vec<Arc<Mutex<Replica>>> = replicas
        .into_iter()
        .map(|replica| Arc::new(Mutex::new(replica)))
        .collect();

    for replica in replicas {
        let replica_clone = replica.clone();
        tokio::spawn(async move {
            let mut locked_replica = replica_clone.lock().await;
            locked_replica.start_server().await;
        });
    }
}

async fn connect_with_retry(addr: &str, max_retries: u32, retry_interval: Duration) -> TcpStream {
    let mut retries = 0;
    loop {
        match TcpStream::connect(addr).await {
            Ok(tcp_stream) => {
                return tcp_stream;
            }
            Err(e) => {
                if retries < max_retries {
                    log::error!("Error connecting to {}: {}. Retrying...", addr, e);
                    retries += 1;
                    time::sleep(retry_interval).await;
                } else {
                    panic!("Error connecting to {}: {}. Maximum retries exceeded", addr, e);
                }
            }
        }
    }
}