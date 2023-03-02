#[macro_use]
extern crate lazy_static;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use anyhow::Result;
use httparse::Request as HRequest;

lazy_static! {
    static ref HASHMAP: dashmap::DashMap<String, String> = {
        dashmap::DashMap::new()
    };
}
#[derive(Debug, Deserialize, Serialize)]
struct KeyValue {
    key: String,
    value: String,
}

#[tokio::main]
async fn main() {
    log::info!("Starting server");
    let listener = TcpListener::bind("127.0.0.1:8081").await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        log::info!("Accepted connection from: {}", stream.peer_addr().unwrap());

        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(mut stream: TcpStream) -> Result<()> {
    // Read the message type from the stream
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await.unwrap();

    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = HRequest::new(&mut headers);
    req.parse(&buffer).unwrap();

    let body_start = req.headers.len();
    let body_end = buffer.len();
    let body = &buffer[body_start..body_end];

    let mut new_body = String::new();
    let mut found_empty_line = false;
    for (_i, line) in String::from_utf8_lossy(body).lines().enumerate() {
        if line.is_empty() {
            found_empty_line = true;
            continue;
        }
        if found_empty_line {
            new_body.push_str(line.trim_end_matches('\0'));
            new_body.push('\n');
        }
    }

    if let Ok(key_value) = serde_json::from_str::<KeyValue>(&new_body) {
        HASHMAP.insert(key_value.key, key_value.value);
    }

    // random map access
    HASHMAP.contains_key("foo");
    HASHMAP.contains_key("bar");
    HASHMAP.contains_key("baz");

    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes()).await.unwrap();


    Ok(())
}