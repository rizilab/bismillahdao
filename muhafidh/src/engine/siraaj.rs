use crate::Result;
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Siraaj {
    pub config: Config,
    //   pub db:     Arc<StorageEngine>,
    //   pub token_handler: Arc<TokenHandlerMetadataOperator>,
}

impl Siraaj {
    pub async fn run() -> Result<()> {
        // info!("Starting Baseer (بصير): The Analyzer");

        // setup_tracing("baseer");
        // info!("setup_tracing");

        // let config = load_config("Config.toml")?;
        // info!("config loaded");

        // let db_engine = Arc::new(make_storage_engine("baseer", &config).await?);
        // info!("db_engine::created");

        // let shutdown_signal = ShutdownSignal::new();

        // db_engine.postgres.db.health_check().await?;
        // db_engine.postgres.db.initialize().await?;

        // let token_handler = Arc::new(TokenHandlerMetadataOperator::new(
        //     db_engine.clone(), shutdown_signal.clone()));

        // let raqib = Raqib { config, db: db_engine, token_handler: token_handler };

        // let shutdown_signal = ShutdownSignal::new();

        // let mut pipeline = make_pumpfun_subscriber_pipeline(raqib)?;

        // let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);

        // tokio::select! {
        //     result = pipeline.run() => {
        //         shutdown_signal.shutdown();
        //         let _ = shutdown_tx.send(()).await;
        //         result.map_err(|e| {
        //             error!("pipeline_error: {}", e);
        //             err_with_loc!(EngineError::EngineError(e))
        //           })?
        //     },
        //     _ = tokio::signal::ctrl_c() => {
        //         info!("termination_signal::graceful_shutdown");

        //         shutdown_signal.shutdown();
        //         let _ = shutdown_tx.send(()).await;
        //     },
        //     _ = shutdown_rx.recv() => {
        //         info!("shutdown_signal::other_component");

        //         shutdown_signal.shutdown();
        //     }
        // }

        // info!("all_component_shutdown");
        // tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // info!("raqib::shutdown");

        Ok(())
    }
}
