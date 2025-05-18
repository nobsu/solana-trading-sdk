use super::{
    amm_calc::{amm_buy_get_token_out, amm_sell_get_sol_out, calculate_with_slippage_buy, calculate_with_slippage_sell},
    dex_traits::DexTrait,
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
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
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

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
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
        sol_lamports: u64,
        slippage_basis_points: u64,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let sol_lamports_with_slippage = calculate_with_slippage_buy(sol_lamports, slippage_basis_points);
        let (pool, pool_base_reserve, pool_quote_reserve) = self.get_pool_liquidity(&mint).await?;
        let blockhash = self.endpoint.rpc.get_latest_blockhash().await?;
        let buy_token_amount = amm_buy_get_token_out(pool_quote_reserve, pool_base_reserve, sol_lamports);

        self.buy_immediately(payer, mint, &pool, None, sol_lamports_with_slippage, buy_token_amount, blockhash, fee, tip)
            .await
    }

    async fn buy_immediately(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        pool: &Pubkey,
        _: Option<&Pubkey>,
        sol_lamports: u64,
        buy_token_amount: u64,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let instruction = self.build_buy_instruction(payer, mint, &pool, buy_token_amount, sol_lamports)?;
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
        let (pool, pool_base_reserve, pool_quote_reserve) = self.get_pool_liquidity(&mint).await?;
        let blockhash = self.endpoint.rpc.get_latest_blockhash().await?;
        let token_amount = token_amount.to_amount(self.endpoint.rpc.clone(), &payer.pubkey(), mint).await?;

        let sol_lamports = amm_sell_get_sol_out(pool_quote_reserve, pool_base_reserve, token_amount);
        let sol_lamports_with_slippage = calculate_with_slippage_sell(sol_lamports, slippage_basis_points);
        self.sell_immediately(
            payer,
            mint,
            &pool,
            None,
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
        _: Option<&Pubkey>,
        token_amount: u64,
        sol_lamports: u64,
        close_mint_ata: bool,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let instruction = self.build_sell_instruction(payer, mint, &pool, token_amount, sol_lamports)?;
        let instructions = build_wsol_sell_instructions(payer, mint, close_mint_ata, instruction)?;
        let signatures = self.endpoint.send_transactions(payer, instructions, blockhash, fee, tip).await?;

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

    pub async fn get_pool_liquidity(&self, mint: &Pubkey) -> anyhow::Result<(Pubkey, u64, u64)> {
        let (pool, pool_account) = self.get_pool(&mint).await?;

        let (pool_base_account, pool_quote_account) = tokio::try_join!(
            self.endpoint.rpc.get_token_account(&pool_account.pool_base_token_account),
            self.endpoint.rpc.get_token_account(&pool_account.pool_quote_token_account),
        )?;

        let pool_base_reserve = u64::from_str(&pool_base_account.unwrap().token_amount.amount)?;
        let pool_quote_reserve = u64::from_str(&pool_quote_account.unwrap().token_amount.amount)?;
        Ok((pool, pool_base_reserve, pool_quote_reserve))
    }

    pub async fn get_pool(&self, mint_address: &Pubkey) -> anyhow::Result<(Pubkey, PoolAccount)> {
        let filters = vec![
            RpcFilterType::DataSize(211),
            RpcFilterType::Memcmp(Memcmp::new(43, MemcmpEncodedBytes::Base58(mint_address.to_string()))),
            RpcFilterType::Memcmp(Memcmp::new(75, MemcmpEncodedBytes::Base58(PUBKEY_WSOL.to_string()))),
        ];

        let accounts = self
            .endpoint
            .rpc
            .get_program_accounts_with_config(
                &PUBKEY_PUMPSWAP,
                RpcProgramAccountsConfig {
                    filters: Some(filters),
                    account_config: RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::Base64),
                        ..Default::default()
                    },
                    ..RpcProgramAccountsConfig::default()
                },
            )
            .await?;

        if accounts.is_empty() {
            return Err(anyhow::anyhow!("No PumpSwap pools found"));
        }

        if accounts.len() > 1 {
            return Err(anyhow::anyhow!("Too many PumpSwap pools found"));
        }

        let (pubkey, account) = &accounts[0];
        let pool_data = PoolAccount::try_from_slice(&account.data)?;

        Ok((pubkey.clone(), pool_data))
    }

    fn build_buy_instruction(&self, payer: &Keypair, mint: &Pubkey, pool: &Pubkey, buy_token_amount: u64, max_sol_cost: u64) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(&[102, 6, 61, 18, 1, 218, 235, 234]); // discriminator
        data.extend_from_slice(&buy_token_amount.to_le_bytes());
        data.extend_from_slice(&max_sol_cost.to_le_bytes());

        let fee_recipient = self.global_account.get().unwrap().protocol_fee_recipients.choose(&mut rand::rng()).unwrap();

        Ok(Instruction::new_with_bytes(
            PUBKEY_PUMPSWAP,
            &data,
            vec![
                AccountMeta::new_readonly(*pool, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(PUBKEY_GLOBAL_ACCOUNT, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new_readonly(PUBKEY_WSOL, false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), &PUBKEY_WSOL), false),
                AccountMeta::new(get_associated_token_address(pool, mint), false),
                AccountMeta::new(get_associated_token_address(pool, &PUBKEY_WSOL), false),
                AccountMeta::new_readonly(*fee_recipient, false),
                AccountMeta::new(get_associated_token_address(fee_recipient, &PUBKEY_WSOL), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
                AccountMeta::new_readonly(PUBKEY_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_PUMPSWAP, false),
            ],
        ))
    }

    pub fn build_sell_instruction(&self, payer: &Keypair, mint: &Pubkey, pool: &Pubkey, token_amount: u64, min_sol_out: u64) -> anyhow::Result<Instruction> {
        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(&[51, 230, 133, 164, 1, 127, 131, 173]); // discriminator
        data.extend_from_slice(&token_amount.to_le_bytes());
        data.extend_from_slice(&min_sol_out.to_le_bytes());

        let fee_recipient = self.global_account.get().unwrap().protocol_fee_recipients.choose(&mut rand::rng()).unwrap();

        Ok(Instruction::new_with_bytes(
            PUBKEY_PUMPSWAP,
            &data,
            vec![
                AccountMeta::new_readonly(*pool, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(PUBKEY_GLOBAL_ACCOUNT, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new_readonly(PUBKEY_WSOL, false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), &PUBKEY_WSOL), false),
                AccountMeta::new(get_associated_token_address(pool, mint), false),
                AccountMeta::new(get_associated_token_address(pool, &PUBKEY_WSOL), false),
                AccountMeta::new_readonly(*fee_recipient, false),
                AccountMeta::new(get_associated_token_address(fee_recipient, &PUBKEY_WSOL), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
                AccountMeta::new_readonly(PUBKEY_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_PUMPSWAP, false),
            ],
        ))
    }
}
