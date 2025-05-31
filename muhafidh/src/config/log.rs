use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    // Directory where logs will be stored
    pub directory: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            directory: Some(".logs".to_string()),
        }
    }
}
