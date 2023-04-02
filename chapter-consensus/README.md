# Chapter Consensus
In the final section of the lecture, we delved into the criticality of achieving consensus, 
particularly for use in total order broadcast or single leader replication. 
In this chapter, we will dive deeper into the raft library and undertake the implementation of a raft store.

## Task
Get familiar with the raft library and implement a raft store.
- [ ] Implement [Storage](patrick-db-raft/src/raft_storage.rs)

## Test
- run the program and see what happens: `cd patrick-db-raft && cargo run`
- You should see the following logs:
  - became follower at term 1, term: 1, raft_id: 3
  - became leader at term 1, term: 1, raft_id: 2
  - became follower at term 1, term: 1, raft_id: 1
  - Sending from 1 to 2, msg: msg_type: MsgPropose to: 2 from: 1 entries {data: "\001"}, to: 2, from: 1, raft_id: 1
  - Sending from 3 to 2, msg: msg_type: MsgPropose to: 2 from: 3 entries {data: "\001"}, to: 2, from: 3, raft_id: 3