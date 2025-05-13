pub mod config;
pub mod engine;
pub mod handler;
pub mod postgres;
pub mod redis;

use std::path::Path;

pub use anyhow::anyhow;
pub use anyhow::Context;
pub use anyhow::Error;
pub use anyhow::Result;
pub use engine::EngineError;
pub use handler::HandlerError;
pub use postgres::PostgresClientError;
pub use redis::RedisClientError;
use tracing::Event;
use tracing::Level;
use tracing::Metadata;
use tracing_appender::rolling::RollingFileAppender;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::layer::Context as LayerContext;
use tracing_subscriber::layer::Filter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

use crate::config::load_config;
use crate::config::LoggingConfig;

// Custom filter for exact debug level matching
struct DebugOnlyFilter;

impl<S> Filter<S> for DebugOnlyFilter
where
  S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
  fn enabled(
    &self,
    meta: &Metadata<'_>,
    _ctx: &LayerContext<'_, S>,
  ) -> bool {
    meta.level() == &Level::DEBUG
  }
}

// Custom filter for error and warn levels
struct ErrorWarnFilter;

impl<S> Filter<S> for ErrorWarnFilter
where
  S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
  fn enabled(
    &self,
    meta: &Metadata<'_>,
    _ctx: &LayerContext<'_, S>,
  ) -> bool {
    meta.level() == &Level::ERROR || meta.level() == &Level::WARN
  }
}

struct MuhafidhFormat {
  engine_name: String,
}

// Implement Clone for MuhafidhFormat
impl Clone for MuhafidhFormat {
  fn clone(&self) -> Self { Self { engine_name: self.engine_name.clone() } }
}

impl<S, N> FormatEvent<S, N> for MuhafidhFormat
where
  S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
  N: for<'writer> FormatFields<'writer> + 'static,
{
  fn format_event(
    &self,
    ctx: &FmtContext<'_, S, N>,
    mut writer: Writer<'_>,
    event: &Event<'_>,
  ) -> std::fmt::Result {
    // To get the message, we need to format the fields with a special visitor
    let metadata = event.metadata();
    let file = metadata.file().unwrap_or("unknown");
    let line = metadata.line().unwrap_or(0);

    if file == "unknown" && !cfg!(feature = "deep-trace") {
      return Ok(());
    }

    write!(writer, "{} {}::{}::{}::", metadata.level(), self.engine_name, file, line,)?;

    // Format the actual message
    ctx.field_format().format_fields(writer.by_ref(), event)?;

    writeln!(writer)
  }
}

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
  let info_appender = RollingFileAppender::new(Rotation::DAILY, base_logs_dir, format!("{}.log", engine_name));

  let debug_appender =
    RollingFileAppender::new(Rotation::DAILY, base_logs_dir.join("debug"), format!("{}.log", engine_name));

  let error_appender =
    RollingFileAppender::new(Rotation::DAILY, base_logs_dir.join("error"), format!("{}.log", engine_name));

  // Create non-blocking writers
  let (non_blocking_info, info_guard) = tracing_appender::non_blocking(info_appender);
  let (non_blocking_debug, debug_guard) = tracing_appender::non_blocking(debug_appender);
  let (non_blocking_error, error_guard) = tracing_appender::non_blocking(error_appender);

  // Store the guards in statics to keep them alive
  static mut INFO_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;
  static mut DEBUG_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;
  static mut ERROR_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;

  // Create filters with proper level directives
  // info and above (info, warn, error)
  let terminal_filter = EnvFilter::builder().parse_lossy("info");

  // info and above (info, warn, error)
  let info_filter = EnvFilter::builder().parse_lossy("info");

  // Create the custom format for all outputs
  let format = MuhafidhFormat { engine_name: engine_name.to_string() };

  // Set up the registry with all outputs
  let subscriber = tracing_subscriber::registry()
    // Terminal output with custom MuhafidhFormat - INFO and above
    .with(
      tracing_subscriber::fmt::Layer::default()
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .event_format(format.clone())
        .with_filter(terminal_filter),
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
        .with_filter(info_filter),
    )
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

  // Set the subscriber as the global default
  match tracing::subscriber::set_global_default(subscriber) {
    Ok(_) => {
      // Store the guards to keep the loggers alive
      unsafe {
        INFO_GUARD = Some(info_guard);
        DEBUG_GUARD = Some(debug_guard);
        ERROR_GUARD = Some(error_guard);
      }
      tracing::info!("{}_logging_started::info_logs::{}\\{}.log", engine_name, base_logs_dir.display(), engine_name);
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

// For consistent error handling with location info
#[macro_export]
macro_rules! err_with_loc {
  ($err:expr) => {
    anyhow::anyhow!($err).context(format!("at {}:{}", file!(), line!()))
  };
}
