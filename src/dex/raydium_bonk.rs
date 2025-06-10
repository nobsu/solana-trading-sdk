use super::{
    amm_calc::{amm_buy_get_token_out, amm_sell_get_sol_out, calculate_with_slippage_buy, calculate_with_slippage_sell},
    dex_traits::{BatchBuyParam, BatchSellParam, DexTrait},
    raydium_bonk_types::*,
    types::{Buy, Create, CreateATA, Sell, TokenAmountType},
};
use crate::{
    common::{
        accounts::PUBKEY_WSOL,
        trading_endpoint::{BatchTxItem, TradingEndpoint},
    },
    instruction::builder::{build_wsol_buy_instructions, build_wsol_sell_instructions, PriorityFee},
};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use std::sync::Arc;

pub struct RaydiumBonk {
    pub endpoint: Arc<TradingEndpoint>,
}

#[async_trait::async_trait]
impl DexTrait for RaydiumBonk {
    async fn initialize(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn initialized(&self) -> anyhow::Result<()> {
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
        let (pool_account, blockhash) = tokio::try_join!(self.get_pool(&mint), self.endpoint.get_latest_blockhash())?;
        let buy_token_amount = amm_buy_get_token_out(pool_account.virtual_quote, pool_account.virtual_base, sol_lamports);

        self.buy_immediately(
            payer,
            mint,
            None,
            None,
            sol_lamports_with_slippage,
            buy_token_amount,
            blockhash,
            CreateATA::Idempotent,
            fee,
            tip,
        )
    }

    fn buy_immediately(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        _: Option<&Pubkey>,
        _: Option<&Pubkey>,
        sol_amount: u64,
        buy_token_amount: u64,
        blockhash: Hash,
        create_ata: CreateATA,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let instruction = self.build_buy_instruction(
            payer,
            mint,
            Buy {
                token_amount: buy_token_amount,
                sol_amount,
            },
        )?;
        let instructions = build_wsol_buy_instructions(payer, mint, sol_amount, instruction, create_ata)?;
        let signatures = self.endpoint.build_and_broadcast_tx(payer, instructions, blockhash, fee, tip, None)?;

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
        let (pool_account, blockhash, token_amount) = tokio::try_join!(
            self.get_pool(&mint),
            self.endpoint.get_latest_blockhash(),
            token_amount.to_amount(self.endpoint.rpc.clone(), &payer_pubkey, mint)
        )?;
        let sol_lamports = amm_sell_get_sol_out(pool_account.virtual_quote, pool_account.virtual_base, token_amount);
        let sol_lamports_with_slippage = calculate_with_slippage_sell(sol_lamports, slippage_basis_points);

        self.sell_immediately(
            payer,
            mint,
            None,
            None,
            token_amount,
            sol_lamports_with_slippage,
            close_mint_ata,
            blockhash,
            fee,
            tip,
        )
    }

    fn sell_immediately(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        _: Option<&Pubkey>,
        _: Option<&Pubkey>,
        token_amount: u64,
        sol_amount: u64,
        close_mint_ata: bool,
        blockhash: Hash,
        fee: Option<PriorityFee>,
        tip: Option<u64>,
    ) -> anyhow::Result<Vec<Signature>> {
        let instruction = self.build_sell_instruction(payer, mint, Sell { token_amount, sol_amount })?;
        let instructions = build_wsol_sell_instructions(payer, mint, instruction, close_mint_ata)?;
        let signatures = self.endpoint.build_and_broadcast_tx(payer, instructions, blockhash, fee, tip, None)?;

        Ok(signatures)
    }

    async fn batch_buy(
        &self,
        mint: &Pubkey,
        slippage_basis_points: u64,
        fee: PriorityFee,
        tip: u64,
        items: Vec<BatchBuyParam>,
    ) -> anyhow::Result<Vec<Signature>> {
        let (pool_account, blockhash) = tokio::try_join!(self.get_pool(&mint), self.endpoint.get_latest_blockhash())?;
        let mut pool_token_amount = pool_account.virtual_base;
        let mut pool_sol_amount = pool_account.virtual_quote;
        let mut batch_items = vec![];

        for item in items {
            let sol_lamports_with_slippage = calculate_with_slippage_buy(item.sol_amount, slippage_basis_points);
            let buy_token_amount = amm_buy_get_token_out(pool_sol_amount, pool_token_amount, item.sol_amount);
            let instruction = self.build_buy_instruction(
                &item.payer,
                &mint,
                Buy {
                    token_amount: buy_token_amount,
                    sol_amount: sol_lamports_with_slippage,
                },
            )?;
            let instructions = build_wsol_buy_instructions(&item.payer, mint, sol_lamports_with_slippage, instruction, CreateATA::Idempotent)?;
            batch_items.push(BatchTxItem {
                payer: item.payer,
                instructions,
            });
            pool_sol_amount += item.sol_amount;
            pool_token_amount -= buy_token_amount;
        }

        let signatures = self.endpoint.build_and_broadcast_batch_txs(batch_items, blockhash, fee, tip).await?;

        Ok(signatures)
    }

    async fn batch_sell(
        &self,
        mint: &Pubkey,
        slippage_basis_points: u64,
        fee: PriorityFee,
        tip: u64,
        items: Vec<BatchSellParam>,
    ) -> anyhow::Result<Vec<Signature>> {
        let (pool_account, blockhash) = tokio::try_join!(self.get_pool(&mint), self.endpoint.get_latest_blockhash())?;
        let mut pool_token_amount = pool_account.virtual_base;
        let mut pool_sol_amount = pool_account.virtual_quote;
        let mut batch_items = vec![];

        for item in items {
            let sol_lamports = amm_sell_get_sol_out(pool_sol_amount, pool_token_amount, item.token_amount);
            let sol_lamports_with_slippage = calculate_with_slippage_sell(sol_lamports, slippage_basis_points);
            let instruction = self.build_sell_instruction(
                &item.payer,
                &mint,
                Sell {
                    token_amount: item.token_amount,
                    sol_amount: sol_lamports_with_slippage,
                },
            )?;
            let instructions = build_wsol_sell_instructions(&item.payer, mint, instruction, item.close_mint_ata)?;
            batch_items.push(BatchTxItem {
                payer: item.payer,
                instructions,
            });
            pool_sol_amount -= sol_lamports;
            pool_token_amount += item.token_amount;
        }

        let signatures = self.endpoint.build_and_broadcast_batch_txs(batch_items, blockhash, fee, tip).await?;

        Ok(signatures)
    }
}

impl RaydiumBonk {
    pub fn new(endpoint: Arc<TradingEndpoint>) -> Self {
        Self { endpoint }
    }

    pub fn get_pool_pda(mint: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 3] = &[b"pool", mint.as_ref(), PUBKEY_WSOL.as_ref()];
        let pda = Pubkey::try_find_program_address(seeds, &PUBKEY_RAYDIUM_BONK)?;
        Some(pda.0)
    }

    pub fn get_pool_mint_vault(mint: &Pubkey, pool: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 3] = &[b"pool_vault", pool.as_ref(), mint.as_ref()];
        let pda = Pubkey::try_find_program_address(seeds, &PUBKEY_RAYDIUM_BONK)?;
        Some(pda.0)
    }

    pub fn get_pool_quote_vault(quote: &Pubkey, pool: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 3] = &[b"pool_vault", pool.as_ref(), quote.as_ref()];
        let pda = Pubkey::try_find_program_address(seeds, &PUBKEY_RAYDIUM_BONK)?;
        Some(pda.0)
    }

    pub async fn get_pool(&self, mint: &Pubkey) -> anyhow::Result<PoolState> {
        let pool_pda = Self::get_pool_pda(mint).unwrap();
        let account = self.endpoint.rpc.get_account(&pool_pda).await?;
        if account.data.is_empty() {
            return Err(anyhow::anyhow!("Bonding curve not found: {}", mint.to_string()));
        }

        let bonding_curve = bincode::deserialize::<PoolState>(&account.data)?;

        Ok(bonding_curve)
    }

    fn build_buy_instruction(&self, payer: &Keypair, mint: &Pubkey, buy: Buy) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let buy_info: BuyInfo = buy.into();
        let buffer = buy_info.to_buffer()?;
        let pool_address = Self::get_pool_pda(mint).ok_or(anyhow::anyhow!("Bonding curve not found: {}", mint.to_string()))?;
        let pool_base_vault = Self::get_pool_mint_vault(mint, &pool_address).ok_or(anyhow::anyhow!("Bonding curve vault not found: {}", mint.to_string()))?;
        let pool_quote_vault =
            Self::get_pool_quote_vault(&PUBKEY_WSOL, &pool_address).ok_or(anyhow::anyhow!("Bonding curve sol vault not found: {}", mint.to_string()))?;

        Ok(Instruction::new_with_bytes(
            PUBKEY_RAYDIUM_BONK,
            &buffer,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK_GLOBAL_CONFIG, false),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK_PLATFORM_CONFIG, false),
                AccountMeta::new(pool_address, false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), &PUBKEY_WSOL), false),
                AccountMeta::new(pool_base_vault, false),
                AccountMeta::new_readonly(pool_quote_vault, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK, false),
            ],
        ))
    }

    pub fn build_sell_instruction(&self, payer: &Keypair, mint: &Pubkey, sell: Sell) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let sell_info: SellInfo = sell.into();
        let buffer = sell_info.to_buffer()?;
        let pool_address = Self::get_pool_pda(mint).ok_or(anyhow::anyhow!("Bonding curve not found: {}", mint.to_string()))?;
        let pool_base_vault = Self::get_pool_mint_vault(mint, &pool_address).ok_or(anyhow::anyhow!("Bonding curve vault not found: {}", mint.to_string()))?;
        let pool_quote_vault =
            Self::get_pool_quote_vault(&PUBKEY_WSOL, &pool_address).ok_or(anyhow::anyhow!("Bonding curve sol vault not found: {}", mint.to_string()))?;

        Ok(Instruction::new_with_bytes(
            PUBKEY_RAYDIUM_BONK,
            &buffer,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK_GLOBAL_CONFIG, false),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK_PLATFORM_CONFIG, false),
                AccountMeta::new(pool_address, false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), &PUBKEY_WSOL), false),
                AccountMeta::new(pool_base_vault, false),
                AccountMeta::new_readonly(pool_quote_vault, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK_EVENT_AUTHORITY, false),
                AccountMeta::new_readonly(PUBKEY_RAYDIUM_BONK, false),
            ],
        ))
    }
}
