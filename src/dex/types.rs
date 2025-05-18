use super::{dex_traits::DexTrait, pumpswap};
use crate::common::trading_endpoint::TradingEndpoint;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use std::sync::Arc;

pub struct Buy {
    pub token_amount: u64,
    pub sol_amount: u64,
}

pub struct Sell {
    pub token_amount: u64,
    pub sol_amount: u64,
}

pub struct Create {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub buy_sol_amount: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DexType {
    Pumpfun,
    PumpSwap,
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
