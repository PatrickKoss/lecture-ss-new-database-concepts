use std::net::SocketAddr;

use anyhow::Result;
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener};

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

pub async fn start_leader_server(addr: SocketAddr) -> Result<()> {
    log::info!("Starting leader server");
    let listener: TcpListener = TcpListener::bind(addr).await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        log::info!("Accepted connection from: {}", stream.peer_addr().unwrap());

        tokio::spawn(handle_client_connection(stream));
    }
}

pub async fn handle_client_connection(mut stream: tokio::net::TcpStream) -> Result<()> {
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

        handle_request(&mut stream, request).await?;
    }
}

async fn handle_request(stream: &mut tokio::net::TcpStream, request: Request) -> Result<()> {
    match request.action_type {
        ActionType::Get => {
            handle_get_key(stream, request).await?;
        }
        ActionType::Update => {
            handle_update_key(stream, request).await?;
        }
        ActionType::Delete => {
            handle_delete_key(stream, request).await?;
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
    todo!("get key from hashmap_log and send appropriate response")
}

async fn handle_update_key(stream: &mut tokio::net::TcpStream, request: Request) -> Result<()> {
    match request.clone().key_value {
        Some(key_value) => {
            todo!("insert key to hashmap_log and send appropriate response")
        }
        None => {
            todo!("handle bad request")
        }
    }
}

async fn handle_delete_key(stream: &mut tokio::net::TcpStream, request: Request) -> Result<()> {
    match request.clone().route.split("/keys/").last() {
        Some(key) => {
            delete_key(stream, key).await?;

            Ok(())
        }
        None => {
            send_bad_request(stream).await?;

            Ok(())
        }
    }
}

async fn delete_key(stream: &mut tokio::net::TcpStream, key: &str) -> Result<()> {
    todo!("delete key from hashmap_log and send appropriate response")
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
