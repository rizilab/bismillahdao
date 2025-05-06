#[derive(Debug)]
pub enum Error {
    Database(String),
    Json(String),
    Auth(String),
    Network(String),
    Encryption(String),
}

impl From<Error> for String {
    fn from(error: Error) -> Self {
        match error {
            Error::Database(msg) => msg,
            Error::Json(msg) => msg,
            Error::Auth(msg) => msg,
            Error::Network(msg) => msg,
            Error::Encryption(msg) => msg,
        }
    }
}
