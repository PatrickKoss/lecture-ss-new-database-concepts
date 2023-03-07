# Chapter Encoding and Evolvability
In this chapter, we will implement a database server, client, and a REST API that
will use the client to connect to the server. We will discuss various encoding schemes
that can be used to represent data in a compact format and allow for evolvability
of the database schema. We will also demonstrate how to handle versioning of the
database schema in a backward-compatible way. Therefore, we will create the Patrick-DB protocol.
A binary protocol that is sent over a tcp connection.

## Task
Once we have constructed our storage engine, we must devise a means of communicating with it. 
The standard approach to interacting with a database involves utilizing a client-server 
architecture. Specifically, the server will function as a tokio tcp listener that accepts 
incoming connections, while the client will function as a tokio tcp client that establishes 
a connection with the server. The client will send requests to the server via TCP using our 
custom protocol, and the server will process the request and send a response back to the client.
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