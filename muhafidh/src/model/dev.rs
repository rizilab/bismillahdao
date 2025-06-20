use serde::Deserialize;
use serde::Serialize;

use super::cex::CexName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dev {
    pub dev_name: DevName,
    pub cex_name: CexName,
    pub address: solana_pubkey::Pubkey,
}

impl Dev {
    pub fn new(
        dev_name: DevName,
        cex_name: CexName,
        address: solana_pubkey::Pubkey,
    ) -> Self {
        Self {
            dev_name,
            cex_name,
            address,
        }
    }

    pub fn get_cex_from_dev_address(address: solana_pubkey::Pubkey) -> Option<CexName> {
        match address.to_string().as_str() {
            "GZVSEAajExLJEvACHHQcujBw7nJq98GWUEZtood9LM9b" => Some(CexName::BybitHW),
            "xXpRSpAe1ajq4tJP78tS3X1AqNwJVQ4Vvb1Swg4hHQh" => Some(CexName::Coinbase2),    
            _ => None,
        }
    }

    /// Get complete developer info (including associated CEX) from address
    pub fn get_dev_info(address: solana_pubkey::Pubkey) -> Option<Dev> {
        match address.to_string().as_str() {
            "GZVSEAajExLJEvACHHQcujBw7nJq98GWUEZtood9LM9b" => Some(Dev::new(
                DevName::MotionDev,
                CexName::BybitHW,
                address,
            )),
            "xXpRSpAe1ajq4tJP78tS3X1AqNwJVQ4Vvb1Swg4hHQh" => Some(Dev::new(
                DevName::CrpSource,
                CexName::Coinbase2,
                address,
            )),
            _ => None,
        }
    }

    /// Get developer name from address (legacy method)
    pub fn get_dev_name(address: solana_pubkey::Pubkey) -> Option<DevName> {
        match address.to_string().as_str() {
            "GZVSEAajExLJEvACHHQcujBw7nJq98GWUEZtood9LM9b" => Some(DevName::MotionDev),
            "xXpRSpAe1ajq4tJP78tS3X1AqNwJVQ4Vvb1Swg4hHQh" => Some(DevName::CrpSource),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub enum DevName {
    #[serde(rename = "motion_dev")]
    MotionDev,
    #[serde(rename = "crp_source")]
    CrpSource,
    #[serde(rename = "unknown_dev")]
    #[default]
    UnknownDev,
}

impl std::fmt::Display for DevName {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            DevName::MotionDev => write!(f, "motion_dev"),
            DevName::CrpSource => write!(f, "crp_source"),
            _ => write!(f, "unknown_dev"),
        }
    }
}

impl From<DevName> for String {
    fn from(dev: DevName) -> Self {
        match dev {
            DevName::MotionDev => "motion_dev".to_string(),
            DevName::CrpSource => "crp_source".to_string(),
            _ => "unknown_dev".to_string(),
        }
    }
}

impl DevName {
    pub fn as_str(&self) -> &'static str {
        match self {
            DevName::MotionDev => "motion_dev",
            DevName::CrpSource => "crp_source",
            _ => "unknown_dev",
        }
    }
}
