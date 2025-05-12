use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cex {
  pub name:                  CexName,
  pub address:               solana_pubkey::Pubkey,
  pub total_token_relations: u64,
}

impl Cex {
  pub fn new(
    name: CexName,
    address: solana_pubkey::Pubkey,
    total_token_relations: u64,
  ) -> Self {
    Self { name, address, total_token_relations }
  }

  pub fn get_exchange_name(address: solana_pubkey::Pubkey) -> Option<CexName> {
    match address.to_string().as_str() {
      "FpwQQhQQoEaVu3WU2qZMfF1hx48YyfwsLoRgXG83E99Q" => Some(CexName::CoinbaseHW1),
      "GJRs4FwHtemZ5ZE9x3FNvJ8TMwitKTh21yxdRPqn7npE" => Some(CexName::CoinbaseHW2),
      "D89hHJT5Aqyx1trP6EnGY9jJUB3whgnq3aUvvCqedvzf" => Some(CexName::CoinbaseHW3),
      "DPqsobysNf5iA9w7zrQM8HLzCKZEDMkZsWbiidsAt1xo" => Some(CexName::CoinbaseHW4),
      "H8sMJSCQxfKiFTCfDR3DUMLPwcRbM61LGFJ8N4dK3WjS" => Some(CexName::Coinbase1),
      "2AQdpHJ2JpcEgPiATUXjQxA8QmafFegfQwSLWSprPicm" => Some(CexName::Coinbase2),
      "59L2oxymiQQ9Hvhh92nt8Y7nDYjsauFkdb3SybdnsG6h" => Some(CexName::Coinbase4),
      "9obNtb5GyUegcs3a1CbBkLuc5hEWynWfJC6gjz5uWQkE" => Some(CexName::Coinbase5),
      "CKy3KzEMSL1PQV6Wppggoqi2nGA7teE4L7JipEK89yqj" => Some(CexName::CoinbaseCW1),
      "G6zmnfSdG6QJaDWYwbGQ4dpCSUC4gvjfZxYQ4ZharV7C" => Some(CexName::CoinbaseCW2),
      "VTvk7sG6QQ28iK3NEKRRD9fvPzk5pKpJL2iwgVqMFcL" => Some(CexName::CoinbaseCW3),
      "85cPov8nuRCkJ88VNMcHaHZ26Ux85PbSrHW4jg7izW4h" => Some(CexName::CoinbaseCW4),
      "D6gCBB3CZEMNbX1PDr3GtZAMhnebEumcgJ2yv8Etv5hF" => Some(CexName::CoinbaseCW5),
      "3qP77PzrHxSrW1S8dH4Ss1dmpJDHpC6ATVgwy5FmXDEf" => Some(CexName::CoinbaseCW6),
      "146yGthSmnTPuCo6Zfbmr56YbAyWZ3rzAhRcT7tTF5ha" => Some(CexName::CoinbaseCW7),
      "GXTrXayxMJUujsRTxYjAbkdbNvs6u2KN89UpG8f6eMAg" => Some(CexName::CoinbaseCW8),
      "AzAvbCQsXurd2PbGLYcB61tyvE8kLDaZShE1S5Bp3WeS" => Some(CexName::CoinbaseCW9),
      "4pHKEisSmAr5CSump4dJnTJgG6eugmtieXcUxDBcQcG5" => Some(CexName::CoinbaseCW10),
      "BmGyWBMEcjJD7JQD1jRJ5vEt7XX2LyVvtxwtTGV4N1bp" => Some(CexName::CoinbaseCW11),
      "py5jDEUAynTufQHM7P6Tu9M8NUd8JYux7aMcLXcC51q" => Some(CexName::CoinbaseCW12),
      "is6MTRHEgyFLNTfYcuV4QBWLjrZBfmhVNYR6ccgr8KV" => Some(CexName::OKXHW1),
      "C68a6RCGLiPskbPYtAcsCjhG8tfTWYcoB4JjCrXFdqyo" => Some(CexName::OKXHW2),
      "5VCwKtCXgCJ6kit5FybXjvriW3xELsFDhYrPSqtJNmcD" => Some(CexName::OKX),
      "9un5wqE3q4oCjyrDkwsdD48KteCJitQX5978Vh7KKxHo" => Some(CexName::OKX2),
      "ASTyfSima4LLAdDgoFGkgqoKowG1LZFDr9fAQrg7iaJZ" => Some(CexName::MEXC1),
      "5PAhQiYdLBd6SVdjzBQDxUAEFyDdF5ExNPQfcscnPRj5" => Some(CexName::MEXC2),
      "FWznbcNXWQuHTawe9RxvQ2LdCENssh12dsznf4RiouN5" => Some(CexName::Kraken),
      "9cNE6KBg2Xmf34FPMMvzDF8yUHMrgLRzBV3vD7b1JnUS" => Some(CexName::KrakenCW),
      "F7RkX6Y1qTfBqoX5oHoZEgrG1Dpy55UZ3GfWwPbM58nQ" => Some(CexName::KrakenCW2),
      "3yFwqXBfZY4jBVUafQ1YEXw189y2dN3V5KQq9uzBDy1E" => Some(CexName::Binance8),
      "2ojv9BAiHUrvsm9gxDe7fJSzbNZSJcxZvf8dqmWGHG8S" => Some(CexName::Binance1),
      "5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9" => Some(CexName::Binance2),
      "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM" => Some(CexName::Binance3),
      "53unSgGWqEWANcPYRF35B2Bgf8BkszUtcccKiXwGGLyr" => Some(CexName::BinanceUSHW),
      "3gd3dqgtJ4jWfBfLYTX67DALFetjc5iS72sCgRhCkW2u" => Some(CexName::Binance10),
      "6QJzieMYfp7yr3EdrePaQoG3Ghxs2wM98xSLRu8Xh56U" => Some(CexName::Binance11),
      "GBrURzmtWujJRTA3Bkvo7ZgWuZYLMMwPCwre7BejJXnK" => Some(CexName::BinanceCW),
      "4S8C1yrRZmJYPzCqzEVjZYf6qCYWFoF7hWLRzssTCotX" => Some(CexName::BitgetCW),
      "A77HErqtfN1hLLpvZ9pCtu66FEtM8BveoaKbbMoZ4RiR" => Some(CexName::BitgetExchange),
      "u6PJ8DtQuPFnfmwHbGFULQ4u4EgjDiyYKjVEsynXq2w" => Some(CexName::Gateio1),
      "HiRpdAZifEsZGdzQ5Xo5wcnaH3D2Jj9SoNsUzcYNK78J" => Some(CexName::Gateio2),
      "AC5RDfQFmDS1deWZos921JfqscXdByf8BKHs5ACWjtW2" => Some(CexName::BybitHW),
      "42brAgAVNzMBP7aaktPvAmBSPEkehnFQejiZc53EpJFd" => Some(CexName::BybitCW),
      "FxteHmLwG9nk1eL4pjNve3Eub2goGkkz6g6TbvdmW46a" => Some(CexName::BitfinexHW),
      "FyJBKcfcEBzGN74uNxZ95GxnCxeuJJujQCELpPv14ZfN" => Some(CexName::BitfinexCW),
      "57vSaRTqN9iXaemgh4AoDsZ63mcaoshfMK8NP3Z5QNbs" => Some(CexName::KuCoin1),
      "BmFdpraQhkiDQE6SnfG5omcA1VwzqfXrwtNYBwWTymy6" => Some(CexName::KuCoin2),
      "HVh6wHNBAsG3pq1Bj5oCzRjoWKVogEDHwUHkRz3ekFgt" => Some(CexName::KuCoin3),
      "DBmae92YTQKLsNzXcPscxiwPqMcz9stQr2prB5ZCAHPd" => Some(CexName::KuCoinCW),
      "7Ci23i82UMa8RpfVbdMjTytiDi2VoZS8uLyHhZBV2Qy7" => Some(CexName::PoloniexHW),
      "8s9j5qUtuE9PGA5s7QeAXEh5oc2UGr71pmJXgyiZMHkt" => Some(CexName::LBank),
      "G9X7F4JzLzbSGMCndiBdWNi5YzZZakmtkdwq7xS3Q3FE" => Some(CexName::StakecomHotWallet),
      _ => None,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CexName {
  #[serde(rename = "coinbase_hw1")]
  CoinbaseHW1,
  #[serde(rename = "coinbase_hw2")]
  CoinbaseHW2,
  #[serde(rename = "coinbase_hw3")]
  CoinbaseHW3,
  #[serde(rename = "coinbase_hw4")]
  CoinbaseHW4,
  #[serde(rename = "coinbase_1")]
  Coinbase1,
  #[serde(rename = "coinbase_2")]
  Coinbase2,
  #[serde(rename = "coinbase_4")]
  Coinbase4,
  #[serde(rename = "coinbase_5")]
  Coinbase5,
  #[serde(rename = "coinbase_cw1")]
  CoinbaseCW1,
  #[serde(rename = "coinbase_cw2")]
  CoinbaseCW2,
  #[serde(rename = "coinbase_cw3")]
  CoinbaseCW3,
  #[serde(rename = "coinbase_cw4")]
  CoinbaseCW4,
  #[serde(rename = "coinbase_cw5")]
  CoinbaseCW5,
  #[serde(rename = "coinbase_cw6")]
  CoinbaseCW6,
  #[serde(rename = "coinbase_cw7")]
  CoinbaseCW7,
  #[serde(rename = "coinbase_cw8")]
  CoinbaseCW8,
  #[serde(rename = "coinbase_cw9")]
  CoinbaseCW9,
  #[serde(rename = "coinbase_cw10")]
  CoinbaseCW10,
  #[serde(rename = "coinbase_cw11")]
  CoinbaseCW11,
  #[serde(rename = "coinbase_cw12")]
  CoinbaseCW12,
  #[serde(rename = "okx_hw1")]
  OKXHW1,
  #[serde(rename = "okx_hw2")]
  OKXHW2,
  #[serde(rename = "okx")]
  OKX,
  #[serde(rename = "okx_2")]
  OKX2,
  #[serde(rename = "mexc_1")]
  MEXC1,
  #[serde(rename = "mexc_2")]
  MEXC2,
  #[serde(rename = "kraken")]
  Kraken,
  #[serde(rename = "kraken_cw")]
  KrakenCW,
  #[serde(rename = "kraken_cw2")]
  KrakenCW2,
  #[serde(rename = "binance_8")]
  Binance8,
  #[serde(rename = "binance_1")]
  Binance1,
  #[serde(rename = "binance_2")]
  Binance2,
  #[serde(rename = "binance_3")]
  Binance3,
  #[serde(rename = "binance_us_hw")]
  BinanceUSHW,
  #[serde(rename = "binance_10")]
  Binance10,
  #[serde(rename = "binance_11")]
  Binance11,
  #[serde(rename = "binance_cw")]
  BinanceCW,
  #[serde(rename = "bitget_cw")]
  BitgetCW,
  #[serde(rename = "bitget_exchange")]
  BitgetExchange,
  #[serde(rename = "gateio_1")]
  Gateio1,
  #[serde(rename = "gateio_2")]
  Gateio2,
  #[serde(rename = "bybit_hw")]
  BybitHW,
  #[serde(rename = "bybit_cw")]
  BybitCW,
  #[serde(rename = "bitfinex_hw")]
  BitfinexHW,
  #[serde(rename = "bitfinex_cw")]
  BitfinexCW,
  #[serde(rename = "kucoin_1")]
  KuCoin1,
  #[serde(rename = "kucoin_2")]
  KuCoin2,
  #[serde(rename = "kucoin_3")]
  KuCoin3,
  #[serde(rename = "kucoin_cw")]
  KuCoinCW,
  #[serde(rename = "poloniex_hw")]
  PoloniexHW,
  #[serde(rename = "lbank")]
  LBank,
  #[serde(rename = "stakecom_hot_wallet")]
  StakecomHotWallet,
}

impl std::fmt::Display for CexName {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    match self {
      CexName::CoinbaseHW1 => write!(f, "coinbase_hw1"),
      CexName::CoinbaseHW2 => write!(f, "coinbase_hw2"),
      CexName::CoinbaseHW3 => write!(f, "coinbase_hw3"),
      CexName::CoinbaseHW4 => write!(f, "coinbase_hw4"),
      CexName::Coinbase1 => write!(f, "coinbase_1"),
      CexName::Coinbase2 => write!(f, "coinbase_2"),
      CexName::Coinbase4 => write!(f, "coinbase_4"),
      CexName::Coinbase5 => write!(f, "coinbase_5"),
      CexName::CoinbaseCW1 => write!(f, "coinbase_cw1"),
      CexName::CoinbaseCW2 => write!(f, "coinbase_cw2"),
      CexName::CoinbaseCW3 => write!(f, "coinbase_cw3"),
      CexName::CoinbaseCW4 => write!(f, "coinbase_cw4"),
      CexName::CoinbaseCW5 => write!(f, "coinbase_cw5"),
      CexName::CoinbaseCW6 => write!(f, "coinbase_cw6"),
      CexName::CoinbaseCW7 => write!(f, "coinbase_cw7"),
      CexName::CoinbaseCW8 => write!(f, "coinbase_cw8"),
      CexName::CoinbaseCW9 => write!(f, "coinbase_cw9"),
      CexName::CoinbaseCW10 => write!(f, "coinbase_cw10"),
      CexName::CoinbaseCW11 => write!(f, "coinbase_cw11"),
      CexName::CoinbaseCW12 => write!(f, "coinbase_cw12"),
      CexName::OKXHW1 => write!(f, "okx_hw1"),
      CexName::OKXHW2 => write!(f, "okx_hw2"),
      CexName::OKX => write!(f, "okx"),
      CexName::OKX2 => write!(f, "okx_2"),
      CexName::MEXC1 => write!(f, "mexc_1"),
      CexName::MEXC2 => write!(f, "mexc_2"),
      CexName::Kraken => write!(f, "kraken"),
      CexName::KrakenCW => write!(f, "kraken_cw"),
      CexName::KrakenCW2 => write!(f, "kraken_cw2"),
      CexName::Binance8 => write!(f, "binance_8"),
      CexName::Binance1 => write!(f, "binance_1"),
      CexName::Binance2 => write!(f, "binance_2"),
      CexName::Binance3 => write!(f, "binance_3"),
      CexName::BinanceUSHW => write!(f, "binance_us_hw"),
      CexName::Binance10 => write!(f, "binance_10"),
      CexName::Binance11 => write!(f, "binance_11"),
      CexName::BinanceCW => write!(f, "binance_cw"),
      CexName::BitgetCW => write!(f, "bitget_cw"),
      CexName::BitgetExchange => write!(f, "bitget_exchange"),
      CexName::Gateio1 => write!(f, "gateio_1"),
      CexName::Gateio2 => write!(f, "gateio_2"),
      CexName::BybitHW => write!(f, "bybit_hw"),
      CexName::BybitCW => write!(f, "bybit_cw"),
      CexName::BitfinexHW => write!(f, "bitfinex_hw"),
      CexName::BitfinexCW => write!(f, "bitfinex_cw"),
      CexName::KuCoin1 => write!(f, "kucoin_1"),
      CexName::KuCoin2 => write!(f, "kucoin_2"),
      CexName::KuCoin3 => write!(f, "kucoin_3"),
      CexName::KuCoinCW => write!(f, "kucoin_cw"),
      CexName::PoloniexHW => write!(f, "poloniex_hw"),
      CexName::LBank => write!(f, "lbank"),
      CexName::StakecomHotWallet => write!(f, "stakecom_hot_wallet"),
    }
  }
}

impl From<CexName> for String {
  fn from(cex: CexName) -> Self {
    match cex {
      CexName::CoinbaseHW1 => "coinbase_hw1".to_string(),
      CexName::CoinbaseHW2 => "coinbase_hw2".to_string(),
      CexName::CoinbaseHW3 => "coinbase_hw3".to_string(),
      CexName::CoinbaseHW4 => "coinbase_hw4".to_string(),
      CexName::Coinbase1 => "coinbase_1".to_string(),
      CexName::Coinbase2 => "coinbase_2".to_string(),
      CexName::Coinbase4 => "coinbase_4".to_string(),
      CexName::Coinbase5 => "coinbase_5".to_string(),
      CexName::CoinbaseCW1 => "coinbase_cw1".to_string(),
      CexName::CoinbaseCW2 => "coinbase_cw2".to_string(),
      CexName::CoinbaseCW3 => "coinbase_cw3".to_string(),
      CexName::CoinbaseCW4 => "coinbase_cw4".to_string(),
      CexName::CoinbaseCW5 => "coinbase_cw5".to_string(),
      CexName::CoinbaseCW6 => "coinbase_cw6".to_string(),
      CexName::CoinbaseCW7 => "coinbase_cw7".to_string(),
      CexName::CoinbaseCW8 => "coinbase_cw8".to_string(),
      CexName::CoinbaseCW9 => "coinbase_cw9".to_string(),
      CexName::CoinbaseCW10 => "coinbase_cw10".to_string(),
      CexName::CoinbaseCW11 => "coinbase_cw11".to_string(),
      CexName::CoinbaseCW12 => "coinbase_cw12".to_string(),
      CexName::OKXHW1 => "okx_hw1".to_string(),
      CexName::OKXHW2 => "okx_hw2".to_string(),
      CexName::OKX => "okx".to_string(),
      CexName::OKX2 => "okx_2".to_string(),
      CexName::MEXC1 => "mexc_1".to_string(),
      CexName::MEXC2 => "mexc_2".to_string(),
      CexName::Kraken => "kraken".to_string(),
      CexName::KrakenCW => "kraken_cw".to_string(),
      CexName::KrakenCW2 => "kraken_cw2".to_string(),
      CexName::Binance8 => "binance_8".to_string(),
      CexName::Binance1 => "binance_1".to_string(),
      CexName::Binance2 => "binance_2".to_string(),
      CexName::Binance3 => "binance_3".to_string(),
      CexName::BinanceUSHW => "binance_us_hw".to_string(),
      CexName::Binance10 => "binance_10".to_string(),
      CexName::Binance11 => "binance_11".to_string(),
      CexName::BinanceCW => "binance_cw".to_string(),
      CexName::BitgetCW => "bitget_cw".to_string(),
      CexName::BitgetExchange => "bitget_exchange".to_string(),
      CexName::Gateio1 => "gateio_1".to_string(),
      CexName::Gateio2 => "gateio_2".to_string(),
      CexName::BybitHW => "bybit_hw".to_string(),
      CexName::BybitCW => "bybit_cw".to_string(),
      CexName::BitfinexHW => "bitfinex_hw".to_string(),
      CexName::BitfinexCW => "bitfinex_cw".to_string(),
      CexName::KuCoin1 => "kucoin_1".to_string(),
      CexName::KuCoin2 => "kucoin_2".to_string(),
      CexName::KuCoin3 => "kucoin_3".to_string(),
      CexName::KuCoinCW => "kucoin_cw".to_string(),
      CexName::PoloniexHW => "poloniex_hw".to_string(),
      CexName::LBank => "lbank".to_string(),
      CexName::StakecomHotWallet => "stakecom_hot_wallet".to_string(),
    }
  }
}

impl CexName {
  pub fn as_str(&self) -> &'static str {
    match self {
      CexName::CoinbaseHW1 => "coinbase_hw1",
      CexName::CoinbaseHW2 => "coinbase_hw2",
      CexName::CoinbaseHW3 => "coinbase_hw3",
      CexName::CoinbaseHW4 => "coinbase_hw4",
      CexName::Coinbase1 => "coinbase_1",
      CexName::Coinbase2 => "coinbase_2",
      CexName::Coinbase4 => "coinbase_4",
      CexName::Coinbase5 => "coinbase_5",
      CexName::CoinbaseCW1 => "coinbase_cw1",
      CexName::CoinbaseCW2 => "coinbase_cw2",
      CexName::CoinbaseCW3 => "coinbase_cw3",
      CexName::CoinbaseCW4 => "coinbase_cw4",
      CexName::CoinbaseCW5 => "coinbase_cw5",
      CexName::CoinbaseCW6 => "coinbase_cw6",
      CexName::CoinbaseCW7 => "coinbase_cw7",
      CexName::CoinbaseCW8 => "coinbase_cw8",
      CexName::CoinbaseCW9 => "coinbase_cw9",
      CexName::CoinbaseCW10 => "coinbase_cw10",
      CexName::CoinbaseCW11 => "coinbase_cw11",
      CexName::CoinbaseCW12 => "coinbase_cw12",
      CexName::OKXHW1 => "okx_hw1",
      CexName::OKXHW2 => "okx_hw2",
      CexName::OKX => "okx",
      CexName::OKX2 => "okx_2",
      CexName::MEXC1 => "mexc_1",
      CexName::MEXC2 => "mexc_2",
      CexName::Kraken => "kraken",
      CexName::KrakenCW => "kraken_cw",
      CexName::KrakenCW2 => "kraken_cw2",
      CexName::Binance8 => "binance_8",
      CexName::Binance1 => "binance_1",
      CexName::Binance2 => "binance_2",
      CexName::Binance3 => "binance_3",
      CexName::BinanceUSHW => "binance_us_hw",
      CexName::Binance10 => "binance_10",
      CexName::Binance11 => "binance_11",
      CexName::BinanceCW => "binance_cw",
      CexName::BitgetCW => "bitget_cw",
      CexName::BitgetExchange => "bitget_exchange",
      CexName::Gateio1 => "gateio_1",
      CexName::Gateio2 => "gateio_2",
      CexName::BybitHW => "bybit_hw",
      CexName::BybitCW => "bybit_cw",
      CexName::BitfinexHW => "bitfinex_hw",
      CexName::BitfinexCW => "bitfinex_cw",
      CexName::KuCoin1 => "kucoin_1",
      CexName::KuCoin2 => "kucoin_2",
      CexName::KuCoin3 => "kucoin_3",
      CexName::KuCoinCW => "kucoin_cw",
      CexName::PoloniexHW => "poloniex_hw",
      CexName::LBank => "lbank",
      CexName::StakecomHotWallet => "stakecom_hot_wallet",
    }
  }
}
