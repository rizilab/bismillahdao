use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    #[serde(rename = "pumpfun")]
    PumpFun,
    #[serde(rename = "bonk")]
    Bonk,
    #[serde(rename = "unknown")]
    Unknown,
}

impl Platform {
    pub fn from_str(s: &str) -> Self {
        match s {
            "pumpfun" => Platform::PumpFun,
            "bonk" => Platform::Bonk,
            _ => Platform::Unknown,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Platform::PumpFun => String::from("pumpfun"),
            Platform::Bonk => String::from("bonk"),
            Platform::Unknown => String::from("unknown"),
        }
    }
}
