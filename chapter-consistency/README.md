# Chapter Consistency
During the lecture, we examined the concept of linearizability, a property that ensures a consistent view of the 
database even when dealing with numerous replicas. To achieve linearizability, we explored various approaches, 
including sequential ordering, causal ordering, timestamp ordering, and Lamport timestamps, ultimately arriving at 
total order broadcast. In this chapter, we will implement a total order broadcast protocol.

## Task
To dependably replicate messages to replicas in a total order, we must develop a total order broadcast protocol. 
We will use a sequencer to order the replicated messages. To demonstrate the functionality of the total order 
broadcast, we will initiate three replicas (TCP servers) and replicate messages to all of them based on client input.
- [ ] Implement [handle_client_connection](patrick-db-sequencer/src/main.rs) to get a sequence number
- [ ] Implement [start_server](patrick-db-tob/src/replica.rs)
- [ ] Implement [handle_tob](patrick-db-tob/src/replica.rs)
- [ ] Implement [handle_update_key](patrick-db-tob/src/replica.rs)

## Test
- start sequencer: `cd patrick-db-sequencer && cargo run`
- start total order broadcast: `cd patrick-db-tob && cargo run`
- if the assert statement passes you are done