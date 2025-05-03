pub mod config;
pub mod postgres;

pub use anyhow::anyhow;
pub use anyhow::Context;
pub use anyhow::Error;
pub use anyhow::Result;
pub use postgres::PostgresClientError;
use tracing::Event;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::registry::LookupSpan;

struct MuhafidhFormat {
  engine_name: String,
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

    write!(
      writer,
      "{} {}::{}::{}::",
      metadata.level(),
      self.engine_name,
      metadata.file().unwrap_or("unknown"),
      metadata.line().unwrap_or(0),
    )?;

    // Format the actual message
    ctx.field_format().format_fields(writer.by_ref(), event)?;

    writeln!(writer)
  }
}

pub fn setup_tracing(engine_name: &str) {
  // Create an EnvFilter that reads from RUST_LOG with INFO as default
  let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,"));

  tracing_subscriber::fmt()
    .with_env_filter(env_filter)
    .with_ansi(true)
    .with_file(true)
    .with_line_number(true)
    .with_target(false)
    .event_format(MuhafidhFormat { engine_name: engine_name.to_string() })
    .init();
}

// For consistent error handling with location info
#[macro_export]
macro_rules! err_with_loc {
  ($err:expr) => {
    anyhow::anyhow!($err).context(format!("at {}:{}", file!(), line!()))
  };
}
