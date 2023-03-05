#[macro_use]
extern crate lazy_static;

use std::env;
use std::time::Duration;


use tokio::net::{TcpStream};
use tokio::sync::mpsc;
use clap::{Parser};
use tokio::time;
use crate::server::{start_follower_server, start_leader_server, Request};

mod hashmap_log;
mod server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    addr: String,
    #[arg(short, long, default_value = "true")]
    leader: String,
    #[arg(short, long, default_value = "127.0.0.1:8081")]
    replicas: String,
    #[arg(short, long, default_value = "log")]
    file: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let addr = env::var("ADDR").ok().unwrap_or(args.addr);
    let leader = env::var("LEADER").ok().unwrap_or(args.leader);
    let _replicas = env::var("REPLICAS").ok().unwrap_or(args.replicas);

    hashmap_log::replay_log(&args.file);
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Create a multi-producer, consumer channel with a buffer size of 100
    let (tx, _rx) = mpsc::channel::<Request>(100);

    // Spawn a task to receive messages from the channel
    tokio::spawn(async move {
        todo!("\
        connect to all replicas provided by the variable replicas (comma separated).\
        Receive a message from the channel and send it to all replicas.\
        To send a message to a replica iterate over the connected tcp streams, serialize the message and write to the stream.
        ");
        let mut _tcp_connections = Vec::<TcpStream>::new();
    });

    if leader == "true" {
        start_leader_server(addr.parse().unwrap(), tx).await.unwrap();
    } else {
        start_follower_server(addr.parse().unwrap()).await.unwrap();
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
