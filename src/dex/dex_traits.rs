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
