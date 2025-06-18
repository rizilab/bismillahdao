pub mod task;

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::debug;
use tracing::info;

use crate::Result;
use crate::config::Config;
use crate::config::load_config;
use crate::handler::shutdown::ShutdownSignal;
use crate::handler::token::creator::CreatorHandlerOperator;
use crate::config::RpcConfig;
use crate::tracing::setup_tracing;
use crate::storage::StorageEngine;
use crate::storage::make_storage_engine;

#[derive(Clone)]
pub struct Baseer {
    pub config: Config,
    pub db: Arc<StorageEngine>,
    pub creator_handler: Arc<CreatorHandlerOperator>,
    pub rpc_config: Arc<RpcConfig>,
}

impl Baseer {
    pub async fn run() -> Result<()> {
        info!("Starting Baseer (بصير): The Analyzer");
        let shutdown_signal = ShutdownSignal::new();

        debug!("loading_configuration");
        let config = load_config("Config.toml").await?;
        if let Err(e) = setup_tracing(config.clone(), "baseer", shutdown_signal.clone()).await {
            error!("failed_to_setup_tracing: {}", e);
        }

        debug!("initializing_db_engine");
        let db_engine = Arc::new(make_storage_engine("baseer", &config).await?);
        debug!("db_engine::created");


        let cancellation_token = CancellationToken::new();

        // Use RpcConfig directly and initialize runtime state
        let mut rpc_config = config.rpc.clone();
        rpc_config.init_runtime_state().await;
        let rpc_config = Arc::new(rpc_config);
        let (operator_sender, operator_receiver) = mpsc::channel(1000);
        let creator_handler = Arc::new(CreatorHandlerOperator::new(
            db_engine.clone(),
            shutdown_signal.clone(),
            operator_receiver,
            operator_sender,
            rpc_config.clone(),
        ));

        let baseer = Baseer {
            config,
            db: db_engine.clone(),
            creator_handler,
            rpc_config,
        };

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);
        let (sender, receiver) = mpsc::channel(1000);

        let token_creator_analyzer_handle =
            baseer.spawn_new_token_creator_analyzer(receiver, cancellation_token.clone());

        let token_subscriber_handle = baseer.spawn_new_token_subscriber(shutdown_signal.clone(), sender);

        let account_recovery_handle = baseer.spawn_account_recovery(cancellation_token.clone());

        let account_queue_reporting_handle = baseer.spawn_account_queue_reporting();

        tokio::select! {
            _ = token_creator_analyzer_handle => {},
            _ = token_subscriber_handle => {},
            _ = account_recovery_handle => {},
            _ = account_queue_reporting_handle => {},
            _ = tokio::signal::ctrl_c() => {
                let _ = shutdown_tx.send(()).await;
            },
            _ = shutdown_rx.recv() => {
                info!("main_loop::received_ctrl_c::shutting_down_all_components");
                shutdown_signal.shutdown();
                cancellation_token.cancel();
            }
        }

        Ok(())
    }
}
