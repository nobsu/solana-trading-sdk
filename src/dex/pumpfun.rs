use super::{
    amm_calc::{amm_buy_get_token_out, amm_sell_get_sol_out, calculate_with_slippage_buy, calculate_with_slippage_sell},
    dex_traits::DexTrait,
    types::{Buy, Create, Sell, TokenAmountType},
};
use crate::{
    common::trading_endpoint::TradingEndpoint,
    instruction::builder::{build_buy_instructions, build_sell_instructions, PriorityFee},
};
use borsh::{BorshDeserialize, BorshSerialize};
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

pub const INITIAL_VIRTUAL_TOKEN_RESERVES: u64 = 1_073_000_000_000_000;
pub const INITIAL_VIRTUAL_SOL_RESERVES: u64 = 30_000_000_000;

lazy_static::lazy_static! {
    static ref PUBKEY_MINT_AUTHORITY_PDA: Pubkey = Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &PUBKEY_PUMPFUN).0;
    static ref PUBKEY_GLOBAL_PDA: Pubkey = Pubkey::find_program_address(&[GLOBAL_SEED], &PUBKEY_PUMPFUN).0;
}

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

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct BuyInfo {
    pub discriminator: u64,
    pub token_amount: u64,
    pub sol_amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SellInfo {
    pub discriminator: u64,
    pub token_amount: u64,
    pub sol_amount: u64,
}

impl From<Buy> for BuyInfo {
    fn from(buy: Buy) -> Self {
        Self {
            discriminator: 16927863322537952870,
            token_amount: buy.token_amount,
            sol_amount: buy.sol_amount,
        }
    }
}

impl From<Sell> for SellInfo {
    fn from(sell: Sell) -> Self {
        Self {
            discriminator: 12502976635542562355,
            token_amount: sell.token_amount,
            sol_amount: sell.sol_amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CreateInfo {
    pub discriminator: u64,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creator: Pubkey,
}

impl CreateInfo {
    pub fn from_create(create: Create, creator: Pubkey) -> Self {
        Self {
            discriminator: 8576854823835016728,
            name: create.name,
            symbol: create.symbol,
            uri: create.uri,
            creator,
        }
    }
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
            return Err(anyhow::anyhow!("Pumpfun not initialized"));
        }
        Ok(())
    }

    async fn create(&self, payer: Keypair, create: Create, fee: Option<PriorityFee>, tip: Option<u64>) -> anyhow::Result<Vec<Signature>> {
        let mint = create.mint;
        let buy_sol_amount = create.buy_sol_amount;
        let create_info = CreateInfo::from_create(create, payer.pubkey());
        let mut buffer = Vec::new();
        create_info.serialize(&mut buffer)?;

        let blockhash = self.endpoint.rpc.get_latest_blockhash().await?;
        let bonding_curve = Self::get_bonding_curve_pda(&mint).ok_or(anyhow::anyhow!("Bonding curve not found"))?;

        let mut instructions = vec![];
        let create_instruction = Instruction::new_with_bytes(
            PUBKEY_PUMPFUN,
            &buffer,
            vec![
                AccountMeta::new(mint, true),
                AccountMeta::new(*PUBKEY_MINT_AUTHORITY_PDA, false),
                AccountMeta::new(bonding_curve, false),
                AccountMeta::new_readonly(*PUBKEY_GLOBAL_PDA, false),
                AccountMeta::new_readonly(mpl_token_metadata::ID, false),
                AccountMeta::new(mpl_token_metadata::accounts::Metadata::find_pda(&mint).0, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
                AccountMeta::new_readonly(solana_program::sysvar::rent::ID, false),
                AccountMeta::new_readonly(PUBKEY_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_PUMPFUN, false),
            ],
        );

        instructions.push(create_instruction);

        if let Some(buy_sol_amount) = buy_sol_amount {
            let buy_token_amount = amm_buy_get_token_out(INITIAL_VIRTUAL_SOL_RESERVES, INITIAL_VIRTUAL_TOKEN_RESERVES, buy_sol_amount);
            let creator_vault = Self::get_creator_vault_pda(&payer.pubkey()).unwrap();
            let bonding_curve = Self::get_bonding_curve_pda(&mint).ok_or(anyhow::anyhow!("Bonding curve not found"))?;
            let buy_instruction = self.build_buy_instruction(
                &payer,
                &mint,
                &bonding_curve,
                &creator_vault,
                Buy {
                    token_amount: buy_token_amount,
                    sol_amount: buy_sol_amount,
                },
            )?;
            instructions.push(buy_instruction);
        }

        let signatures = self.endpoint.build_and_broadcast_tx(&payer, instructions, blockhash, fee, tip).await?;

        Ok(signatures)
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
        sol_amount: u64,
        buy_token_amount: u64,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let creator = creator.ok_or(anyhow::anyhow!("Creator not provided"))?;
        let creator_vault = Self::get_creator_vault_pda(creator).unwrap();
        let instruction = self.build_buy_instruction(
            payer,
            mint,
            &pool,
            &creator_vault,
            Buy {
                token_amount: buy_token_amount,
                sol_amount,
            },
        )?;
        let instructions = build_buy_instructions(payer, mint, instruction)?;
        let signatures = self.endpoint.build_and_broadcast_tx(payer, instructions, blockhash, fee, tip).await?;

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
        sol_amount: u64,
        close_mint_ata: bool,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let creator = creator.ok_or(anyhow::anyhow!("Creator not provided"))?;
        let creator_vault = Self::get_creator_vault_pda(creator).unwrap();
        let instruction = self.build_sell_instruction(payer, mint, &pool, &creator_vault, Sell { token_amount, sol_amount })?;
        let instructions = build_sell_instructions(payer, mint, instruction, close_mint_ata)?;
        let signatures = self.endpoint.build_and_broadcast_tx(payer, instructions, blockhash, fee, tip).await?;

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

    fn build_buy_instruction(&self, payer: &Keypair, mint: &Pubkey, bonding_curve: &Pubkey, creator_vault: &Pubkey, buy: Buy) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let buy_info: BuyInfo = buy.into();
        let mut buffer = Vec::new();
        buy_info.serialize(&mut buffer)?;

        Ok(Instruction::new_with_bytes(
            PUBKEY_PUMPFUN,
            &buffer,
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
        sell: Sell,
    ) -> anyhow::Result<Instruction> {
        let sell_info: SellInfo = sell.into();
        let mut buffer = Vec::new();
        sell_info.serialize(&mut buffer)?;

        Ok(Instruction::new_with_bytes(
            PUBKEY_PUMPFUN,
            &buffer,
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
