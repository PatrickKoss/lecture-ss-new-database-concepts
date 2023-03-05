#[macro_use]
extern crate lazy_static;

use std::env;
use clap::{Parser};
use crate::server::{start_leader_server};

mod hashmap_log;
mod server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    addr: String,
    #[arg(short, long, default_value = "true")]
    leader: String,
    #[arg(short, long, default_value = "127.0.0.1:8081")]
    replicas: String,
    #[arg(short, long, default_value = "log")]
    file: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let addr = env::var("ADDR").ok().unwrap_or(args.addr);

    hashmap_log::replay_log(&args.file);
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    start_leader_server(addr.parse().unwrap()).await.unwrap();
}
