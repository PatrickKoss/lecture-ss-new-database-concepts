# Chapter Partitioning
To handle big data, we need to partition the data across multiple nodes. In this chapter,
we will implement simple client-side partitioning to distribute data across multiple nodes.
We will discuss various partitioning schemes and their tradeoffs, such as consistent hashing
and range partitioning. We will also discuss how to handle data rebalancing and node failures
in a partitioned system.

## Task
- [ ] Implement [new](patrick-db-client/src/lib.rs)
- [ ] Implement [get](patrick-db-client/src/lib.rs)
- [ ] Implement [update](patrick-db-client/src/lib.rs)
- [ ] Implement [delete](patrick-db-client/src/lib.rs)
- [ ] Implement [get_db_index](patrick-db-client/src/lib.rs)
- [ ] Implement [calculate_hash](patrick-db-client/src/lib.rs)

## Test
- Start follower 1: `cd patrick-db && cargo run -- --leader=false --addr=127.0.0.1:8081 --file=log2 --replicas=""`
- Start follower 2: `cd patrick-db && cargo run -- --leader=false --addr=127.0.0.1:8083 --file=log4 --replicas=""`
- start db server 1: `cd patrick-db && cargo run`
- start db server 2: `cd patrick-db && cargo run -- --addr=127.0.0.1:8082 --file=log3 --replicas="127.0.0.1:8083"`
- start rest api: `cd patrick-db-rest-api && cargo run`
- create key: `curl -X POST -H "Content-Type: application/json" -d '{"key": "foo", "value": "bar"}' http://localhost:8000/keys`
- get key: `curl http://localhost:8000/keys/foo`
- delete key: `curl -X DELETE http://localhost:8000/keys/foo`
- check if log was replicated: `cat patrick-db/log2`