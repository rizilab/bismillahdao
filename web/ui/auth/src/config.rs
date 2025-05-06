use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub redirect_to: String,
}