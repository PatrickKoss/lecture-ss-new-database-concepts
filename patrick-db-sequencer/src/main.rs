use std::env;
use bincode::serialize;
use clap::{Parser};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{Ordering, AtomicU64};

static GLOBAL_SEQUENCE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:7979")]
    addr: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Response {
    counter: u64,
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let args = Args::parse();
    let addr = env::var("ADDR").ok().unwrap_or(args.addr);

    log::info!("Starting leader server");
    let listener: TcpListener = TcpListener::bind(addr).await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        log::info!("Accepted connection from: {}", stream.peer_addr().unwrap());
        tokio::spawn(handle_client_connection(stream));
    }
}

async fn handle_client_connection(mut stream: tokio::net::TcpStream) -> Result<()> {
    loop {
        log::info!("Handling client: {}", stream.peer_addr()?);

        // Read the message type from the stream
        let mut request_buf = [0; 256];
        let n = stream.read(&mut request_buf).await.unwrap();
        if n == 0 {
            log::info!("Connection closed by client: {}", stream.peer_addr()?);

            return Ok(());
        }

        let old_count = GLOBAL_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let response = Response { counter: old_count };

        let serialized = serialize(&response)?;
        stream.write_all(&serialized).await?;
    }
}
