// pub async fn make_message_queue(engine_name: &str) -> Result<Arc<RedisClient>, RedisClientError> {
//     let message_queue_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
//     let client = RedisClient::new(&message_queue_url).await?;
//     info!("{}::message_queue::connection_established: {}", engine_name, message_queue_url);
//     Ok(Arc::new(client))
// }

// pub async fn make_kv_store() -> Result<Arc<RedisKVStore>> {
//     match is_local() {
//         true => {
//             let kv_store = RedisKVStore::new("redis://localhost:6379").await?;
//             Ok(Arc::new(kv_store))
//         }
//         false => {
//             let kv_store =
//                 RedisKVStore::new(must_get_env("REDIS_URL").as_str()).await?;
//             Ok(Arc::new(kv_store))
//         }
//     }
// }
