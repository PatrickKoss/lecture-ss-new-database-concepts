# Chapter Transaction
During the lecture, we discovered that a multi-threaded server can lead to a variety of
problems, and we need to exercise caution to avoid anomalies. As a result, we introduced
the concept of a single-threaded server to circumvent concurrency issues.
In this chapter, we will conduct a small load test to compare the performance of a
single-threaded server versus a multi-threaded server.

## Task
To compare their performance, we will create both a single-threaded server and a multi-threaded server, 
then conduct a benchmark. The servers will parse the HTTP request and write to a hashmap based on the 
request method and body. Once the implementation is complete, we will use [drill](https://github.com/fcsonline/drill)  
to execute the benchmark.
- [ ] Implement [single-thread-server/handle_connection](single-thread-server/src/main.rs)
- [ ] Implement [multi-thread-server/](multi-thread-server/src/main.rs)
- [ ] Run the benchmark `make benchmarksingle && make benchmarkmulti`

## Test
- Run the benchmark single threaded server `make benchmarksingle`
- Run the benchmark multithreaded server `make benchmarkmulti`
- You can play around with the concurrency during the benchmark by adjusting concurrency in [benchmark.yml](drill/benchmark.yml)
and [benchmark2.yml](drill/benchmark.yml)