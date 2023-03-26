use bincode::{deserialize, serialize};

use tokio::time;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc};

use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};

use tokio::sync::mpsc::Sender;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub message: String,
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
    pub sequence_number: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ResponseSequencer {
    counter: u64,
}

#[derive(Clone)]
pub struct Replica {
    sequencer: Arc<Mutex<TcpStream>>,
    peers: Arc<RwLock<Vec<Arc<Mutex<TcpStream>>>>>,
    peer_addresses: Vec<String>,
    messages: Arc<RwLock<Vec<String>>>,
    addr: String,
    counter: Arc<Mutex<u64>>,
}

impl Replica {
    pub async fn new(addr: String, sequencer_addr: String, peer_addresses: Vec<String>) -> Self {
        let sequencer = connect_with_retry(&sequencer_addr, 20, Duration::from_secs(2)).await;
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
        // broadcasting async to peers with a channel
        let (tx, mut rx) = mpsc::channel::<Request>(100);
        let listener = TcpListener::bind(&self.addr).await.unwrap();
        let self_arc = Arc::new(self.clone());

        // Spawn a task to connect to peers
        let self_arc_connect_to_peers = self_arc.clone();
        tokio::spawn(async move {
            let arc = self_arc_connect_to_peers.clone();
            let mut replicas = Vec::new();
            for addr in &arc.peer_addresses {
                let stream = connect_with_retry(addr, 20, Duration::from_secs(2)).await;
                replicas.push(Arc::new(Mutex::new(stream)));
            }

            let mut p = arc.peers.write().await;
            *p = replicas;
            log::info!("Connected to peers: {:?} for address: {}", &arc.peer_addresses, &self_arc_connect_to_peers.addr);
        });

        // Spawn a task to receive messages from the channel
        let self_arc_tob = self_arc.clone();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                log::debug!("broadcasting message: {:?}, addr: {}", &message, &self_arc_tob.addr);
                let tcp_connections = self_arc_tob.peers.read().await;
                for stream in tcp_connections.iter() {
                    let mut stream = stream.lock().await;
                    let bytes = serialize(&message).unwrap();
                    stream.write_all(&bytes).await.unwrap();
                    log::debug!("Sent {:?} to follower {}", &message, stream.peer_addr().unwrap());
                }
            }
        });

        // Spawn a task to handle incoming connections
        let self_arc_server = self_arc.clone();
        loop {
            let (stream, _) = listener.accept().await.unwrap();

            let self_arc_thread = self_arc_server.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let arc = self_arc_thread.clone();
                arc.handle_client_connection(stream, tx).await.unwrap();
            });
        }
    }

    async fn handle_client_connection(&self, mut stream: TcpStream, tx: Sender<Request>) -> Result<()> {
        loop {
            let mut request_buf = [0; 256];
            let n = stream.read(&mut request_buf).await.unwrap();
            if n == 0 {
                log::info!("Connection closed by client: {}", stream.peer_addr()?);

                return Ok(());
            }

            let request: Request = deserialize(&request_buf).unwrap();
            log::debug!("Received request: {:?}", &request);

            self.handle_request(&mut stream, request, tx.clone()).await?;
        }
    }

    async fn handle_request(&self, stream: &mut TcpStream, request: Request, tx: Sender<Request>) -> Result<()> {
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
        let message = self.messages.read().await.join(",");

        let response = Response { message };
        let response_buf = serialize(&response).unwrap();
        stream.write_all(&response_buf).await.unwrap();

        Ok(())
    }

    async fn handle_tob(&self, stream: &mut TcpStream, request: Request) -> Result<()> {
        log::debug!("Received request from tob: {:?}, addr: {}", &request, &self.addr);
        // loop ensures that the messages are received in order
        loop {
            let mut counter = self.counter.lock().await;
            // we start at counter = 0, then receive message 0. Increment counter to 1 and wait
            // for message 1. And so on.
            if request.sequence_number == *counter {
                // append message from request to messages in replica
                let mut messages = self.messages.write().await;
                messages.push(request.clone().message);
                // increase received messages counter
                *counter += 1;
                break;
            }
            log::debug!("Waiting for counter: {}, current counter: {}, addr: {}", request.sequence_number, *counter, &self.addr);
            // because we are in a loop and another thread want to use the counter as well,
            // we need to release the lock by dropping it. Locks are released when they go out of
            // scope aka dropped.
            drop(counter);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let response = Response { message: "success".to_string() };
        let response_buf = serialize(&response).unwrap();
        stream.write_all(&response_buf).await.unwrap();

        Ok(())
    }

    async fn handle_update_key(&self, stream: &mut TcpStream, mut request: Request, tx: Sender<Request>) -> Result<()> {
        // get sequence number from sequencer
        let mut request_buf = [0; 256];
        let r = vec![1];
        let mut seq = self.sequencer.lock().await;
        seq.write_all(&r).await.unwrap();
        seq.read(&mut request_buf).await.unwrap();
        let response: ResponseSequencer = deserialize(&request_buf).unwrap();
        request.sequence_number = response.counter;
        request.action_type = ActionType::Tob;

        // tob the message to all replicas and return success to client
        let response = Response { message: "success".to_string() };
        let response_buf = serialize(&response).unwrap();
        stream.write_all(&response_buf).await.unwrap();
        tx.send(request).await.unwrap();

        Ok(())
    }
}

pub async fn connect_with_retry(addr: &str, max_retries: u32, retry_interval: Duration) -> TcpStream {
    let mut retries = 0;
    loop {
        match TcpStream::connect(addr).await {
            Ok(tcp_stream) => {
                log::debug!("Connected to {}", addr);
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