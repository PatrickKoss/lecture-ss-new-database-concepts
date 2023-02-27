use actix_web::{App, delete, get, HttpResponse, HttpServer, middleware, post, web};
use anyhow::Result;
use bincode::{deserialize, serialize};
use patrick_db_client::PDBConnectionPool;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyValue {
    pub key: String,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    pub message: String,
}

#[get("/")]
async fn home() -> HttpResponse {
    HttpResponse::Ok().json(Message {
        message: "connection works".to_string(),
    })
}

#[post("/keys")]
async fn create_key(pool: web::Data<PDBConnectionPool>, key_value: web::Json<KeyValue>) -> HttpResponse {
    let key_value = key_value.into_inner();
    let val_binary = serialize(&key_value.value).unwrap();
    let db_key_value = patrick_db_client::KeyValue {
        key: key_value.key,
        value: val_binary,
    };

    match pool.update(db_key_value).await {
        Ok(_) => {
            HttpResponse::Ok().json(Message {
                message: "successfully added key".to_string(),
            })
        }
        Err(e) => {
            log::error!("Error: {}", e);
            HttpResponse::BadGateway().json(Message {
                message: "database unavailable".to_string(),
            })
        }
    }
}

#[delete("/keys/{key_id}")]
async fn delete_key(pool: web::Data<PDBConnectionPool>, key_id: web::Path<String>) -> HttpResponse {
    let key_id = key_id.into_inner();

    match pool.delete(&key_id).await {
        Ok(_) => {
            HttpResponse::Ok().json(Message {
                message: "successfully deleted key".to_string(),
            })
        }
        Err(e) => {
            log::error!("Error: {}", e);
            HttpResponse::BadGateway().json(Message {
                message: "database unavailable".to_string(),
            })
        }
    }
}

#[get("/keys/{key_id}")]
async fn get_key(
    pool: web::Data<PDBConnectionPool>,
    key_id: web::Path<String>,
) -> HttpResponse {
    let key_id = key_id.into_inner();

    // use web::block to offload blocking Diesel code without blocking server thread
    match pool.get(&key_id).await {
        Ok(key_value_response) => {
            if let Some(key_value) = key_value_response.key_value {
                let value: i32 = deserialize(&key_value.value).unwrap();
                let key_value = KeyValue {
                    key: key_value.key,
                    value,
                };
                HttpResponse::Ok().json(key_value)
            } else {
                HttpResponse::NotFound().json(Message {
                    message: "key not found".to_string(),
                })
            }
        }
        Err(_e) => {
            HttpResponse::BadGateway().json(Message {
                message: "database unavailable".to_string(),
            })
        }
    }
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let config = patrick_db_client::Config{
        addresses: vec!["127.0.0.1:8080".to_string(), "127.0.0.1:8082".to_string()],
        min_connections: 1,
    };
    let pool = PDBConnectionPool::new(config).await?;

    log::info!("starting HTTP server at http://localhost:8000");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(middleware::Logger::default())
            .service(home)
            .service(get_key)
            .service(create_key)
            .service(delete_key)
    })
        .bind(format!("0.0.0.0:{}", "8000"))?
        .run()
        .await?;

    Ok(())
}
