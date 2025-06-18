use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscordChannel {
    Debug,
    Error,
    Info
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannelConfig {
    pub channel_name: DiscordChannel,
    pub channel_id: String,
    pub key: String
}

impl DiscordChannelConfig {
    pub fn get_webhook_url(&self) -> String {
        format!("https://discord.com/api/webhooks/{}/{}", self.channel_id, self.key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub fallback_timeout_ms: u64,
    pub channels: Vec<DiscordChannelConfig>
}

impl DiscordConfig {
    pub fn get_channel_by_name(&self, name: &DiscordChannel) -> Option<&DiscordChannelConfig> {
        self.channels.iter().find(|c| c.channel_name == *name)
    }
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            fallback_timeout_ms: 1000,
            channels: vec![],
        }
    }
}