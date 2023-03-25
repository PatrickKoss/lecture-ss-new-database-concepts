use std::env;

use bincode::{deserialize, serialize};
use clap::{Parser};
use tokio::time;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc};

use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use rand::seq::SliceRandom;
use tokio::sync::mpsc::Sender;

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
    Tob = 3,
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
    pub messages: Arc<RwLock<Vec<String>>>,
    pub addr: String,
    pub counter: Arc<Mutex<u64>>,
}

impl Replica {
    pub async fn new(addr: String, sequencer_addr: String, peer_addresses: Vec<String>) -> Self {
        let sequencer = TcpStream::connect(sequencer_addr).await.unwrap();
        let peers = Arc::new(RwLock::new(Vec::new()));
        let messages = Arc::new(RwLock::new(Vec::new()));
        let counter = Arc::new(Mutex::new(0));

        Replica {
            sequencer: Arc::new(Mutex::new(sequencer)),
            peers,
            peer_addresses,
            messages,
            addr,
            counter,
        }
    }

    pub async fn start_server(&self) -> Result<()> {
        log::info!("Starting server on: {}", self.addr);
        let (tx, mut rx) = mpsc::channel::<Request>(100);
        let listener = TcpListener::bind(&self.addr).await.unwrap();
        let self_arc_lock_free = Arc::new(self.clone());

        let self_arc_lock_free2 = self_arc_lock_free.clone();
        tokio::spawn(async move {
            let arc = self_arc_lock_free2.clone();
            let mut replicas = Vec::new();
            for addr in &arc.peer_addresses {
                let stream = connect_with_retry(addr, 20, Duration::from_secs(2)).await;
                replicas.push(Arc::new(Mutex::new(stream)));
            }

            let mut p = arc.peers.write().await;
            *p = replicas;
            log::info!("Connected to peers: {:?} for address: {}", &arc.peer_addresses, &self_arc_lock_free2.addr);
        });

        // Spawn a task to receive messages from the channel
        let self_arc_lock_free_tob = self_arc_lock_free.clone();
        tokio::spawn(async move {
            while let Some(mut message) = rx.recv().await {
                message.action_type = ActionType::Tob;
                let tcp_connections = self_arc_lock_free_tob.peers.read().await;
                for stream in tcp_connections.iter() {
                    let mut stream = stream.lock().await;
                    let bytes = serialize(&message).unwrap();
                    stream.write_all(&bytes).await.unwrap();
                    log::info!("Sent {:?} to follower {}", &message, stream.peer_addr().unwrap());
                }
            }
        });

        let self_arc_lock_free3 = self_arc_lock_free.clone();
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            log::info!("Accepted connection from: {}", stream.peer_addr().unwrap());

            let self_arc_lock_free4 = self_arc_lock_free3.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let arc = self_arc_lock_free4.clone();
                arc.handle_client_connection(stream, tx).await.unwrap();
            });
        }
    }

    pub async fn handle_client_connection(&self, mut stream: TcpStream, tx: Sender<Request>) -> Result<()> {
        loop {
            let mut request_buf = [0; 256];
            let n = stream.read(&mut request_buf).await.unwrap();
            if n == 0 {
                log::info!("Connection closed by client: {}", stream.peer_addr()?);

                return Ok(());
            }

            let request: Request = deserialize(&request_buf).unwrap();
            log::info!("Received request: {:?}", &request);

            self.handle_request(&mut stream, request, tx.clone()).await?;
        }
    }

    async fn handle_request(&self, stream: &mut tokio::net::TcpStream, request: Request, tx: Sender<Request>) -> Result<()> {
        match request.action_type {
            ActionType::Get => {
                self.handle_get(stream).await?;
            }
            ActionType::Update => {
                self.handle_update_key(stream, request, tx).await?;
            }
            ActionType::Tob => {
                self.handle_tob(stream, request).await?;
            }
        }

        Ok(())
    }

    async fn handle_get(&self, stream: &mut TcpStream) -> Result<()> {
        let counter = self.counter.lock().await;

        let response = Response { counter:  *counter };
        let response_buf = serialize(&response).unwrap();
        stream.write_all(&response_buf).await.unwrap();

        Ok(())
    }

    async fn handle_tob(&self, stream: &mut TcpStream, request: Request) -> Result<()> {
        let mut messages = self.messages.write().await;
        messages.push(request.clone().message);

        let response = Response { counter:  0 };
        let response_buf = serialize(&response).unwrap();
        stream.write_all(&response_buf).await.unwrap();

        Ok(())
    }

    async fn handle_update_key(&self, stream: &mut TcpStream, request: Request, tx: Sender<Request>) -> Result<()> {
        let mut request_buf = [0; 256];
        let r = vec![1];
        let mut seq = self.sequencer.lock().await;
        seq.write_all(&r).await.unwrap();
        seq.read(&mut request_buf).await.unwrap();
        let response: ResponseSequencer = deserialize(&request_buf).unwrap();

        let mut counter = self.counter.lock().await;
        *counter = response.counter;

        let response = Response { counter:  response.counter };
        let response_buf = serialize(&response).unwrap();
        stream.write_all(&response_buf).await.unwrap();
        tx.send(request).await.unwrap();

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
    let messages: Vec<String> = message.split(',').map(|s| s.to_string()).collect();

    let mut replicas = Vec::new();
    for addr in replica_addrs.clone() {
        replicas.push(Replica::new(addr.clone(), sequencer.clone(), replica_addrs.clone()).await);
    }

    let replicas: Vec<Arc<Replica>> = replicas
        .into_iter()
        .map(Arc::new)
        .collect();

    for replica in &replicas {
        let replica_clone = replica.clone();
        tokio::spawn(async move {
            replica_clone.start_server().await.unwrap();
        });
    }

    log::info!("sleep 2 seconds to wait for replicas to connect to each other");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // connect to all replicas
    let mut replica_streams = Vec::new();
    for addr in replica_addrs {
        let stream = connect_with_retry(&addr, 20, Duration::from_secs(2)).await;
        replica_streams.push(Arc::new(Mutex::new(stream)));
    }

    // send messages to random replicas
    for message in messages {
        let mut rng = rand::thread_rng();
        let replica_stream = replica_streams.choose(&mut rng).unwrap();
        let mut replica_stream = replica_stream.lock().await;
        let msg = Request { action_type: ActionType::Update, message };
        let msg_buf = serialize(&msg).unwrap();
        replica_stream.write_all(&msg_buf).await.unwrap();
        let mut response_buf = [0; 256];
        replica_stream.read(&mut response_buf).await.unwrap();
        let response: Response = deserialize(&response_buf).unwrap();
        log::info!("Response: {:?}", response);
    }

    // iterate over all replicas and do get request
    for replica_stream in &replica_streams {
        let mut replica_stream = replica_stream.lock().await;
        let msg = Request { action_type: ActionType::Get, message: "".to_string() };
        let msg_buf = serialize(&msg).unwrap();
        replica_stream.write_all(&msg_buf).await.unwrap();
        let mut response_buf = [0; 256];
        replica_stream.read(&mut response_buf).await.unwrap();
        let response: Response = deserialize(&response_buf).unwrap();
        log::info!("counter: {:?}, addr {}", response, replica_stream.peer_addr().unwrap());
    }
}

async fn connect_with_retry(addr: &str, max_retries: u32, retry_interval: Duration) -> TcpStream {
    let mut retries = 0;
    loop {
        match TcpStream::connect(addr).await {
            Ok(tcp_stream) => {
                log::info!("Connected to {}", addr);
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