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
            todo!("connect to all peers and store them in the peers vector. (7 lines of code)");
            log::info!("Connected to peers: {:?} for address: {}", &arc.peer_addresses, &self_arc_connect_to_peers.addr);
        });

        // Spawn a task to receive messages from the channel
        let self_arc_tob = self_arc.clone();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                log::debug!("broadcasting message: {:?}, addr: {}", &message, &self_arc_tob.addr);
                todo!("get all peer connections and iterate over them, then send the request to all peers. (7 lines)");
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

        todo!("total order broadcasting bring the messages in order. Each request has a sequence \
        number. If the sequence number is not the same as the counter in the replica, wait for \
        the correct sequence number. If it is the same then append the message in request and \
        increment the counter in replica by 1. Finally return a success response to the client. (12 lines)");

        Ok(())
    }

    async fn handle_update_key(&self, stream: &mut TcpStream, mut request: Request, tx: Sender<Request>) -> Result<()> {
        todo!("get a sequence from the sequencer, adjust the request sequencer number with the \
        number from the sequencer and the action type to tob. In the last step use the channel \
        to send the request to the tob task. (12 lines)");

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