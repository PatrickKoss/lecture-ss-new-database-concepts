# Chapter Replication
In this chapter, we will evolve our database to be highly available and replicate the
database to multiple nodes. We will implement a replication protocol that ensures that
data is consistent across all nodes, even in the presence of node failures.
We will also discuss various replication topologies and their tradeoffs.

## Task
- [ ] Implement [main/tokio::spawn](patrick-db/src/main.rs)
- [ ] Implement [handle_update_key](patrick-db/src/server.rs)
- [ ] Implement [delete_key](patrick-db/src/server.rs)

## Test
- Start follower: `cd patrick-db && cargo run -- --leader=false --addr=127.0.0.1:8081 --file=log2 --replicas=""`
- start db server: `cd patrick-db && cargo run`
- start rest api: `cd patrick-db-rest-api && cargo run`
- create key: `curl -X POST -H "Content-Type: application/json" -d '{"key": "foo", "value": "bar"}' http://localhost:8000/keys`
- get key: `curl http://localhost:8000/keys/foo`
- delete key: `curl -X DELETE http://localhost:8000/keys/foo`
- check if log was replicated: `cat patrick-db/log2`