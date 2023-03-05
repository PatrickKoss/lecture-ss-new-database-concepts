# Chapter Indexing
In this chapter, we will implement a global hashmap that stores the key-value pairs.
The hashmap will be persisted to an append-only log, ensuring that data is not
lost in case of a system failure. We will also provide methods to get, create,
and delete key-value pairs in the hashmap. This chapter will provide a solid foundation
for building a persistent key-value database.

## Task
- [ ] Implement [replay_log](src/main.rs)
- [ ] Implement [get](src/main.rs)
- [ ] Implement [insert](src/main.rs)
- [ ] Implement [delete](src/main.rs)

## Test
- get: `cargo run -- get key`
- insert: `cargo run -- add key value`
- delete: `cargo run -- delete key`