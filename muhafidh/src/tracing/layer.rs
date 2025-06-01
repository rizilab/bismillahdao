use std::sync::Arc;
use tokio::sync::mpsc;
use tracing_subscriber::Layer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::layer::Context;
use tracing::Event;
use tracing::Subscriber;
use tracing::field::{Field, Visit};

use crate::config::discord::DiscordConfig;
use crate::handler::discord::webhook::DiscordWebhookHandlerOperator;
use crate::handler::shutdown::ShutdownSignal;

// Visitor to extract the message from the event's fields
struct MessageVisitor {
    message: Option<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }

    // It's a good idea to handle other types if your logs might use them for messages
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }

    // Add other `record_` methods if necessary (e.g., record_i64, record_bool)
}

pub struct DiscordLayer {
    pub config: Arc<DiscordConfig>, // Assuming you might need it for engine_name or other settings
    pub discord_webhook_handler: DiscordWebhookHandlerOperator, // Sender to the DiscordWebhookHandler
    pub engine_name: String, // To mimic MuhafidhFormat
}

impl DiscordLayer {
    pub fn new(config: DiscordConfig, shutdown: ShutdownSignal, engine_name: String) -> Self {
        let config = Arc::new(config);
        let (sender, receiver) = mpsc::channel(1000);
        let discord_webhook_handler = DiscordWebhookHandlerOperator::new(shutdown, receiver, sender, config.clone());
        
        Self { config, discord_webhook_handler, engine_name }
    }
}

impl<S> Layer<S> for DiscordLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) { // ctx is not used here
        let metadata = event.metadata();
        let level = metadata.level();
        let target = metadata.target();
        let file = metadata.file().unwrap_or("unknown");
        let line = metadata.line().unwrap_or(0);

        // Skip if file is unknown and deep-trace is not enabled (similar to MuhafidhFormat)
        if file == "unknown" && !cfg!(feature = "deep-trace") {
            return;
        }

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");

        let mut visitor = MessageVisitor { message: None };
        event.record(&mut visitor);
        let event_message = visitor.message.unwrap_or_else(|| "No message field in event".to_string());

        let formatted_message = format!(
            "{} {}::{}::{}::{}:: {}",
            level,
            timestamp,
            self.engine_name,
            file,
            line,
            event_message
        );
        
        if target.starts_with("muhafidh") { 
            if let Err(e) = self.discord_webhook_handler.send_message(target, level, formatted_message) {
                eprintln!("Failed to send log to Discord handler: {}", e);
            }
        }
    }
}
