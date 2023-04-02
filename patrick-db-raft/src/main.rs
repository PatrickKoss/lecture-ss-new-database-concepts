use std::time::Duration;
use std::collections::HashMap;
use std::sync::Arc;

use raft::{Config, raw_node::RawNode};
use slog::{Drain, Logger, o};
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::channel;
use tokio::sync::Mutex;

mod raft_storage;
mod raft_node;

fn create_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    Logger::root(drain, o!())
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let logger = create_logger();

    let node_channels = Arc::new(Mutex::new(HashMap::<u64, Sender<raft_node::Msg>>::new()));

    // start all raft nodes in tokio thread
    let voters = vec![1, 2, 3];
    for voter in &voters {
        let config = Config {
            id: *voter,
            heartbeat_tick: 1,
            election_tick: 3,
            ..Default::default()
        };

        let storage = raft_storage::Storage::new(voters.clone());
        let node = RawNode::new(&config, storage, &logger).unwrap();

        // to communicate with other nodes we need to create a channel
        let (tx, rx) = channel::<raft_node::Msg>(1000);
        let mut node_channels_insert = node_channels.lock().await;

        // insert channel into hashmap
        node_channels_insert.insert(*voter, tx.clone());
        drop(node_channels_insert);

        // pass hashmap to node
        let node_channels_clone = node_channels.clone();
        tokio::spawn(async move {
            raft_node::run_node(node, rx, node_channels_clone).await;
        });
    }

    // Simulate a message coming down the stream.
    let node_channels_clone = node_channels.clone();
    let node_channels = node_channels_clone.lock().await;
    let sender_node = 2_u64;
    if let Some(channel) = node_channels.get(&sender_node) {
        let tx = channel.clone();
        drop(node_channels);
        tokio::time::sleep(Duration::from_millis(5000)).await;
        let mut interval = tokio::time::interval(Duration::from_millis(3000));
        loop {
            interval.tick().await;
            if let Err(e) = tx.send(raft_node::Msg::Propose {
                id: 2,
                callback: Box::new(|| ()),
            }).await {
                log::warn!("Sending tick failed: {}", e);
            }
        }
    }
}
