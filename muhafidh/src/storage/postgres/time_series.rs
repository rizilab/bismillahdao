// this file is for time series data type

// pub async fn make_timeseries_client(engine_name: &str) -> Result<Arc<RedisClient>, RedisClientError> {
//     let timeseries_url = std::env::var("TIMESERIES_URL").unwrap_or_else(|_| "http://127.0.0.1:8000/".to_string());
//     let client = RedisClient::new(&timeseries_url).await?;
//     info!("{}::timeseries_client::connection_established: {}", engine_name, timeseries_url);
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
