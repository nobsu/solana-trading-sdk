use super::{
    amm_calc::{amm_buy_get_token_out, amm_sell_get_sol_out, calculate_with_slippage_buy, calculate_with_slippage_sell},
    DexTrait, TokenAmountType,
};
use crate::{
    common::trading_endpoint::TradingEndpoint,
    instruction::builder::{build_sell_instructions, build_wsol_buy_instructions, PriorityFee},
};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use std::sync::Arc;

pub const PUBKEY_PUMPFUN: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUBKEY_GLOBAL_ACCOUNT: Pubkey = pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
pub const PUBKEY_EVENT_AUTHORITY: Pubkey = pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const PUBKEY_FEE_RECIPIENT: Pubkey = pubkey!("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV");

pub const GLOBAL_SEED: &[u8] = b"global";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint-authority";
pub const BONDING_CURVE_SEED: &[u8] = b"bonding-curve";
pub const CREATOR_VAULT_SEED: &[u8] = b"creator-vault";
pub const METADATA_SEED: &[u8] = b"metadata";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalAccount {
    pub discriminator: u64,
    pub initialized: bool,
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondingCurveAccount {
    pub discriminator: u64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
    pub creator: Pubkey,
}

pub struct Pumpfun {
    pub endpoint: Arc<TradingEndpoint>,
    pub global_account: OnceCell<Arc<GlobalAccount>>,
}

#[async_trait::async_trait]
impl DexTrait for Pumpfun {
    async fn initialize(&self) -> anyhow::Result<()> {
        let account = self.endpoint.rpc.get_account(&PUBKEY_GLOBAL_ACCOUNT).await?;
        let global_account = bincode::deserialize::<GlobalAccount>(&account.data)?;
        let global_account = Arc::new(global_account);

        self.global_account.set(global_account).unwrap();
        Ok(())
    }

    fn initialized(&self) -> anyhow::Result<()> {
        if self.global_account.get().is_none() {
            return Err(anyhow::anyhow!("PumpSwap not initialized"));
        }
        Ok(())
    }

    fn create(&self, _: u64) -> anyhow::Result<u64> {
        Err(anyhow::anyhow!("Not supported"))
    }

    async fn buy(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        sol_lamports: u64,
        slippage_basis_points: u64,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let sol_lamports_with_slippage = calculate_with_slippage_buy(sol_lamports, slippage_basis_points);
        let (bonding_curve, bonding_curve_account) = self.get_bonding_curve(&mint).await?;
        let blockhash = self.endpoint.rpc.get_latest_blockhash().await?;
        let buy_token_amount = amm_buy_get_token_out(
            bonding_curve_account.virtual_sol_reserves,
            bonding_curve_account.virtual_token_reserves,
            sol_lamports,
        );

        self.buy_immediately(
            payer,
            mint,
            &bonding_curve,
            Some(&bonding_curve_account.creator),
            sol_lamports_with_slippage,
            buy_token_amount,
            blockhash,
            fee,
            tip,
        )
        .await
    }

    async fn buy_immediately(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        pool: &Pubkey,
        creator: Option<&Pubkey>,
        sol_lamports: u64,
        buy_token_amount: u64,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let creator = creator.ok_or(anyhow::anyhow!("Creator not provided"))?;
        let creator_vault = Self::get_creator_vault_pda(creator).unwrap();
        let instruction = self.build_buy_instruction(payer, mint, &pool, &creator_vault, buy_token_amount, sol_lamports)?;
        let instructions = build_wsol_buy_instructions(payer, mint, sol_lamports, instruction)?;
        let signatures = self.endpoint.send_transactions(payer, instructions, blockhash, fee, tip).await?;

        Ok(signatures)
    }

    async fn sell(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        token_amount: TokenAmountType,
        slippage_basis_points: u64,
        close_mint_ata: bool,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let (pool, pool_account) = self.get_bonding_curve(&mint).await?;
        let blockhash = self.endpoint.rpc.get_latest_blockhash().await?;
        let token_amount = token_amount.to_amount(self.endpoint.rpc.clone(), &payer.pubkey(), mint).await?;
        let sol_lamports = amm_sell_get_sol_out(pool_account.virtual_sol_reserves, pool_account.virtual_token_reserves, token_amount);
        let sol_lamports_with_slippage = calculate_with_slippage_sell(sol_lamports, slippage_basis_points);

        self.sell_immediately(
            payer,
            mint,
            &pool,
            Some(&pool_account.creator),
            token_amount,
            sol_lamports_with_slippage,
            close_mint_ata,
            blockhash,
            fee,
            tip,
        )
        .await
    }

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
    ) -> anyhow::Result<Vec<Signature>> {
        let creator = creator.ok_or(anyhow::anyhow!("Creator not provided"))?;
        let creator_vault = Self::get_creator_vault_pda(creator).unwrap();
        let instruction = self.build_sell_instruction(payer, mint, &pool, &creator_vault, token_amount, sol_lamports)?;
        let instructions = build_sell_instructions(payer, mint, instruction, close_mint_ata)?;
        let signatures = self.endpoint.send_transactions(payer, instructions, blockhash, fee, tip).await?;

        Ok(signatures)
    }
}

impl Pumpfun {
    pub fn new(endpoint: Arc<TradingEndpoint>) -> Self {
        Self {
            endpoint,
            global_account: OnceCell::new(),
        }
    }

    pub fn get_bonding_curve_pda(mint: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 2] = &[BONDING_CURVE_SEED, mint.as_ref()];
        let program_id: &Pubkey = &PUBKEY_PUMPFUN;
        let pda = Pubkey::try_find_program_address(seeds, program_id)?;
        Some(pda.0)
    }

    pub fn get_creator_vault_pda(creator: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 2] = &[CREATOR_VAULT_SEED, creator.as_ref()];
        let program_id: &Pubkey = &PUBKEY_PUMPFUN;
        let pda = Pubkey::try_find_program_address(seeds, program_id)?;
        Some(pda.0)
    }

    pub async fn get_bonding_curve(&self, mint: &Pubkey) -> anyhow::Result<(Pubkey, BondingCurveAccount)> {
        let bonding_curve_pda = Self::get_bonding_curve_pda(mint).unwrap();
        let account = self.endpoint.rpc.get_account(&bonding_curve_pda).await?;
        if account.data.is_empty() {
            return Err(anyhow::anyhow!("Bonding curve not found"));
        }

        let bonding_curve = bincode::deserialize::<BondingCurveAccount>(&account.data)?;

        Ok((bonding_curve_pda, bonding_curve))
    }

    fn build_buy_instruction(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        bonding_curve: &Pubkey,
        creator_vault: &Pubkey,
        buy_token_amount: u64,
        max_sol_cost: u64,
    ) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(&[102, 6, 61, 18, 1, 218, 235, 234]); // discriminator
        data.extend_from_slice(&buy_token_amount.to_le_bytes());
        data.extend_from_slice(&max_sol_cost.to_le_bytes());

        Ok(Instruction::new_with_bytes(
            PUBKEY_PUMPFUN,
            &data,
            vec![
                AccountMeta::new_readonly(PUBKEY_GLOBAL_ACCOUNT, false),
                AccountMeta::new(PUBKEY_FEE_RECIPIENT, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new(*bonding_curve, false),
                AccountMeta::new(get_associated_token_address(bonding_curve, mint), false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new(*creator_vault, false),
                AccountMeta::new_readonly(PUBKEY_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_PUMPFUN, false),
            ],
        ))
    }

    pub fn build_sell_instruction(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        bonding_curve: &Pubkey,
        creator_vault: &Pubkey,
        token_amount: u64,
        min_sol_out: u64,
    ) -> anyhow::Result<Instruction> {
        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(&[51, 230, 133, 164, 1, 127, 131, 173]); // discriminator
        data.extend_from_slice(&token_amount.to_le_bytes());
        data.extend_from_slice(&min_sol_out.to_le_bytes());

        Ok(Instruction::new_with_bytes(
            PUBKEY_PUMPFUN,
            &data,
            vec![
                AccountMeta::new_readonly(PUBKEY_GLOBAL_ACCOUNT, false),
                AccountMeta::new(PUBKEY_FEE_RECIPIENT, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new(*bonding_curve, false),
                AccountMeta::new(get_associated_token_address(&bonding_curve, mint), false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new(*creator_vault, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(PUBKEY_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_PUMPFUN, false),
            ],
        ))
    }
}
