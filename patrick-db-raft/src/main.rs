use std::{time::{Duration, Instant}};
use std::collections::HashMap;

use raft::{Config, GetEntriesContext, raw_node::RawNode, Result as RaftResult, Error as RaftError, Storage as RaftStorage};
use raft::prelude::*;
use slog::{Drain, Logger, o};
use raft::eraftpb::Entry;

use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{Receiver, Sender};
use protobuf::CachedSize;
use protobuf::UnknownFields;
use protobuf::SingularPtrField;
use bytes::Bytes;
use tokio::sync::mpsc::channel;
use tokio::sync::Mutex;

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
    RaftEntry(Entry),
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
    hard_state: RwLock<HardState>,
    conf_state: RwLock<ConfState>,
    entries: RwLock<Vec<Entry>>,
    snapshot: RwLock<Option<Snapshot>>,
}

impl Storage {
    pub fn new(voters: Vec<u64>) -> Storage {
        let mut conf_state = ConfState::default();
        conf_state.set_voters(voters);

        Storage {
            hard_state: RwLock::new(HardState::default()),
            conf_state: RwLock::new(conf_state),
            entries: RwLock::new(Vec::new()),
            snapshot: RwLock::new(None),
        }
    }

    pub fn save_entries(&self, new_entries: &[Entry]) -> RaftResult<()> {
        let mut entries = self.entries.write().unwrap();
        for entry in new_entries {
            entries.push(entry.clone());
        }
        Ok(())
    }

    pub fn update_hard_state(&self, new_hard_state: HardState) -> RaftResult<()> {
        let mut hard_state = self.hard_state.write().unwrap();
        *hard_state = new_hard_state;
        Ok(())
    }

    pub fn update_conf_state(&self, new_conf_state: ConfState) -> RaftResult<()> {
        let mut conf_state = self.conf_state.write().unwrap();
        *conf_state = new_conf_state;
        Ok(())
    }

    pub fn create_snapshot(&self, request_index: u64, term: u64, conf_state: ConfState) -> RaftResult<Snapshot> {
        let conf_state_field:SingularPtrField<ConfState> = SingularPtrField::some(conf_state);
        let snapshot_metadata = SnapshotMetadata {
            index: request_index,
            term,
            cached_size: CachedSize::default(),
            conf_state: conf_state_field,
            unknown_fields: UnknownFields::default(),
        };

        let snap_shot_field: SingularPtrField<SnapshotMetadata> = SingularPtrField::some(snapshot_metadata);
        let snapshot = Snapshot {
            metadata: snap_shot_field,
            data: Bytes::new(),
            unknown_fields: UnknownFields::default(),
            cached_size: CachedSize::default(), // You can store any relevant data for your application here.
        };

        {
            let mut stored_snapshot = self.snapshot.write().unwrap();
            *stored_snapshot = Some(snapshot.clone());
        }

        Ok(snapshot)
    }

    pub fn apply_snapshot(&self, snapshot: Snapshot) -> RaftResult<()> {
        let mut stored_snapshot = self.snapshot.write().unwrap();
        *stored_snapshot = Some(snapshot);
        Ok(())
    }
}

impl raft::Storage for Storage {
    fn initial_state(&self) -> RaftResult<RaftState> {
        let hard_state = self.hard_state.read().unwrap().clone();
        let conf_state = self.conf_state.write().unwrap().clone();

        Ok(RaftState {
            hard_state,
            conf_state,
        })
    }

    fn entries(&self, low: u64, high: u64, _max_size: impl Into<Option<u64>>, _context: GetEntriesContext) -> RaftResult<Vec<Entry>> {
        let entries = self.entries.read().unwrap();
        let res = entries[low as usize..high as usize].to_vec();
        Ok(res)
    }

    fn term(&self, idx: u64) -> RaftResult<u64> {
        if idx == 0 {
            return Ok(0);
        }

        let entries = self.entries.read().unwrap();
        entries.get((idx as usize) - 1).map(|entry| entry.term).ok_or(RaftError::Store(raft::StorageError::Unavailable))
    }

    fn first_index(&self) -> RaftResult<u64> {
        let entries = self.entries.read().unwrap();
        Ok(entries.first().map(|entry| entry.index).unwrap_or(1))
    }

    fn last_index(&self) -> RaftResult<u64> {
        let entries = self.entries.read().unwrap();
        Ok(entries.last().map(|entry| entry.index).unwrap_or(0))
    }

    fn snapshot(&self, _request_index: u64, _to: u64) -> RaftResult<Snapshot> {
        let snapshot: Snapshot = match self.snapshot.read().unwrap().as_ref() {
            Some(s) => s.clone(),
            None => return Err(RaftError::Store(raft::StorageError::SnapshotOutOfDate)),
        };

        Ok(snapshot)
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
    // let storage = MemStorage::new_with_conf_state((vec![1,2,3,4,5], vec![]));
    let node_channels = Arc::new(Mutex::new(HashMap::<u64, Sender<Msg>>::new()));
    let voters = vec![1, 2, 3];
    for voter in &voters {
        let config = Config {
            id: *voter,
            heartbeat_tick: 1,
            election_tick: 3,
            ..Default::default()
        };
        let storage = Storage::new(voters.clone());
        let node = RawNode::new(&config, storage, &logger).unwrap();
        let (tx, rx) = channel::<Msg>(1000);
        let mut node_channels_insert = node_channels.lock().await;
        node_channels_insert.insert(*voter, tx.clone());
        drop(node_channels_insert);
        let node_channels_clone = node_channels.clone();
        tokio::spawn(async move {
            run_node(node, rx, node_channels_clone).await;
        });
    }

    // Simulate a message coming down the stream.
    let mut node_channels = node_channels.lock().await;
    let sender_node = 1_u64;
    if let Some(channel) = node_channels.get(&sender_node) {
        let tx = channel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5000)).await;
            let mut interval = tokio::time::interval(Duration::from_millis(3000));
            loop {
                interval.tick().await;
                if let Err(e) = tx.send(Msg::Propose {
                    id: 1,
                    callback: Box::new(|| ()),
                }).await {
                    log::warn!("Sending tick failed failed: {}", e);
                }
            }
        });
    }
    drop(node_channels);

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}

async fn run_node(mut node: RawNode<Storage>, mut rx: Receiver<Msg>, node_channels: Arc<Mutex<HashMap<u64, Sender<Msg>>>>) {
    // Ticking the Raft node
    let timeout = Duration::from_millis(100);
    let mut remaining_timeout = timeout;

    loop {
        let now = Instant::now();

        match rx.recv().await {
            Some(Msg::Propose { id, callback: _ }) => {
                log::info!("Propose: {:?}", id);
                if let Err(e) = node.propose(vec![], vec![id]) {
                    log::error!("Propose error: {:?}", e);
                }
            }
            Some(Msg::Raft(m)) => {
                log::info!("got raft message: {:?}", m);
                log::info!("entries: {:?}", m.get_entries());
                node.step(m).unwrap()
            },
            Some(Msg::RaftEntry(entry)) => node.mut_store().save_entries(&[entry]).unwrap(),
            None => (),
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

        for msg in ready.take_messages() {
            log::info!("take_messages and send them to peers: {:?}", msg);
            let mut node_channels = node_channels.lock().await;
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
            let mut node_channels = node_channels.lock().await;
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
    }
}
