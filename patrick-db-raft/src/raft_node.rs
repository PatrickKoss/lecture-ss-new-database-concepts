use std::{time::{Duration, Instant}};
use std::collections::HashMap;
use std::sync::Arc;
use log::log;

use raft::{raw_node::RawNode, Storage as RaftStorage};
use raft::eraftpb::Entry;
use raft::prelude::*;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

use crate::raft_storage::Storage;

pub async fn run_node(mut node: RawNode<Storage>, mut rx: Receiver<Msg>, node_channels: Arc<Mutex<HashMap<u64, Sender<Msg>>>>) {
    let timeout = Duration::from_millis(100);
    let mut remaining_timeout = timeout;

    loop {
        let now = Instant::now();

        match rx.recv().await {
            // used to send message from other thread to node
            Some(Msg::Propose { id, callback: _ }) => {
                // propose send a message from node to leader
                log::info!("Propose: {:?}", id);
                if let Err(e) = node.propose(vec![], vec![id]) {
                    log::error!("Propose error: {:?}", e);
                };
            }
            // used to receive message from other node
            Some(Msg::Raft(m)) => {
                log::info!("got raft message: {:?}", m);
                // step advances the state machine using the given message.
                if let Err(e) = node.step(m) {
                    log::error!("step error: {:?}", e);
                }
                // example propose since rx is dropped after this execution
                if let Err(e) = node.propose(vec![], vec![1]) {
                    log::error!("Propose error: {:?}", e);
                }
                // we could assume continue should solve the issue of dropping rx but it does not
                // continue;
            },
            None => (),
        }

        // tick advances the internal logical clock by a single tick.
        let elapsed = now.elapsed();
        if elapsed >= remaining_timeout {
            remaining_timeout = timeout;
            node.tick();
        } else {
            remaining_timeout -= elapsed;
        }

        // we can only work with nodes that are ready
        if !node.has_ready() {
            continue;
        }
        log::info!("node is ready");

        let mut ready = node.ready();

        for msg in ready.take_messages() {
            log::info!("take_messages and send them to peers: {:?}", msg);
            let node_channels = node_channels.lock().await;
            if let Some(channel) = node_channels.get(&msg.to) {
                let tx = channel.clone();
                if let Err(e) = tx.send(Msg::Raft(msg)).await {
                    log::warn!("sending to channel for replication message failed: {}", e);
                }
            } else {
                log::warn!("recipient not found for message: {:?}", msg);
            }
            drop(node_channels);
        }

        if !ready.snapshot().is_empty() {
            node.mut_store()
                .apply_snapshot(ready.snapshot().clone())
                .unwrap();
        }

        let mut last_apply_index = 0;
        for entry in ready.take_committed_entries() {
            last_apply_index = entry.index;

            if entry.data.is_empty() {
                log::info!("entry data is empty");
                continue;
            }

            // here we can eventually pass the store and handle the changes
            match entry.get_entry_type() {
                EntryType::EntryNormal => handle_normal(entry),
                EntryType::EntryConfChange => handle_conf_change(entry),
                EntryType::EntryConfChangeV2 => handle_conf_change_v2(entry),
            }
        }

        if !ready.entries().is_empty() {
            node.mut_store().save_entries(ready.entries()).unwrap();
        }

        if let Some(hs) = ready.hs() {
            node.mut_store().update_hard_state(hs.clone());
        }

        for msg in ready.take_persisted_messages() {
            log::info!("take_persisted_messages and send them to peers: {:?}", msg);
            let node_channels = node_channels.lock().await;
            if let Some(channel) = node_channels.get(&msg.to) {
                let tx = channel.clone();
                if let Err(e) = tx.send(Msg::Raft(msg)).await {
                    log::warn!("sending to channel for replication message failed: {}", e);
                }
            } else {
                log::warn!("recipient not found for message: {:?}", msg);
            }
            drop(node_channels);
        }

        let mut light_rd = node.advance(ready);
        handle_messages(light_rd.take_messages());
        handle_committed_entries(light_rd.take_committed_entries());
        node.advance_apply();
        log::info!("node applied to {}", last_apply_index);
    }
}

pub enum Msg {
    Propose {
        id: u8,
        callback: Box<dyn Fn() + Send>,
    },
    Raft(Message),
}

fn handle_normal(entry: Entry) {
    log::info!("handle_normal: {:?}", entry)
}

fn handle_conf_change(entry: Entry) {
    log::info!("handle_conf_change: {:?}", entry);
}

fn handle_conf_change_v2(entry: Entry) {
    log::info!("handle_conf_change_v2: {:?}", entry);
}

fn handle_messages(messages: Vec<Message>) {
    log::info!("handle_messages: {:?}", messages);
}

fn handle_committed_entries(committed_entries: Vec<Entry>) {
    log::info!("handle_committed_entries: {:?}", committed_entries);
}