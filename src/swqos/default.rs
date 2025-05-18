use super::{swqos_rpc::SWQoSClientTrait, SWQoSTrait};
use rand::seq::IndexedRandom;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::sync::Arc;

#[derive(Clone)]
pub struct DefaultSWQoSClient {
    pub name: String,
    pub rpc_client: Arc<RpcClient>,
    pub tip_accounts: Vec<Pubkey>,
    pub swqos_endpoint: String,
    pub swqos_header: Option<(String, String)>,
    pub swqos_client: Arc<reqwest::Client>,
}

#[async_trait::async_trait]
impl SWQoSTrait for DefaultSWQoSClient {
    async fn send_transaction(&self, transaction: VersionedTransaction) -> anyhow::Result<()> {
        self.swqos_client
            .send_swqos_transaction(&self.swqos_endpoint, self.swqos_header.clone(), &transaction)
            .await
    }

    async fn send_transactions(&self, transactions: Vec<VersionedTransaction>) -> anyhow::Result<()> {
        self.swqos_client
            .send_swqos_transactions(&self.swqos_endpoint, self.swqos_header.clone(), &transactions)
            .await
    }

    fn get_tip_account(&self) -> Option<Pubkey> {
        Some(*self.tip_accounts.choose(&mut rand::rng())?)
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

impl DefaultSWQoSClient {
    pub fn new(name: &str, rpc_client: Arc<RpcClient>, endpoint: String, header: Option<(String, String)>, tip_accounts: Vec<Pubkey>) -> Self {
        let swqos_client = reqwest::Client::new_swqos_client();

        Self {
            name: name.to_string(),
            rpc_client,
            tip_accounts,
            swqos_endpoint: endpoint,
            swqos_header: header,
            swqos_client: Arc::new(swqos_client),
        }
    }
}
