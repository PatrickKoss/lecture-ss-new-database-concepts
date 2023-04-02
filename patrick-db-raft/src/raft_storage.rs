use std::sync::RwLock;

use bytes::Bytes;
use protobuf::CachedSize;
use protobuf::SingularPtrField;
use protobuf::UnknownFields;
use raft::{Error as RaftError, GetEntriesContext, Result as RaftResult, Storage as RaftStorage};
use raft::eraftpb::Entry;
use raft::prelude::*;
use slog::Drain;

pub struct Storage {
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
        let conf_state_field: SingularPtrField<ConfState> = SingularPtrField::some(conf_state);
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
        log::info!("term({})", idx);
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