# Lecture New Database Concepts
In the lecture we will create a key value store called Patrick-DB from scratch.  
Patrick-DB is a persistent key value database that is designed to be similar to 
Memcached but with the added benefit of persisting the in-memory hashmap to disk. 
This database can be replicated and partitioned across multiple nodes, making it ideal for 
large-scale applications.

## Features
- Persistent storage: Patrick-DB persists the in-memory hashmap to disk, 
ensuring that data is not lost in case of a system failure.
- Replication: Patrick-DB supports replication, making it possible to distribute data across 
multiple nodes for high availability and load balancing.
- Partitioning: Patrick-DB supports partitioning, making it possible to distribute data 
across multiple nodes for improved performance and scalability.
- Easy to use API: Patrick-DB provides a simple and easy to use API for storing, 
retrieving and deleting key-value pairs.

## Chapters
To realize the features we will have to implement the following sub topics

### Index structures
In this chapter, we will implement a global hashmap that stores the key-value pairs. 
The hashmap will be persisted to an append-only log, ensuring that data is not 
lost in case of a system failure. We will also provide methods to get, create, 
and delete key-value pairs in the hashmap. This chapter will provide a solid foundation 
for building a persistent key-value database.

### Encoding and Evolvability
In this chapter, we will implement a database server, client, and a REST API that 
will use the client to connect to the server. We will discuss various encoding schemes 
that can be used to represent data in a compact format and allow for evolvability 
of the database schema. We will also demonstrate how to handle versioning of the 
database schema in a backward-compatible way. Therefore, we will create the Patrick-DB protocol.
A binary protocol that is sent over a tcp connection.

### Replication
In this chapter, we will evolve our database to be highly available and replicate the 
database to multiple nodes. We will implement a replication protocol that ensures that 
data is consistent across all nodes, even in the presence of node failures. 
We will also discuss various replication topologies and their tradeoffs.

### Partitioning
To handle big data, we need to partition the data across multiple nodes. In this chapter, 
we will implement simple client-side partitioning to distribute data across multiple nodes. 
We will discuss various partitioning schemes and their tradeoffs, such as consistent hashing 
and range partitioning. We will also discuss how to handle data rebalancing and node failures 
in a partitioned system.