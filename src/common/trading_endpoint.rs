use crate::{
    instruction::builder::{build_transaction, PriorityFee, TipFee},
    swqos::SWQoSTrait,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    signature::{Keypair, Signature},
};
use std::sync::Arc;

pub struct TradingEndpoint {
    pub rpc: Arc<RpcClient>,
    pub swqos: Vec<Arc<dyn SWQoSTrait>>,
}

impl TradingEndpoint {
    pub fn new(rpc: Arc<RpcClient>, swqos: Vec<Arc<dyn SWQoSTrait>>) -> Self {
        Self { rpc, swqos }
    }

    pub async fn send_transactions(
        &self,
        payer: &Keypair,
        instructions: Vec<Instruction>,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let mut tasks = vec![];
        for swqos in &self.swqos {
            let tip = if let Some(tip_account) = swqos.get_tip_account() {
                if let Some(tip) = tip {
                    Some(TipFee {
                        tip_account,
                        tip_lamports: tip,
                    })
                } else {
                    // If no tip is provided, skip this Tip-SWQoS
                    continue;
                }
            } else {
                None
            };

            let tx = build_transaction(payer, instructions.clone(), blockhash, fee, tip)?;
            tasks.push(swqos.send_transaction(tx));
        }

        let signatures = futures::future::try_join_all(tasks).await?;

        Ok(signatures)
    }
}
