// this is where we use pgrouting as graph database
// pub async fn make_graph_client(engine_name: &str) -> Result<Arc<RedisClient>, RedisClientError> {
//     let graph_url = std::env::var("GRAPH_URL").unwrap_or_else(|_| "http://127.0.0.1:8000/".to_string());
//     let client = RedisClient::new(&graph_url).await?;
//     info!("{}::graph_client::connection_established: {}", engine_name, graph_url);
//     Ok(Arc::new(client))
// }
