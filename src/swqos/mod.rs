use nextblock::NEXTBLOCK_TIP_ACCOUNTS;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::sync::Arc;

pub mod default;
pub mod jito;
pub mod nextblock;
pub mod swqos_rpc;
pub mod temporal;
pub mod zeroslot;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SWQoSType {
    Default(String, String),
    Jito(String, String),
    NextBlock(String, String),
    Temporal(String, String),
    Zeroslot(String, String),
}

#[async_trait::async_trait]
pub trait SWQoSTrait: Send + Sync {
    async fn send_transaction(&self, transaction: VersionedTransaction) -> anyhow::Result<()>;
    async fn send_transactions(&self, transactions: Vec<VersionedTransaction>) -> anyhow::Result<()>;
    fn get_tip_account(&self) -> Option<Pubkey>;
    fn get_name(&self) -> &str;
}

impl SWQoSType {
    pub fn instantiate(&self, rpc_client: Arc<RpcClient>) -> Arc<dyn SWQoSTrait> {
        match self {
            SWQoSType::Default(endpoint, _) => Arc::new(default::DefaultSWQoSClient::new(
                "default",
                rpc_client,
                endpoint.to_string(),
                vec![],
            )),
            SWQoSType::NextBlock(endpoint, auth_token) => Arc::new(default::DefaultSWQoSClient::new(
                "nextblock",
                rpc_client,
                format!("{}/api-key={}", endpoint, auth_token),
                NEXTBLOCK_TIP_ACCOUNTS.into(),
            )),
            SWQoSType::Zeroslot(endpoint, auth_token) => Arc::new(default::DefaultSWQoSClient::new(
                "0slot",
                rpc_client,
                format!("{}/api-key={}", endpoint, auth_token),
                NEXTBLOCK_TIP_ACCOUNTS.into(),
            )),
            SWQoSType::Jito(endpoint, auth_token) => Arc::new(default::DefaultSWQoSClient::new(
                "jito",
                rpc_client,
                format!("{}/api-key={}", endpoint, auth_token),
                NEXTBLOCK_TIP_ACCOUNTS.into(),
            )),
            SWQoSType::Temporal(endpoint, auth_token) => Arc::new(default::DefaultSWQoSClient::new(
                "temporal",
                rpc_client,
                format!("{}/api-key={}", endpoint, auth_token),
                NEXTBLOCK_TIP_ACCOUNTS.into(),
            )),
        }
    }
}
