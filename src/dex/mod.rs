use crate::{common::trading_endpoint::TradingEndpoint, instruction::builder::PriorityFee};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
};
use spl_associated_token_account::get_associated_token_address;
use std::{any::Any, sync::Arc};

pub mod amm_calc;
pub mod pumpfun;
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

impl TokenAmountType {
    pub async fn to_amount(&self, rpc: Arc<RpcClient>, payer: &Pubkey, mint: &Pubkey) -> anyhow::Result<u64> {
        match self {
            TokenAmountType::Percent(percent) => {
                let ata = get_associated_token_address(payer, mint);
                let balance = rpc.get_token_account_balance(&ata).await?;
                let balance_u64 = balance.amount.parse::<u64>()?;
                Ok((balance_u64 * percent) / 100)
            }
            TokenAmountType::Amount(amount) => Ok(*amount),
        }
    }
}

#[async_trait::async_trait]
pub trait DexTrait: Send + Sync + Any {
    async fn initialize(&self) -> anyhow::Result<()>;
    fn initialized(&self) -> anyhow::Result<()>;
    fn create(&self, amount: u64) -> anyhow::Result<u64>;
    async fn buy(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        sol_lamsports: u64,
        slippage_basis_points: u64,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>>;
    async fn buy_immediately(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        pool: &Pubkey,
        creator: Option<&Pubkey>,
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
        slippage_basis_points: u64,
        close_mint_ata: bool,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>>;
    async fn sell_immediately(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        pool: &Pubkey,
        creator: Option<&Pubkey>,
        token_amount: u64,
        sol_lamports: u64,
        close_mint_ata: bool,
        blockhash: Hash,
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
