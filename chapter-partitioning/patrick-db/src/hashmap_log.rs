use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::sync::{Arc};
use dashmap::mapref::one::Ref;
use anyhow::Result;
use tokio::sync::Mutex;
use std::env;

use serde::{Deserialize, Serialize};
use clap::{Parser};

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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct KeyValue {
    pub key: String,
    pub value: Vec<u8>,
    pub update: bool,
}

pub fn get<'a>(key: &str) -> Option<Ref<'a, String, Vec<u8>>> {
    HASHMAP.get(key)
}

pub async fn insert(key: String, value: Vec<u8>) -> Result<()> {
    let mut file = FILE.lock().await;
    let key_value = KeyValue {
        key,
        value,
        update: true,
    };
    let buf = bincode::serialize(&key_value)?;
    file.write_all(&buf)?;
    file.flush()?;

    HASHMAP.insert(key_value.key, key_value.value);

    Ok(())
}

pub async fn delete(key: &str) -> Result<()> {
    let mut file = FILE.lock().await;
    let key_value = KeyValue {
        key: key.to_string(),
        value: vec![],
        update: false,
    };
    let buf = bincode::serialize(&key_value)?;
    file.write_all(&buf)?;
    file.flush()?;

    HASHMAP.remove(key);

    Ok(())
}

pub fn replay_log(filename: &str) {
    let buf = get_file_as_byte_vec(filename);
    let mut pos = 0;
    while let Ok(key_value) = bincode::deserialize::<KeyValue>(&buf[pos..]) {
        pos += bincode::serialized_size(&key_value).unwrap() as usize;
        if key_value.update {
            HASHMAP.insert(key_value.key, key_value.value);
        } else {
            HASHMAP.remove(&key_value.key);
        }
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
