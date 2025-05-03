use std::str::FromStr;

use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;

/// Helper trait for checking if an address belongs to a CEX
pub trait Cex {
  /// Get the exchange name (e.g., "Coinbase", "Binance")
  fn check_exchange_name<'a>(address: &'a str) -> &'a str;
}

impl Cex for Pubkey {
  fn check_exchange_name<'a>(address: &'a str) -> &'a str {
    match address {
      "FpwQQhQQoEaVu3WU2qZMfF1hx48YyfwsLoRgXG83E99Q" => "Coinbase HW 1",
      "GJRs4FwHtemZ5ZE9x3FNvJ8TMwitKTh21yxdRPqn7npE" => "Coinbase HW 2",
      "D89hHJT5Aqyx1trP6EnGY9jJUB3whgnq3aUvvCqedvzf" => "Coinbase HW 3",
      "DPqsobysNf5iA9w7zrQM8HLzCKZEDMkZsWbiidsAt1xo" => "Coinbase HW 4",
      "H8sMJSCQxfKiFTCfDR3DUMLPwcRbM61LGFJ8N4dK3WjS" => "Coinbase 1",
      "2AQdpHJ2JpcEgPiATUXjQxA8QmafFegfQwSLWSprPicm" => "Coinbase 2",
      "59L2oxymiQQ9Hvhh92nt8Y7nDYjsauFkdb3SybdnsG6h" => "Coinbase 4",
      "9obNtb5GyUegcs3a1CbBkLuc5hEWynWfJC6gjz5uWQkE" => "Coinbase 5",
      "CKy3KzEMSL1PQV6Wppggoqi2nGA7teE4L7JipEK89yqj" => "Coinbase CW 1",
      "G6zmnfSdG6QJaDWYwbGQ4dpCSUC4gvjfZxYQ4ZharV7C" => "Coinbase CW 2",
      "VTvk7sG6QQ28iK3NEKRRD9fvPzk5pKpJL2iwgVqMFcL" => "Coinbase CW 3",
      "85cPov8nuRCkJ88VNMcHaHZ26Ux85PbSrHW4jg7izW4h" => "Coinbase CW 4",
      "D6gCBB3CZEMNbX1PDr3GtZAMhnebEumcgJ2yv8Etv5hF" => "Coinbase CW 5",
      "3qP77PzrHxSrW1S8dH4Ss1dmpJDHpC6ATVgwy5FmXDEf" => "Coinbase CW 6",
      "146yGthSmnTPuCo6Zfbmr56YbAyWZ3rzAhRcT7tTF5ha" => "Coinbase CW 7",
      "GXTrXayxMJUujsRTxYjAbkdbNvs6u2KN89UpG8f6eMAg" => "Coinbase CW 8",
      "AzAvbCQsXurd2PbGLYcB61tyvE8kLDaZShE1S5Bp3WeS" => "Coinbase CW 9",
      "4pHKEisSmAr5CSump4dJnTJgG6eugmtieXcUxDBcQcG5" => "Coinbase CW 10",
      "BmGyWBMEcjJD7JQD1jRJ5vEt7XX2LyVvtxwtTGV4N1bp" => "Coinbase CW 11",
      "py5jDEUAynTufQHM7P6Tu9M8NUd8JYux7aMcLXcC51q" => "Coinbase CW 12",
      "is6MTRHEgyFLNTfYcuV4QBWLjrZBfmhVNYR6ccgr8KV" => "OKX HW 1",
      "C68a6RCGLiPskbPYtAcsCjhG8tfTWYcoB4JjCrXFdqyo" => "OKX HW 2",
      "5VCwKtCXgCJ6kit5FybXjvriW3xELsFDhYrPSqtJNmcD" => "OKX ",
      "9un5wqE3q4oCjyrDkwsdD48KteCJitQX5978Vh7KKxHo" => "OKX 2",
      "ASTyfSima4LLAdDgoFGkgqoKowG1LZFDr9fAQrg7iaJZ" => "MEXC 1",
      "5PAhQiYdLBd6SVdjzBQDxUAEFyDdF5ExNPQfcscnPRj5" => "MEXC 2",
      "FWznbcNXWQuHTawe9RxvQ2LdCENssh12dsznf4RiouN5" => "Kraken",
      "9cNE6KBg2Xmf34FPMMvzDF8yUHMrgLRzBV3vD7b1JnUS" => "Kraken CW",
      "F7RkX6Y1qTfBqoX5oHoZEgrG1Dpy55UZ3GfWwPbM58nQ" => "Kraken CW 2",
      "3yFwqXBfZY4jBVUafQ1YEXw189y2dN3V5KQq9uzBDy1E" => "Binance 8",
      "2ojv9BAiHUrvsm9gxDe7fJSzbNZSJcxZvf8dqmWGHG8S" => "Binance 1",
      "5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9" => "Binance 2",
      "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM" => "Binance 3",
      "53unSgGWqEWANcPYRF35B2Bgf8BkszUtcccKiXwGGLyr" => "Binance US HW",
      "3gd3dqgtJ4jWfBfLYTX67DALFetjc5iS72sCgRhCkW2u" => "Binance 10",
      "6QJzieMYfp7yr3EdrePaQoG3Ghxs2wM98xSLRu8Xh56U" => "Binance 11",
      "GBrURzmtWujJRTA3Bkvo7ZgWuZYLMMwPCwre7BejJXnK" => "Binance CW",
      "4S8C1yrRZmJYPzCqzEVjZYf6qCYWFoF7hWLRzssTCotX" => "Bitget CW",
      "A77HErqtfN1hLLpvZ9pCtu66FEtM8BveoaKbbMoZ4RiR" => "Bitget Exchange",
      "u6PJ8DtQuPFnfmwHbGFULQ4u4EgjDiyYKjVEsynXq2w" => "Gate.io 1",
      "HiRpdAZifEsZGdzQ5Xo5wcnaH3D2Jj9SoNsUzcYNK78J" => "Gate.io 2",
      "AC5RDfQFmDS1deWZos921JfqscXdByf8BKHs5ACWjtW2" => "Bybit HW",
      "42brAgAVNzMBP7aaktPvAmBSPEkehnFQejiZc53EpJFd" => "Bybit CW",
      "FxteHmLwG9nk1eL4pjNve3Eub2goGkkz6g6TbvdmW46a" => "Bitfinex HW",
      "FyJBKcfcEBzGN74uNxZ95GxnCxeuJJujQCELpPv14ZfN" => "Bitfinex CW",
      "57vSaRTqN9iXaemgh4AoDsZ63mcaoshfMK8NP3Z5QNbs" => "KuCoin 1",
      "BmFdpraQhkiDQE6SnfG5omcA1VwzqfXrwtNYBwWTymy6" => "KuCoin 2",
      "HVh6wHNBAsG3pq1Bj5oCzRjoWKVogEDHwUHkRz3ekFgt" => "KuCoin 3",
      "DBmae92YTQKLsNzXcPscxiwPqMcz9stQr2prB5ZCAHPd" => "KuCoin CW",
      "7Ci23i82UMa8RpfVbdMjTytiDi2VoZS8uLyHhZBV2Qy7" => "Poloniex HW",
      "8s9j5qUtuE9PGA5s7QeAXEh5oc2UGr71pmJXgyiZMHkt" => "LBank",
      "G9X7F4JzLzbSGMCndiBdWNi5YzZZakmtkdwq7xS3Q3FE" => "Stake.com Hot Wallet",
      _ => address,
    }
  }
}
