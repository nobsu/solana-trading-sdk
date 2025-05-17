use base64::{engine::general_purpose, Engine};
use solana_sdk::transaction::VersionedTransaction;
use std::time::Duration;
use tokio::time::timeout;

pub const SWQOS_RPC_TIMEOUT: std::time::Duration = Duration::from_secs(1);

#[async_trait::async_trait]
pub trait SWQoSClientTrait {
    fn new_swqos_client() -> reqwest::Client {
        reqwest::Client::builder().timeout(SWQOS_RPC_TIMEOUT).build().unwrap()
    }
    async fn send_swqos_transaction(&self, url: &str, transaction: &VersionedTransaction) -> anyhow::Result<()>;
    async fn send_swqos_transactions(&self, url: &str, transaction: &[VersionedTransaction]) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl SWQoSClientTrait for reqwest::Client {
    async fn send_swqos_transaction(&self, url: &str, transaction: &VersionedTransaction) -> anyhow::Result<()> {
        let tx_bytes = bincode::serialize(transaction)?;
        let tx_base64 = general_purpose::STANDARD.encode(tx_bytes);
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "sendTransaction",
            "params": [
                tx_base64,
                { "encoding": "base64" }
            ],
            "id": 1,
        });

        let response = timeout(SWQOS_RPC_TIMEOUT, self.post(url).json(&body).send()).await??;

        if response.status() != 200 {
            return Err(anyhow::anyhow!(
                "send_swqos_transaction error: {} {} {}",
                url,
                response.status(),
                serde_json::to_string(&body).unwrap(),
            ));
        }

        let response_json = timeout(SWQOS_RPC_TIMEOUT, response.json::<serde_json::Value>()).await??;
        if let Some(result) = response_json.get("result") {
            println!("Transaction sent successfully: {} {}", url, result);
        } else if let Some(error) = response_json.get("error") {
            eprintln!("Transaction sent error: {} {}", url, error);
        }

        Ok(())
    }

    async fn send_swqos_transactions(&self, url: &str, transactions: &[VersionedTransaction]) -> anyhow::Result<()> {
        let txs_base64 = transactions
            .iter()
            .map(|tx| {
                let tx_bytes = bincode::serialize(tx)?;
                Ok(general_purpose::STANDARD.encode(tx_bytes))
            })
            .collect::<anyhow::Result<Vec<String>>>()?;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "sendTransaction",
            "params": [
                txs_base64,
                { "encoding": "base64" }
            ],
            "id": 1,
        });

        let response = timeout(SWQOS_RPC_TIMEOUT, self.post(url).json(&body).send()).await??;

        if response.status() != 200 {
            return Err(anyhow::anyhow!(
                "send_swqos_transaction error: {} {} {}",
                url,
                response.status(),
                serde_json::to_string(&body).unwrap(),
            ));
        }

        Ok(())
    }
}
