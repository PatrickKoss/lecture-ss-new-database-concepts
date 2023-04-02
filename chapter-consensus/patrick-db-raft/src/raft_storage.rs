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
        todo!("To initialize the RaftState, it is necessary to lock the hard_state and conf_state \
        fields and create clones of them. Once this is done, you should return a result of Ok(RaftState{}).")
    }

    fn entries(&self, low: u64, high: u64, _max_size: impl Into<Option<u64>>, _context: GetEntriesContext) -> RaftResult<Vec<Entry>> {
        todo!("If you want to obtain the entries, you must first lock the entries field. \
        After doing so, you can return a slice of the entries, which should be a vector of \
        entries between the values of low and high. Return a result of the constructed vector.")
    }

    fn term(&self, idx: u64) -> RaftResult<u64> {
        todo!("In case the index is equal to 0, it is necessary to return a result of Ok(0). \
        However, if the index is different than 0, you should lock the entries and obtain the \
        term value of the entry located at (idx as usize) - 1. The term value corresponds to the \
        term field of the Entry.")
    }

    fn first_index(&self) -> RaftResult<u64> {
        todo!("To obtain the first entry of the entries vector, you need to lock it and retrieve \
        the entry located at index field 0. However, if the vector is empty, you should return \
        a result of Ok(1).")
    }

    fn last_index(&self) -> RaftResult<u64> {
        todo!("To obtain the last entry of the entries vector, you need to lock it and retrieve \
        the entry located at index field last. However, if the vector is empty, you should return \
        a result of Ok(1).")
    }

    fn snapshot(&self, _request_index: u64, _to: u64) -> RaftResult<Snapshot> {
        todo!("To retrieve the snapshot, you should lock the snapshot field and create a clone of \
        it. Cloning can be a bit challenging, but one way to do it is to lock the field, unwrap \
        it, and get a reference to it using as_ref(). This will return an Option<&T>, which you \
        can then clone using clone(). Finally, you should return the result of the snapshot.")
    }
}