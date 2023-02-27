use std::collections::HashSet;
use std::net::SocketAddr;

use anyhow::Result;
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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
    pub route: String,
}

// Define a struct to represent a replica
#[derive(Debug, Clone)]
pub struct Replica {
    pub addr: SocketAddr,
    pub log: Vec<String>,
    pub peers: HashSet<SocketAddr>,
}

pub(crate) async fn run() {
    // Create the initial set of replicas
    let mut replicas = vec![
        Replica {
            addr: "127.0.0.1:8000".parse().unwrap(),
            log: Vec::new(),
            peers: HashSet::new(),
        },
        Replica {
            addr: "127.0.0.1:8001".parse().unwrap(),
            log: Vec::new(),
            peers: HashSet::new(),
        },
        Replica {
            addr: "127.0.0.1:8002".parse().unwrap(),
            log: Vec::new(),
            peers: HashSet::new(),
        },
    ];

    // Add each replica's peer list
    for i in 0..replicas.len() {
        for j in 0..replicas.len() {
            if i != j {
                let addr = replicas[j].addr.clone();
                replicas[i].peers.insert(addr);
            }
        }
    }

    // Start a listener for each replica
    for replica in replicas.into_iter() {
        tokio::spawn(start_server(replica));
    }

    let request = Request {
        action_type: ActionType::Update,
        message: "hello world".to_string(),
        route: "/users".to_string(),
    };
    let mut stream = TcpStream::connect("127.0.0.1:8000").await.unwrap();
    let bytes = serialize(&request).unwrap();
    stream.write_all(&bytes).await.unwrap();
    let mut stream = TcpStream::connect("127.0.0.1:8000").await.unwrap();
    stream.write_all(&bytes).await.unwrap();
}

pub async fn start_server(replica: Replica) -> Result<()> {
    log::info!("Starting server");
    let listener = TcpListener::bind(&replica.addr).await.unwrap();

    loop {
        let replica = replica.clone();
        let (stream, _) = listener.accept().await.unwrap();
        log::info!("Accepted connection from: {}", stream.peer_addr().unwrap());

        tokio::spawn(handle_client_connection(stream, replica));
    }
}

pub async fn handle_client_connection(mut stream: tokio::net::TcpStream, replica: Replica) -> Result<()> {
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

    handle_request(&mut stream, request, replica).await?;

    Ok(())
}

async fn handle_request(stream: &mut tokio::net::TcpStream, request: Request, replica: Replica) -> Result<()> {
    match request.action_type {
        ActionType::Get => {
            handle_update_key(stream, request, replica).await?;
        }
        ActionType::Update => {
            handle_update_key(stream, request, replica).await?;
        }
        ActionType::Delete => {
            handle_update_key(stream, request, replica).await?;
        }
    }

    Ok(())
}

async fn handle_update_key(stream: &mut tokio::net::TcpStream, request: Request, mut replica: Replica) -> Result<()> {
    if request.route == "/users".to_string() {
        // tob to all other replicas
        let mut request = request.clone();
        request.route = "/tob".to_string();
        replica.log.push(request.message.clone());
        println!("replica log: {:?}", &replica.log);
        for peer in replica.peers {
            let mut peer_stream = TcpStream::connect(peer).await.unwrap();
            let bytes = serialize(&request).unwrap();
            peer_stream.write_all(&bytes).await.unwrap();

            let mut request_buf = [0; 256];
            peer_stream.read(&mut request_buf).await.unwrap();
            let peer_request: Request = deserialize(&request_buf).unwrap();
            println!("response tob: {:?}", peer_request);
            // Send the response back to the user
            let response = "success";
            stream.write_all(response.as_bytes()).await?;
        }
    } else {
        println!("in tob");
        // Send the response back to replica
        let request2 = Request {
            action_type: ActionType::Update,
            message: "hello world tob".to_string(),
            route: "/users".to_string(),
        };
        let bytes = serialize(&request2).unwrap();
        stream.write_all(&bytes).await?;
    }

    Ok(())
}
