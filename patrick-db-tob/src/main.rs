mod replica;

use std::env;

use bincode::{deserialize, serialize};
use clap::{Parser};

use tokio::io::{AsyncReadExt, AsyncWriteExt};



use std::sync::{Arc};

use std::time::Duration;

use tokio::sync::{Mutex};
use rand::seq::SliceRandom;


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

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let args = Args::parse();
    // sequencer address
    let sequencer = env::var("SEQUENCER").ok().unwrap_or(args.sequencer);
    // replica addresses comma separated
    let replica_args = env::var("REPLICAS").ok().unwrap_or(args.replicas);
    // replica vector of addresses
    let replica_addrs: Vec<String> = replica_args.split(',').map(|s| s.to_string()).collect();
    // messages to send to replicas comma separated
    let message = env::var("MESSAGES").ok().unwrap_or(args.messages);
    // messages vector of strings
    let messages: Vec<String> = message.split(',').map(|s| s.to_string()).collect();

    // create replicas
    let mut replicas = Vec::new();
    for addr in replica_addrs.clone() {
        replicas.push(replica::Replica::new(addr.clone(), sequencer.clone(), replica_addrs.clone()).await);
    }

    // put replicas in Arc to share them between threads (further progress)
    let replicas: Vec<Arc<replica::Replica>> = replicas
        .into_iter()
        .map(Arc::new)
        .collect();

    // start server of replicas
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
        let stream = replica::connect_with_retry(&addr, 20, Duration::from_secs(2)).await;
        replica_streams.push(Arc::new(Mutex::new(stream)));
    }

    // send messages to random replicas
    for message in messages {
        // select random replica
        let mut rng = rand::thread_rng();
        let replica_stream = replica_streams.choose(&mut rng).unwrap();
        let mut replica_stream = replica_stream.lock().await;

        // send update request to replica
        let msg = replica::Request { action_type: replica::ActionType::Update, message, sequence_number: 0 };
        let msg_buf = serialize(&msg).unwrap();
        replica_stream.write_all(&msg_buf).await.unwrap();
        let mut response_buf = [0; 256];
        replica_stream.read(&mut response_buf).await.unwrap();
    }

    log::info!("sleep 2 seconds to wait for replicas to receive all messages");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // iterate over all replicas and do get request
    for replica_stream in &replica_streams {
        // get all messages from replica
        let mut replica_stream = replica_stream.lock().await;
        let msg = replica::Request { action_type: replica::ActionType::Get, message: "".to_string(), sequence_number: 0 };
        let msg_buf = serialize(&msg).unwrap();
        replica_stream.write_all(&msg_buf).await.unwrap();
        let mut response_buf = [0; 256];
        replica_stream.read(&mut response_buf).await.unwrap();
        let response: replica::Response = deserialize(&response_buf).unwrap();

        // compare messages from replica with messages sent to replicas
        log::info!("messages: {}, addr {}", &response.message, replica_stream.peer_addr().unwrap());
        assert_eq!(response.message, message)
    }
}