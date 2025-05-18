use base64::{engine::general_purpose, Engine};
use solana_sdk::transaction::VersionedTransaction;
use std::time::Duration;
use tokio::time::timeout;

pub const SWQOS_RPC_TIMEOUT: std::time::Duration = Duration::from_secs(1);

pub trait FormatBase64VersionedTransaction {
    fn to_base64_string(&self) -> String;
}

impl FormatBase64VersionedTransaction for VersionedTransaction {
    fn to_base64_string(&self) -> String {
        let tx_bytes = bincode::serialize(self).unwrap();
        general_purpose::STANDARD.encode(tx_bytes)
    }
}

#[async_trait::async_trait]
pub trait SWQoSClientTrait {
    fn new_swqos_client() -> reqwest::Client {
        reqwest::Client::builder().timeout(SWQOS_RPC_TIMEOUT).build().unwrap()
    }
    async fn send_swqos_transaction(&self, url: &str, header: Option<(String, String)>, transaction: &VersionedTransaction) -> anyhow::Result<()>;
    async fn send_swqos_transactions(&self, url: &str, header: Option<(String, String)>, transaction: &[VersionedTransaction]) -> anyhow::Result<()>;
    async fn json_post(&self, url: &str, header: Option<(String, String)>, body: serde_json::Value) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl SWQoSClientTrait for reqwest::Client {
    async fn send_swqos_transaction(&self, url: &str, header: Option<(String, String)>, transaction: &VersionedTransaction) -> anyhow::Result<()> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "sendTransaction",
            "params": [
                transaction.to_base64_string(),
                { "encoding": "base64" }
            ],
            "id": 1,
        });

        self.json_post(url, header, body).await?;
        println!("Transaction sent successfully: {} {}", url, transaction.signatures[0]);
        Ok(())
    }

    async fn send_swqos_transactions(&self, url: &str, header: Option<(String, String)>, transactions: &[VersionedTransaction]) -> anyhow::Result<()> {
        let txs_base64 = transactions.iter().map(|tx| tx.to_base64_string()).collect::<Vec<String>>();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "sendTransaction",
            "params": [
                txs_base64,
                { "encoding": "base64" }
            ],
            "id": 1,
        });

        self.json_post(url, header, body).await?;
        println!(
            "Transaction batch sent successfully: {} {}",
            url,
            transactions.iter().map(|tx| tx.signatures[0].to_string()).collect::<Vec<_>>().join(", ")
        );
        Ok(())
    }

    async fn json_post(&self, url: &str, header: Option<(String, String)>, body: serde_json::Value) -> anyhow::Result<()> {
        let response = if let Some((key, value)) = header {
            timeout(SWQOS_RPC_TIMEOUT, self.post(url).header(key, value).json(&body).send()).await??
        } else {
            timeout(SWQOS_RPC_TIMEOUT, self.post(url).json(&body).send()).await??
        };
        let http_status = response.status();
        let response_json = timeout(SWQOS_RPC_TIMEOUT, response.json::<serde_json::Value>()).await??;

        if http_status != 200 || response_json.get("error").is_some() {
            return Err(anyhow::anyhow!(
                "json_post error: {} {} {}",
                url,
                http_status,
                serde_json::to_string(&response_json).unwrap(),
            ));
        }

        Ok(())
    }
}
