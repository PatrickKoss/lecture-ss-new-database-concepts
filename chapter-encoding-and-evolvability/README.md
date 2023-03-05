# Chapter Encoding and Evolvability
In this chapter, we will implement a database server, client, and a REST API that
will use the client to connect to the server. We will discuss various encoding schemes
that can be used to represent data in a compact format and allow for evolvability
of the database schema. We will also demonstrate how to handle versioning of the
database schema in a backward-compatible way. Therefore, we will create the Patrick-DB protocol.
A binary protocol that is sent over a tcp connection.

## Task
- [ ] Implement [get_key](patrick-db/src/server.rs)
- [ ] Implement [handle_update_key](patrick-db/src/server.rs)
- [ ] Implement [delete_key](patrick-db/src/server.rs)
- [ ] Implement [get](patrick-db-client/src/lib.rs)
- [ ] Implement [update](patrick-db-client/src/lib.rs)
- [ ] Implement [delete](patrick-db-client/src/lib.rs)

## Test
- start db server: `cd patrick-db && cargo run`
- start rest api: `cd patrick-db-rest-api && cargo run`
- create key: `curl -X POST -H "Content-Type: application/json" -d '{"key": "foo", "value": "bar"}' http://localhost:8000/keys`
- get key: `curl http://localhost:8000/keys/foo`
- delete key: `curl -X DELETE http://localhost:8000/keys/foo`