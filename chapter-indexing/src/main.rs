#[macro_use]
extern crate lazy_static;

use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::sync::{Arc};
use dashmap::mapref::one::Ref;
use anyhow::Result;
use tokio::sync::Mutex;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Subcommand, PartialEq, Debug)]
enum Action {
    Add {key: String, value: String},
    Get {key: String},
    Delete {key: String},
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "log")]
    file: String,
    #[command(subcommand)]
    action: Action,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct KeyValue {
    pub key: String,
    pub value: Vec<u8>,
    pub update: bool,
}

lazy_static! {
    static ref HASHMAP: dashmap::DashMap<String, Vec<u8>> = {
        dashmap::DashMap::new()
    };
    static ref FILE: Mutex<BufWriter<File>> = {
        let args = Args::parse();
        let f = env::var("FILE").ok().unwrap_or(args.file);

        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .append(true)
            .create(true)
            .open(f)
            .unwrap();

        Mutex::new(BufWriter::new(file))
    };
}

pub fn get<'a>(key: &str) -> Option<Ref<'a, String, Vec<u8>>> {
    todo!("get value from the dashmap")
}

pub async fn insert(key: String, value: Vec<u8>) -> Result<()> {
    todo!("insert value into the dashmap and write to the log file")
}

pub async fn delete(key: &str) -> Result<()> {
    todo!("delete value from the dashmap and write to the log file")
}

pub fn replay_log(filename: &str) {
    let buf = get_file_as_byte_vec(filename);
    let mut pos = 0;
    while let Ok(key_value) = bincode::deserialize::<KeyValue>(&buf[pos..]) {
        todo!("increment pos by the size of the deserialized KeyValue, insert or delete the key-value pair in the dashmap based on the update field");
    }
}

fn get_file_as_byte_vec(filename: &str) -> Vec<u8> {
    let mut f = OpenOptions::new()
        .write(true)
        .read(true)
        .append(true)
        .create(true)
        .open(filename)
        .unwrap();
    let metadata = fs::metadata(filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read_exact(&mut buffer).expect("buffer overflow");

    buffer
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    replay_log(&args.file);
    match args.action {
        Action::Add {key, value} => {
            println!("add");
            println!("{}", &key);
            println!("{}", &value);
            insert(key, value.into_bytes()).await?
        }
        Action::Get {key } => {
            println!("get");
            println!("{}", &key);
            get(&key).map(|r| println!("{:?}", r));
        }
        Action::Delete {key} => {
            println!("delete");
            println!("{}", &key);
            delete(&key).await?
        }
    }

    Ok(())
}
