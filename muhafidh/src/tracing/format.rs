use tracing::Event;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::registry::LookupSpan;

pub struct MuhafidhFormat {
    pub engine_name: String,
}

// Implement Clone for MuhafidhFormat
impl Clone for MuhafidhFormat {
    fn clone(&self) -> Self {
        Self {
            engine_name: self.engine_name.clone(),
        }
    }
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

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");

        write!(
            writer,
            "{} {}::{}::{}::{}::",
            metadata.level(),
            timestamp,
            self.engine_name,
            file,
            line
        )?;

        // Format the actual message
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}