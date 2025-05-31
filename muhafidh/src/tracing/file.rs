use tracing_appender::rolling::RollingFileAppender;
use tracing_appender::rolling::Rotation;

use crate::config::LoggingConfig;
use crate::config::load_config;
use super::filter::DebugOnlyFilter;
use super::filter::ErrorWarnFilter;
use super::filter::ErrorOnlyFilter;
#[cfg(feature = "dev")]
use super::filter::InfoOnlyFilter;
use super::format::MuhafidhFormat;
use std::path::Path;
use tracing_subscriber::Layer;
use tracing_subscriber::prelude::*;

pub fn setup_tracing(engine_name: &str) {
    // Attempt to load config, falling back to defaults if it fails
    let config_result = load_config("Config.toml");

    // Default logging config
    let logging_config = config_result
        .map(|config| config.logging)
        .unwrap_or_else(|_| LoggingConfig::default());

    // Base logs directory
    let base_logs_dir = Path::new(logging_config.directory.as_deref().unwrap_or(".logs"));

    // Create logs directories if they don't exist
    let logs_dirs = [base_logs_dir, &base_logs_dir.join("debug"), &base_logs_dir.join("error")];

    for dir in &logs_dirs {
        if !dir.exists() {
            std::fs::create_dir_all(dir).expect(&format!("Failed to create logs directory: {}", dir.display()));
        }
    }

    // Create file appenders for each log level
    #[cfg(feature = "dev")]
    let info_appender = RollingFileAppender::new(Rotation::DAILY, base_logs_dir, format!("{}.log", engine_name));

    let debug_appender =
        RollingFileAppender::new(Rotation::DAILY, base_logs_dir.join("debug"), format!("{}.log", engine_name));

    let error_appender =
        RollingFileAppender::new(Rotation::DAILY, base_logs_dir.join("error"), format!("{}.log", engine_name));

    // Create non-blocking writers
    #[cfg(feature = "dev")]
    let (non_blocking_info, info_guard) = tracing_appender::non_blocking(info_appender);
    let (non_blocking_debug, debug_guard) = tracing_appender::non_blocking(debug_appender);
    let (non_blocking_error, error_guard) = tracing_appender::non_blocking(error_appender);

    // Store the guards in statics to keep them alive
    #[cfg(feature = "dev")]
    static mut INFO_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;
    static mut DEBUG_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;
    static mut ERROR_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;

    // Create the custom format for all outputs
    let format = MuhafidhFormat {
        engine_name: engine_name.to_string(),
    };

    // Set up the registry with all outputs
    let subscriber = tracing_subscriber::registry()
        // DEBUG log file - debug only using custom filter
        .with(
            tracing_subscriber::fmt::Layer::default()
                .with_ansi(false)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .event_format(format.clone())
                .with_writer(non_blocking_debug)
                .with_filter(DebugOnlyFilter),
        )
        // ERROR log file - warn and error only
        .with(
            tracing_subscriber::fmt::Layer::default()
                .with_ansi(false)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .event_format(format.clone())
                .with_writer(non_blocking_error)
                .with_filter(ErrorWarnFilter),
        );

    #[cfg(feature = "prod")]
    let subscriber = subscriber
        // Terminal output with custom MuhafidhFormat - Error
        .with(
            tracing_subscriber::fmt::Layer::default()
                .with_ansi(true)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .event_format(format.clone())
                .with_filter(ErrorOnlyFilter),
        );

    #[cfg(feature = "dev")]
    let subscriber = subscriber
        // Terminal output with custom MuhafidhFormat - INFO and above
        .with(
            tracing_subscriber::fmt::Layer::default()
                .with_ansi(true)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .event_format(format.clone())
                .with_filter(InfoOnlyFilter),
        )
        // INFO log file - info and above
        .with(
            tracing_subscriber::fmt::Layer::default()
                .with_ansi(false)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .event_format(format.clone())
                .with_writer(non_blocking_info)
                .with_filter(InfoOnlyFilter),
        );

    // Set the subscriber as the global default
    match tracing::subscriber::set_global_default(subscriber) {
        Ok(_) => {
            // Store the guards to keep the loggers alive
            #[cfg(feature = "dev")]
            unsafe {
                INFO_GUARD = Some(info_guard);
            }
            unsafe {
                DEBUG_GUARD = Some(debug_guard);
                ERROR_GUARD = Some(error_guard);
            }
            tracing::info!(
                "{}_logging_started::info_logs::{}\\{}.log",
                engine_name,
                base_logs_dir.display(),
                engine_name
            );
            tracing::info!(
                "{}_logging_started::debug_logs::{}\\debug\\{}.log",
                engine_name,
                base_logs_dir.display(),
                engine_name
            );
            tracing::info!(
                "{}_logging_started::error_logs::{}\\error\\{}.log",
                engine_name,
                base_logs_dir.display(),
                engine_name
            );
        },
        Err(e) => {
            eprintln!("Error setting up logging: {}", e);
        },
    }
}