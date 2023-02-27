use std::net::SocketAddr;

use anyhow::Result;
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener};
use tokio::sync::mpsc::Sender;

use crate::hashmap_log;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyValue {
    pub key: String,
    pub value: Vec<u8>,
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
    pub key_value: Option<KeyValue>,
    pub route: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub status: u16,
    pub key_value: Option<KeyValue>,
}

pub async fn start_leader_server(addr: SocketAddr, tx: Sender<Request>) -> Result<()> {
    log::info!("Starting leader server");
    let listener = TcpListener::bind(addr).await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        log::info!("Accepted connection from: {}", stream.peer_addr().unwrap());

        let tx = tx.clone();
        tokio::spawn(handle_client_connection(stream, tx));
    }
}

pub async fn start_follower_server(addr: SocketAddr) -> Result<()> {
    log::info!("Starting follower server on: {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        log::info!("Accepted connection from: {}", stream.peer_addr().unwrap());

        tokio::spawn(handle_replica_request(stream));
    }
}

pub async fn handle_client_connection(mut stream: tokio::net::TcpStream, tx: Sender<Request>) -> Result<()> {
    loop {
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

        let tx = tx.clone();
        handle_request(&mut stream, request, Some(&tx)).await?;
    }
}

async fn handle_replica_request(mut stream: tokio::net::TcpStream) -> Result<()> {
    loop {
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

        handle_request(&mut stream, request, None).await?;
    }
}

async fn handle_request(stream: &mut tokio::net::TcpStream, request: Request, tx: Option<&Sender<Request>>) -> Result<()> {
    match request.action_type {
        ActionType::Get => {
            handle_get_key(stream, request).await?;
        }
        ActionType::Update => {
            handle_update_key(stream, request, tx).await?;
        }
        ActionType::Delete => {
            handle_delete_key(stream, request, tx).await?;
        }
    }

    Ok(())
}

async fn handle_get_key(stream: &mut tokio::net::TcpStream, request: Request) -> Result<()> {
    match request.route.split("/keys/").last() {
        Some(key) => {
            get_key(stream, key).await?;

            Ok(())
        }
        None => {
            send_bad_request(stream).await?;

            Ok(())
        }
    }
}

async fn get_key(stream: &mut tokio::net::TcpStream, key: &str) -> Result<()> {
    match hashmap_log::get(key) {
        Some(key_value) => {
            let response = Response {
                status: 200,
                key_value: Some(KeyValue {
                    key: key.to_string(),
                    value: key_value.value().clone(),
                }),
            };

            send_response(stream, response).await?;

            Ok(())
        }
        None => {
            send_not_found(stream).await?;

            Ok(())
        }
    }
}

async fn handle_update_key(stream: &mut tokio::net::TcpStream, request: Request, tx: Option<&Sender<Request>>) -> Result<()> {
    match request.clone().key_value {
        Some(key_value) => {
            hashmap_log::insert(key_value.key.clone(), key_value.value.clone()).await?;
            if let Some(tx) = tx {
                if let Err(e) = tx.send(request).await {
                    log::warn!("Sending to channel for replication failed: {}", e);
                }
            }

            send_ok(stream).await?;

            Ok(())
        }
        None => {
            send_bad_request(stream).await?;

            Ok(())
        }
    }
}

async fn handle_delete_key(stream: &mut tokio::net::TcpStream, request: Request, tx: Option<&Sender<Request>>) -> Result<()> {
    match request.clone().route.split("/keys/").last() {
        Some(key) => {
            delete_key(stream, key, request, tx).await?;

            Ok(())
        }
        None => {
            send_bad_request(stream).await?;

            Ok(())
        }
    }
}

async fn delete_key(stream: &mut tokio::net::TcpStream, key: &str,  request: Request, tx: Option<&Sender<Request>>) -> Result<()> {
    match hashmap_log::delete(key).await {
        Ok(_) => {
            if let Some(tx) = tx {
                if let Err(e) = tx.send(request).await {
                    log::warn!("Sending to channel for replication failed: {}", e);
                }
            }
            send_ok(stream).await?;

            Ok(())
        }
        Err(_) => {
            send_not_found(stream).await?;

            Ok(())
        }
    }
}

async fn send_bad_request(stream: &mut tokio::net::TcpStream) -> Result<()> {
    let response = Response {
        status: 400,
        key_value: None,
    };

    send_response(stream, response).await
}

async fn send_not_found(stream: &mut tokio::net::TcpStream) -> Result<()> {
    let response = Response {
        status: 404,
        key_value: None,
    };

    send_response(stream, response).await
}

async fn send_ok(stream: &mut tokio::net::TcpStream) -> Result<()> {
    let response = Response {
        status: 200,
        key_value: None,
    };

    send_response(stream, response).await
}

async fn send_response(stream: &mut tokio::net::TcpStream, response: Response) -> Result<()> {
    let serialized = serialize(&response)?;
    stream.write_all(&serialized).await?;

    log::debug!("Sent response: {:?}", response);

    Ok(())
}
