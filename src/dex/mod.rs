use crate::{common::trading_endpoint::TradingEndpoint, instruction::builder::PriorityFee};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    hash::Hash,
};
use std::sync::Arc;

pub mod amm_calc;
pub mod pumpswap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DexType {
    Pumpfun,
    PumpSwap,
}

pub enum TokenAmountType {
    Percent(u64),
    Amount(u64),
}

#[async_trait::async_trait]
pub trait DexTrait: Send + Sync {
    async fn initialize(&self) -> anyhow::Result<()>;
    fn initialized(&self) -> anyhow::Result<()>;
    fn create(&self, amount: u64) -> anyhow::Result<u64>;
    async fn buy(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        sol_lamsports: u64,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>>;
    async fn buy_immediately(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        pool: &Pubkey,
        sol_lamports: u64,
        token_amount: u64,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>>;
    async fn sell(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        token_amount: TokenAmountType,
        close_mint_ata: bool,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>>;
}

impl DexType {
    pub fn all() -> Vec<DexType> {
        vec![DexType::Pumpfun, DexType::PumpSwap]
    }

    pub fn instantiate(&self, endpoint: Arc<TradingEndpoint>) -> Arc<dyn DexTrait> {
        match self {
            DexType::Pumpfun => Arc::new(pumpswap::PumpSwap::new(endpoint)),
            DexType::PumpSwap => Arc::new(pumpswap::PumpSwap::new(endpoint)),
        }
    }
}
