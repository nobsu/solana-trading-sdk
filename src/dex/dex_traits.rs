use super::types::{Create, TokenAmountType};
use crate::instruction::builder::PriorityFee;
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
};
use std::any::Any;

#[async_trait::async_trait]
pub trait DexTrait: Send + Sync + Any {
    async fn initialize(&self) -> anyhow::Result<()>;
    fn initialized(&self) -> anyhow::Result<()>;
    async fn create(&self, payer: Keypair, create: Create, fee: Option<PriorityFee>, tip: Option<u64>) -> anyhow::Result<Vec<Signature>>;
    async fn buy(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        sol_amount: u64,
        slippage_basis_points: u64,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>>;
    async fn buy_immediately(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        pool: Option<&Pubkey>,
        creator_vault: Option<&Pubkey>,
        sol_amount: u64,
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
        pool: Option<&Pubkey>,
        creator_vault: Option<&Pubkey>,
        token_amount: u64,
        sol_amount: u64,
        close_mint_ata: bool,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>>;
    async fn batch_buy(
        &self,
        mint: &Pubkey,
        slippage_basis_points: u64,
        fee: PriorityFee,
        tip: u64,
        items: Vec<BatchBuyParam>,
    ) -> anyhow::Result<Vec<Signature>>;
    async fn batch_sell(
        &self,
        mint: &Pubkey,
        slippage_basis_points: u64,
        fee: PriorityFee,
        tip: u64,
        items: Vec<BatchSellParam>,
    ) -> anyhow::Result<Vec<Signature>>;
}

pub struct BatchBuyParam {
    pub payer: Keypair,
    pub sol_amount: u64,
}

pub struct BatchSellParam {
    pub payer: Keypair,
    pub token_amount: u64,
    pub close_mint_ata: bool,
}
