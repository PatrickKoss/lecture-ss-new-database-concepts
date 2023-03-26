use std::{sync::mpsc::{channel, RecvTimeoutError}, time::{Duration, Instant}};

use std::sync::Mutex;

use raft::{Config, GetEntriesContext, raw_node::RawNode};
use raft::prelude::*;
use slog::{Drain, Logger, o};
use raft::eraftpb::Entry;
use raft::storage::MemStorage;
use regex::internal::Input;

fn create_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    Logger::root(drain, o!())
}

enum Msg {
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

struct Storage {
    term: Mutex<u64>,
    first_index: Mutex<u64>,
    last_index: Mutex<u64>,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            term: Mutex::new(0),
            first_index: Mutex::new(1),
            last_index: Mutex::new(1),
        }
    }
}

impl raft::Storage for Storage {
    fn initial_state(&self) -> raft::Result<RaftState> {
        log::info!("initial_state");
        let hard_state = HardState::default();
        let mut conf_state = ConfState::default();
        conf_state.set_voters(vec![1, 2, 3, 4, 5]);
        Ok(RaftState {
            hard_state,
            conf_state,
        })
    }

    fn entries(&self, low: u64, high: u64, max_size: impl Into<Option<u64>>, context: GetEntriesContext) -> raft::Result<Vec<Entry>> {
        log::info!("entries: {:?}, {:?}, {:?}, {:?}", low, high, max_size.into(), context);
        Ok(Vec::new())
    }

    fn term(&self, idx: u64) -> raft::Result<u64> {
        log::info!("term: {:?}", idx);
        let mut term = self.term.lock().unwrap();
        *term += 1;
        Ok(*term)
    }

    fn first_index(&self) -> raft::Result<u64> {
        log::info!("first_index");
        let mut first_index = self.first_index.lock().unwrap();
        *first_index += 1;
        Ok(*first_index)
    }

    fn last_index(&self) -> raft::Result<u64> {
        log::info!("last_index");
        let mut last_index = self.last_index.lock().unwrap();
        *last_index += 1;
        Ok(*last_index)
    }

    fn snapshot(&self, request_index: u64, to: u64) -> raft::Result<Snapshot> {
        log::info!("snapshot: {:?}, {:?}", request_index, to);
        Ok(Snapshot::default())
    }
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    // Create Raft node
    let config = Config {
        id: 1,
        ..Default::default()
    };
    config.validate().unwrap();
    let logger = create_logger();
    let storage = MemStorage::new_with_conf_state((vec![1,2,3,4,5], vec![]));
    // let storage = Storage::new();
    let mut node = RawNode::new(&config, storage, &logger).unwrap();

    // Ticking the Raft node
    let (tx, rx) = channel();
    let timeout = Duration::from_millis(100);
    let mut remaining_timeout = timeout;

    // Simulate a message coming down the stream.
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(5000)).await;
        let mut interval = tokio::time::interval(Duration::from_millis(3000));
        loop {
            interval.tick().await;
            tx.send(Msg::Propose {
                id: 1,
                callback: Box::new(|| ()),
            }).unwrap();
        }
    });

    loop {
        let now = Instant::now();
        match rx.recv_timeout(remaining_timeout) {
            Ok(Msg::Propose { id, callback: _ }) => {
                log::info!("Propose: {:?}", id);
                if let Err(e) = node.propose(vec![], vec![id]) {
                    log::error!("Propose error: {:?}", e);
                }
            }
            Ok(Msg::Raft(m)) => node.step(m).unwrap(),
            Err(RecvTimeoutError::Timeout) => (),
            Err(RecvTimeoutError::Disconnected) => unimplemented!(),
        }

        let elapsed = now.elapsed();
        if elapsed >= remaining_timeout {
            remaining_timeout = timeout;
            node.tick();
        } else {
            remaining_timeout -= elapsed;
        }

        if !node.has_ready() {
            continue;
        }

        let mut ready = node.ready();

        for _msg in ready.take_messages() {
            log::info!("take_messages and send them to peers: {:?}", _msg);
            // Send messages to other peers.
        }

        if !ready.snapshot().is_empty() {
            node.mut_store()
                .wl()
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

            match entry.get_entry_type() {
                EntryType::EntryNormal => handle_normal(entry),
                EntryType::EntryConfChange => handle_conf_change(entry),
                EntryType::EntryConfChangeV2 => handle_conf_change_v2(entry),
            }
        }

        if !ready.entries().is_empty() {
            node.mut_store().wl().append(ready.entries()).unwrap();
        }

        if let Some(hs) = ready.hs() {
            node.mut_store().wl().set_hardstate(hs.clone());
        }

        for _msg in ready.take_persisted_messages() {
            log::info!("take_persisted_messages and send them to peers: {:?}", _msg);
            // Send persisted messages to other peers.
        }

        let mut light_rd = node.advance(ready);
        handle_messages(light_rd.take_messages());
        handle_committed_entries(light_rd.take_committed_entries());
        node.advance_apply();
    }
}
