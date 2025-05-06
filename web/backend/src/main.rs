mod domain;
mod application;
mod infrastructure;

use std::sync::Arc;
use actix_web::{web, App, HttpServer, middleware};
use actix_cors::Cors;
use futures::future;
use sqlx::postgres::PgPoolOptions;
use anyhow::Result;
use tracing::{info, error};
use tracing_subscriber::FmtSubscriber;

use infrastructure::config::AppConfig;
use infrastructure::config::run_migrations;
use infrastructure::driven::database::PostgresUserRepository;
use application::services::UserService;

#[actix_web::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting application...");
    
    // Load configuration
    let config = match AppConfig::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };
    
    info!("Configuration loaded successfully");
    
    // Set up database connection pool
    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database.url).await {
        Ok(pool) => {
            info!("Database connection established");
            pool
        },
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };
    
    // Run database migrations
    if let Err(e) = run_migrations(&pool).await {
        error!("Failed to run database migrations: {}", e);
        std::process::exit(1);
    }
    
    // Create shared components
    let db_pool = Arc::new(pool);
    let user_repo = Arc::new(PostgresUserRepository::new(db_pool.clone()));
    let user_service = Arc::new(UserService::new(
        user_repo.clone(),
        config.auth.jwt_secret.clone(),
    ));
    
    // Static file paths
    let dist_path = if cfg!(feature = "dev") {
        "../ui/dist"
    } else {
        "/usr/local/share/r4gmi-auth/dist"
    };
    
    // Set up Auth server (port 8080)
    let auth_server_config = config.auth_server.clone();
    let auth_service = user_service.clone();
    let auth_app_state = web::Data::new(
        infrastructure::driving::web::api::AppState {
            user_service: auth_service,
        }
    );
    
    let auth_server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();
            
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .app_data(auth_app_state.clone())
            .service(
                infrastructure::driving::web::auth::auth_static_files(dist_path)
            )
            .default_service(web::get().to(infrastructure::driving::web::auth::auth_index))
    })
    .bind((auth_server_config.host, auth_server_config.port))?
    .run();
    
    // Set up API server (port 8081)
    let api_server_config = config.api_server.clone();
    let api_service = user_service.clone();
    let api_app_state = web::Data::new(
        infrastructure::driving::web::api::AppState {
            user_service: api_service,
        }
    );
    
    let api_server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();
            
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .app_data(api_app_state.clone())
            .service(infrastructure::driving::web::api::user_routes())
    })
    .bind((api_server_config.host, api_server_config.port))?
    .run();
    
    // Set up Landing server (port 8082)
    let landing_server_config = config.landing_server.clone();
    
    let landing_server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .service(
                infrastructure::driving::web::landing::landing_static_files(dist_path)
            )
            .default_service(web::get().to(infrastructure::driving::web::landing::landing_index))
    })
    .bind((landing_server_config.host, landing_server_config.port))?
    .run();
    
    // Start all servers
    info!("Auth server listening on {}:{}", auth_server_config.host, auth_server_config.port);
    info!("API server listening on {}:{}", api_server_config.host, api_server_config.port);
    info!("Landing server listening on {}:{}", landing_server_config.host, landing_server_config.port);
    
    // Run all servers concurrently
    future::try_join3(auth_server, api_server, landing_server).await?;
    
    info!("Application shutting down");
    Ok(())
}