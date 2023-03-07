# Chapter Indexing
In this chapter, we will implement a global hashmap that stores the key-value pairs.
The hashmap will be persisted to an append-only log, ensuring that data is not
lost in case of a system failure. We will also provide methods to get, create,
and delete key-value pairs in the hashmap. This chapter will provide a solid foundation
for building a persistent key-value database.

## Task
The initial step in implementing the key-value store is to establish the index mechanism. 
To accomplish this, we create a global hashmap that will store the key-value pairs, 
and persist this hashmap to a log file. The get method will read the hashmap and 
return the value for the given key. The insert method will insert the key-value pair 
into the hashmap and append the pair to the log. The delete method will remove the key-value 
pair from the hashmap and append the pair to the log. Finally, the replay_log method will read 
the log file and insert the key-value pairs into the hashmap. To validate the correctness 
of the implementation, we can use the command-line interface tool to perform get, insert, 
and delete operations on key-value pairs.

- [ ] Implement [replay_log](src/main.rs)
- [ ] Implement [get](src/main.rs)
- [ ] Implement [insert](src/main.rs)
- [ ] Implement [delete](src/main.rs)

## Test
- get: `cargo run -- get key`
- insert: `cargo run -- add key value`
- delete: `cargo run -- delete key`