use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::VersionedTransaction};
use std::sync::Arc;

pub mod jito;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SWQoSType {
    Default,
    Jito,
}

#[async_trait::async_trait]
pub trait SWQoSTrait: Send + Sync {
    async fn send_transaction(&self, transaction: VersionedTransaction) -> anyhow::Result<Signature>;
    async fn send_transactions(&self, transactions: Vec<VersionedTransaction>) -> anyhow::Result<Vec<Signature>>;
    fn get_tip_account(&self) -> Option<Pubkey>;
    fn get_type(&self) -> SWQoSType;
}

impl From<SWQoSType> for Arc<dyn SWQoSTrait> {
    fn from(value: SWQoSType) -> Self {
        match value {
            SWQoSType::Default => Arc::new(jito::JitoSwqos::new()),
            SWQoSType::Jito => Arc::new(jito::JitoSwqos::new()),
        }
    }
}
