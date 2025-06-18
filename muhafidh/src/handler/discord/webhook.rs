use std::sync::Arc;

use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::Level;
use tracing::error;

use super::DiscordHandlerLevel;
use crate::Result;
use crate::config::DiscordChannel;
use crate::config::DiscordChannelConfig;
use crate::config::DiscordConfig;
use crate::err_with_loc;
use crate::error::handler::HandlerError;
use crate::handler::shutdown::ShutdownSignal;

pub struct DiscordWebhookHandler {
    receiver: mpsc::Receiver<DiscordHandlerLevel>,
    discord_config: Arc<DiscordConfig>,
    http_client: Client, // Add HTTP client
}

impl DiscordWebhookHandler {
    pub fn new(
        receiver: mpsc::Receiver<DiscordHandlerLevel>,
        discord_config: Arc<DiscordConfig>,
    ) -> Self {
        Self {
            receiver,
            discord_config,
            http_client: Client::new(), // Initialize client
        }
    }

    async fn send_to_discord(
        &self,
        channel_config: &DiscordChannelConfig,
        message: &str,
    ) -> Result<()> {
        if message.trim().is_empty() {
            return Err(err_with_loc!("Empty message")); // Don't send empty messages
        }

        // Discord messages have a 2000 character limit. Split if longer.
        // This is a simple split, more sophisticated handling might be needed.
        let chunks = message
            .as_bytes()
            .chunks(1900) // A bit less than 2000 to leave room for formatting/metadata
            .map(|chunk| std::str::from_utf8(chunk).unwrap_or("Error: Non-UTF8 chunk"))
            .collect::<Vec<&str>>();

        for chunk in chunks {
            let payload = json!({
                "content": format!("{}", chunk) // Use ansi code block for better formatting
            });

            let webhook_url =
                format!("https://discord.com/api/webhooks/{}/{}", channel_config.channel_id, channel_config.key);

            match self.http_client.post(&webhook_url).json(&payload).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "<failed to read response text>".to_string());
                        return Err(err_with_loc!(HandlerError::SendDiscordError(format!(
                            "Failed to send log to Discord channel {:?}: {} - {}",
                            channel_config.channel_name, status, text
                        ))));
                    }
                },
                Err(e) => {
                    return Err(err_with_loc!(HandlerError::SendDiscordError(format!(
                        "Error sending log to Discord channel {:?}: {}",
                        channel_config.channel_name, e
                    ))));
                },
            }
        }

        Ok(())
    }
}

async fn run_discord_webhook_handler(mut discord_webhook_handler: DiscordWebhookHandler) {
    loop {
        tokio::select! {
            Some(msg) = discord_webhook_handler.receiver.recv() => {
                match msg {
                    DiscordHandlerLevel::Info { message } => {
                        let channel = discord_webhook_handler.discord_config.get_channel_by_name(&DiscordChannel::Info);
                        if let Some(channel) = channel {
                            if let Err(e) = discord_webhook_handler.send_to_discord(channel, &message).await {
                                eprintln!("Error sending log to Discord channel {:?}: {}", channel.channel_name, e);
                            }
                        }
                    },
                    DiscordHandlerLevel::Error { message } => {
                        let channel = discord_webhook_handler.discord_config.get_channel_by_name(&DiscordChannel::Error);
                        if let Some(channel) = channel {
                            if let Err(e) = discord_webhook_handler.send_to_discord(channel, &message).await {
                                eprintln!("Error sending log to Discord channel {:?}: {}", channel.channel_name, e);
                            }
                        }
                    },
                    DiscordHandlerLevel::Debug { message } => {
                        let channel = discord_webhook_handler.discord_config.get_channel_by_name(&DiscordChannel::Debug);
                        if let Some(channel) = channel {
                            if let Err(e) = discord_webhook_handler.send_to_discord(channel, &message).await {
                                eprintln!("Error sending log to Discord channel {:?}: {}", channel.channel_name, e);
                            }
                        }
                    },
                }
            },
            else => {
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscordWebhookHandlerOperator {
    pub shutdown: ShutdownSignal,
    pub sender: mpsc::Sender<DiscordHandlerLevel>,
    pub discord_config: Arc<DiscordConfig>,
}

impl DiscordWebhookHandlerOperator {
    pub fn new(
        shutdown: ShutdownSignal,
        receiver: mpsc::Receiver<DiscordHandlerLevel>,
        sender: mpsc::Sender<DiscordHandlerLevel>,
        discord_config: Arc<DiscordConfig>,
    ) -> Self {
        let discord_webhook = DiscordWebhookHandler::new(receiver, discord_config.clone());

        tokio::spawn(run_discord_webhook_handler(discord_webhook));

        Self {
            shutdown,
            sender,
            discord_config,
        }
    }

    pub fn send_message(
        &self,
        target: &str,
        level: &Level,
        message: String,
    ) -> Result<()> {
        match level {
            &Level::INFO => {
                if target.starts_with("muhafidh::handler::token::creator") {
                    if let Err(e) = self.sender.try_send(DiscordHandlerLevel::Info {
                        message,
                    }) {
                        error!("Failed to send log to Discord: {}", e);
                    }
                }
                Ok(())
            },
            // &Level::ERROR => {
            //     if let Err(e) = self.sender.try_send(DiscordHandlerLevel::Error { message }) {
            //         error!("Failed to send log to Discord: {}", e);
            //     }
            //     Ok(())
            // }
            _ => Ok(()),
        }
    }
}
