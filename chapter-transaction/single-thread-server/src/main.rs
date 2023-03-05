use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use anyhow::Result;
use httparse::Request as HRequest;

#[derive(Debug, Deserialize, Serialize)]
struct KeyValue {
    key: String,
    value: String,
}

#[tokio::main]
async fn main() {
    log::info!("Starting server");
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let mut map = std::collections::HashMap::<String, String>::new();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        log::info!("Accepted connection from: {}", stream.peer_addr().unwrap());

        handle_connection(stream, &mut map).await;
    }
}

async fn handle_connection(mut stream: TcpStream, map: &mut std::collections::HashMap<String, String>) -> Result<()> {
    // Read the message type from the stream
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await.unwrap();

    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = HRequest::new(&mut headers);
    req.parse(&buffer).unwrap();

    let body_start = req.headers.len();
    let body_end = buffer.len();
    let body = &buffer[body_start..body_end];

    todo!("\
    \read the body line by line by first parsing it to string String::from_utf8_lossy(body) and then iterate over it with lines()\
    \then find the first empty line and start building a new string with the json body.\
    \the string body looks like this:\
    \\Content-Type: application/json\
    \\Content-Length: 20\
    \\\
    \\{{\"key\":\"foo\",\"value\":\"bar\"}}\
    \\keep in mind we need to trim the string by '\0'\
    \\after that we can deserialize the json body to KeyValue struct\
    \\serde_json::from_str::<KeyValue>(&string_body)
    \\then insert the key value pair to the hashmap if it is valid\
    ");

    // random map access
    map.contains_key("foo");
    map.contains_key("bar");
    map.contains_key("baz");

    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes()).await.unwrap();


    Ok(())
}