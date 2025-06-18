pub mod webhook;

pub enum DiscordHandlerLevel {
    Info {
        message: String,
    },
    Error {
        message: String,
    },
    Debug {
        message: String,
    },
}