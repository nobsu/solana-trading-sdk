use super::{boopfun_types::*, dex_traits::DexTrait, types::Create};
use crate::{
    common::{accounts::PUBKEY_WSOL, trading_endpoint::TradingEndpoint},
    dex::types::{PoolInfo, SwapInfo},
    instruction::builder::PriorityFee,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use std::sync::Arc;

pub struct Believe {
    pub endpoint: Arc<TradingEndpoint>,
}

#[async_trait::async_trait]
impl DexTrait for Believe {
    async fn initialize(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn initialized(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_trading_endpoint(&self) -> Arc<TradingEndpoint> {
        self.endpoint.clone()
    }

    fn use_wsol(&self) -> bool {
        false
    }

    async fn get_pool(&self, mint: &Pubkey) -> anyhow::Result<PoolInfo> {
        let bonding_curve_pda = Self::get_bonding_curve_pda(mint).unwrap();
        let account = self.endpoint.rpc.get_account(&bonding_curve_pda).await?;
        if account.data.is_empty() {
            return Err(anyhow::anyhow!("Bonding curve not found: {}", mint.to_string()));
        }

        let bonding_curve = bincode::deserialize::<BondingCurveAccount>(&account.data)?;

        Ok(PoolInfo {
            pool: bonding_curve_pda,
            creator: Some(bonding_curve.creator),
            creator_vault: None,
            token_reserves: bonding_curve.virtual_token_reserves,
            sol_reserves: bonding_curve.virtual_sol_reserves,
        })
    }

    async fn create(&self, _: Keypair, _: Create, _: Option<PriorityFee>, _: Option<u64>) -> anyhow::Result<Vec<Signature>> {
        Err(anyhow::anyhow!("Not supported"))
    }

    fn build_buy_instruction(&self, payer: &Keypair, mint: &Pubkey, _: Option<&Pubkey>, buy: SwapInfo) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let buy_info: BuyInfo = buy.into();
        let buffer = buy_info.to_buffer()?;
        let bonding_curve = Self::get_bonding_curve_pda(mint).ok_or(anyhow::anyhow!("Bonding curve not found: {}", mint.to_string()))?;
        let bonding_curve_vault = Self::get_bonding_curve_vault(mint).ok_or(anyhow::anyhow!("Bonding curve vault not found: {}", mint.to_string()))?;
        let bonding_curve_sol_vault =
            Self::get_bonding_curve_sol_vault(mint).ok_or(anyhow::anyhow!("Bonding curve sol vault not found: {}", mint.to_string()))?;
        let trading_fee_vault = Self::get_trading_fee_vault(mint).ok_or(anyhow::anyhow!("Trading fee vault not found: {}", mint.to_string()))?;

        Ok(Instruction::new_with_bytes(
            PUBKEY_BOOPFUN,
            &buffer,
            vec![
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new(bonding_curve, false),
                AccountMeta::new(trading_fee_vault, false),
                AccountMeta::new(bonding_curve_vault, false),
                AccountMeta::new(bonding_curve_sol_vault, false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(PUBKEY_BOOPFUN_CONFIG, false),
                AccountMeta::new_readonly(PUBKEY_BOOPFUN_VAULT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_WSOL, false),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            ],
        ))
    }

    fn build_sell_instruction(&self, payer: &Keypair, mint: &Pubkey, _: Option<&Pubkey>, sell: SwapInfo) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let sell_info: SellInfo = sell.into();
        let buffer = sell_info.to_buffer()?;
        let bonding_curve = Self::get_bonding_curve_pda(mint).ok_or(anyhow::anyhow!("Bonding curve not found: {}", mint.to_string()))?;
        let bonding_curve_vault = Self::get_bonding_curve_vault(mint).ok_or(anyhow::anyhow!("Bonding curve vault not found: {}", mint.to_string()))?;
        let bonding_curve_sol_vault =
            Self::get_bonding_curve_sol_vault(mint).ok_or(anyhow::anyhow!("Bonding curve sol vault not found: {}", mint.to_string()))?;
        let trading_fee_vault = Self::get_trading_fee_vault(mint).ok_or(anyhow::anyhow!("Trading fee vault not found: {}", mint.to_string()))?;

        Ok(Instruction::new_with_bytes(
            PUBKEY_BOOPFUN,
            &buffer,
            vec![
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new(bonding_curve, false),
                AccountMeta::new(trading_fee_vault, false),
                AccountMeta::new(bonding_curve_vault, false),
                AccountMeta::new(bonding_curve_sol_vault, false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(PUBKEY_BOOPFUN_CONFIG, false),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            ],
        ))
    }
}

impl Believe {
    pub fn new(endpoint: Arc<TradingEndpoint>) -> Self {
        Self { endpoint }
    }

    pub fn get_bonding_curve_pda(mint: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 2] = &[BONDING_CURVE_SEED, mint.as_ref()];
        let pda = Pubkey::try_find_program_address(seeds, &PUBKEY_BOOPFUN)?;
        Some(pda.0)
    }

    pub fn get_bonding_curve_vault(mint: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 2] = &[BONDING_CURVE_VAULT_SEED, mint.as_ref()];
        let pda = Pubkey::try_find_program_address(seeds, &PUBKEY_BOOPFUN)?;
        Some(pda.0)
    }

    pub fn get_bonding_curve_sol_vault(mint: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 2] = &[BONDING_CURVE_SOL_VAULT_SEED, mint.as_ref()];
        let pda = Pubkey::try_find_program_address(seeds, &PUBKEY_BOOPFUN)?;
        Some(pda.0)
    }

    pub fn get_trading_fee_vault(mint: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 2] = &[TRADING_FEE_VAULT_SEED, mint.as_ref()];
        let pda = Pubkey::try_find_program_address(seeds, &PUBKEY_BOOPFUN)?;
        Some(pda.0)
    }
}
