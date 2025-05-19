use super::{
    amm_calc::{amm_buy_get_token_out, amm_sell_get_sol_out, calculate_with_slippage_buy, calculate_with_slippage_sell},
    dex_traits::DexTrait,
    pumpfun::PUBKEY_PUMPFUN,
    types::{Create, TokenAmountType},
};
use crate::{
    common::{accounts::PUBKEY_WSOL, trading_endpoint::TradingEndpoint},
    instruction::builder::{build_wsol_buy_instructions, build_wsol_sell_instructions, PriorityFee},
};
use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::sync::OnceCell;
use rand::seq::IndexedRandom;
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
use std::{str::FromStr, sync::Arc};

pub const PUBKEY_PUMPSWAP: Pubkey = pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
pub const PUBKEY_GLOBAL_ACCOUNT: Pubkey = pubkey!("ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw");
pub const PUBKEY_EVENT_AUTHORITY: Pubkey = pubkey!("GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR");

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct GlobalAccount {
    pub discriminator: u64,
    pub admin: Pubkey,
    pub lp_fee_basis_points: u64,
    pub protocol_fee_basis_points: u64,
    pub disable_flags: u8,
    pub protocol_fee_recipients: [Pubkey; 8],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolAccount {
    pub discriminator: u64,
    pub pool_bump: u8,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub lp_supply: u64,
}
pub struct PoolInfo {
    pub pool_address: Pubkey,
    pub pool_account: PoolAccount,
    pub pool_base_reserve: u64,
    pub pool_quote_reserve: u64,
}

pub struct PumpSwap {
    pub endpoint: Arc<TradingEndpoint>,
    pub global_account: OnceCell<Arc<GlobalAccount>>,
}

#[async_trait::async_trait]
impl DexTrait for PumpSwap {
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

    async fn create(&self, _: Keypair, _: Create, _: Option<PriorityFee>, _: Option<u64>) -> anyhow::Result<Vec<Signature>> {
        Err(anyhow::anyhow!("Not supported"))
    }

    async fn buy(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        sol_amount: u64,
        slippage_basis_points: u64,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let (pool_info, blockhash) = tokio::try_join!(self.get_pool(&mint), self.endpoint.get_latest_blockhash(),)?;
        let buy_token_amount = amm_buy_get_token_out(pool_info.pool_quote_reserve, pool_info.pool_base_reserve, sol_amount);
        let creator_valut = Self::get_creator_vault(&pool_info.pool_account.creator);
        let sol_lamports_with_slippage = calculate_with_slippage_buy(sol_amount, slippage_basis_points);

        self.buy_immediately(
            payer,
            mint,
            None,
            Some(&creator_valut),
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
        _: Option<&Pubkey>,
        creator_vault: Option<&Pubkey>,
        sol_amount: u64,
        buy_token_amount: u64,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let creator_vault = creator_vault.ok_or(anyhow::anyhow!("creator vault not provided: {}", mint.to_string()))?;
        let instruction = self.build_buy_instruction(payer, mint, &creator_vault, buy_token_amount, sol_amount)?;
        let instructions = build_wsol_buy_instructions(payer, mint, sol_amount, instruction)?;
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
        let payer_pubkey = payer.pubkey();
        let (pool_info, blockhash, token_amount) = tokio::try_join!(
            self.get_pool(&mint),
            self.endpoint.get_latest_blockhash(),
            token_amount.to_amount(self.endpoint.rpc.clone(), &payer_pubkey, mint)
        )?;
        let creator_valut = Self::get_creator_vault(&pool_info.pool_account.creator);
        let sol_lamports = amm_sell_get_sol_out(pool_info.pool_quote_reserve, pool_info.pool_base_reserve, token_amount);
        let sol_lamports_with_slippage = calculate_with_slippage_sell(sol_lamports, slippage_basis_points);

        self.sell_immediately(
            payer,
            mint,
            None,
            Some(&creator_valut),
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
        _: Option<&Pubkey>,
        creator_vault: Option<&Pubkey>,
        token_amount: u64,
        sol_amount: u64,
        close_mint_ata: bool,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let creator_vault = creator_vault.ok_or(anyhow::anyhow!("creator vault not provided: {}", mint.to_string()))?;
        let instruction = self.build_sell_instruction(payer, mint, &creator_vault, token_amount, sol_amount)?;
        let instructions = build_wsol_sell_instructions(payer, mint, close_mint_ata, instruction)?;
        let signatures = self.endpoint.build_and_broadcast_tx(payer, instructions, blockhash, fee, tip).await?;

        Ok(signatures)
    }
}

impl PumpSwap {
    pub fn new(endpoint: Arc<TradingEndpoint>) -> Self {
        Self {
            endpoint,
            global_account: OnceCell::new(),
        }
    }

    pub fn get_creator_vault(creator: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"creator_vault", creator.as_ref()], &PUBKEY_PUMPSWAP).0
    }

    pub fn get_pool_authority_pda(mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"pool-authority", mint.as_ref()], &PUBKEY_PUMPFUN).0
    }

    pub fn get_pool_address(mint: &Pubkey) -> Pubkey {
        println!("mint: {:?}", mint.to_string());
        println!("pool_authority: {:?}", Self::get_pool_authority_pda(mint).to_string());
        Pubkey::find_program_address(
            &[
                b"pool",
                &0u16.to_le_bytes(),
                Self::get_pool_authority_pda(mint).as_ref(),
                mint.as_ref(),
                PUBKEY_WSOL.as_ref(),
            ],
            &PUBKEY_PUMPSWAP,
        )
        .0
    }

    pub async fn get_pool(&self, mint: &Pubkey) -> anyhow::Result<PoolInfo> {
        let pool = Self::get_pool_address(&mint);

        let pool_base = get_associated_token_address(&pool, &mint);
        let pool_quote = get_associated_token_address(&pool, &PUBKEY_WSOL);
        let (pool_account, pool_base_account, pool_quote_account) = tokio::try_join!(
            self.endpoint.rpc.get_account(&pool),
            self.endpoint.rpc.get_token_account(&pool_base),
            self.endpoint.rpc.get_token_account(&pool_quote),
        )?;

        if pool_account.data.is_empty() {
            return Err(anyhow::anyhow!("Pool account not found: {}", mint.to_string()));
        }

        let pool_account = bincode::deserialize::<PoolAccount>(&pool_account.data)?;
        let pool_base_account = pool_base_account.ok_or_else(|| anyhow::anyhow!("Pool base account not found: {}", mint.to_string()))?;
        let pool_quote_account = pool_quote_account.ok_or_else(|| anyhow::anyhow!("Pool quote account not found: {}", mint.to_string()))?;

        let pool_base_reserve = u64::from_str(&pool_base_account.token_amount.amount)?;
        let pool_quote_reserve = u64::from_str(&pool_quote_account.token_amount.amount)?;

        Ok(PoolInfo {
            pool_address: pool,
            pool_account,
            pool_base_reserve,
            pool_quote_reserve,
        })
    }

    fn build_buy_instruction(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        creator_vault: &Pubkey,
        buy_token_amount: u64,
        max_sol_cost: u64,
    ) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(&[102, 6, 61, 18, 1, 218, 235, 234]); // discriminator
        data.extend_from_slice(&buy_token_amount.to_le_bytes());
        data.extend_from_slice(&max_sol_cost.to_le_bytes());

        let pool = Self::get_pool_address(&mint);
        let creator_vault_ata = get_associated_token_address(creator_vault, &PUBKEY_WSOL);
        let fee_recipient = self.global_account.get().unwrap().protocol_fee_recipients.choose(&mut rand::rng()).unwrap();

        Ok(Instruction::new_with_bytes(
            PUBKEY_PUMPSWAP,
            &data,
            vec![
                AccountMeta::new_readonly(pool, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(PUBKEY_GLOBAL_ACCOUNT, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new_readonly(PUBKEY_WSOL, false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), &PUBKEY_WSOL), false),
                AccountMeta::new(get_associated_token_address(&pool, mint), false),
                AccountMeta::new(get_associated_token_address(&pool, &PUBKEY_WSOL), false),
                AccountMeta::new_readonly(*fee_recipient, false),
                AccountMeta::new(get_associated_token_address(fee_recipient, &PUBKEY_WSOL), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
                AccountMeta::new_readonly(PUBKEY_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_PUMPSWAP, false),
                AccountMeta::new_readonly(creator_vault_ata, true),
                AccountMeta::new_readonly(*creator_vault, false),
            ],
        ))
    }

    pub fn build_sell_instruction(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        creator_vault: &Pubkey,
        token_amount: u64,
        min_sol_out: u64,
    ) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(&[51, 230, 133, 164, 1, 127, 131, 173]); // discriminator
        data.extend_from_slice(&token_amount.to_le_bytes());
        data.extend_from_slice(&min_sol_out.to_le_bytes());

        let pool = Self::get_pool_address(&mint);
        let creator_vault_ata = get_associated_token_address(creator_vault, &PUBKEY_WSOL);
        let fee_recipient = self.global_account.get().unwrap().protocol_fee_recipients.choose(&mut rand::rng()).unwrap();

        Ok(Instruction::new_with_bytes(
            PUBKEY_PUMPSWAP,
            &data,
            vec![
                AccountMeta::new_readonly(pool, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(PUBKEY_GLOBAL_ACCOUNT, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new_readonly(PUBKEY_WSOL, false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), &PUBKEY_WSOL), false),
                AccountMeta::new(get_associated_token_address(&pool, mint), false),
                AccountMeta::new(get_associated_token_address(&pool, &PUBKEY_WSOL), false),
                AccountMeta::new_readonly(*fee_recipient, false),
                AccountMeta::new(get_associated_token_address(fee_recipient, &PUBKEY_WSOL), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
                AccountMeta::new_readonly(PUBKEY_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_PUMPSWAP, false),
                AccountMeta::new_readonly(creator_vault_ata, true),
                AccountMeta::new_readonly(*creator_vault, false),
            ],
        ))
    }
}
