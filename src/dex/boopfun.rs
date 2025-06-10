use super::{
    amm_calc::{amm_buy_get_token_out, amm_sell_get_sol_out, calculate_with_slippage_buy, calculate_with_slippage_sell},
    boopfun_tpyes::*,
    dex_traits::{BatchBuyParam, BatchSellParam, DexTrait},
    types::{Buy, Create, CreateATA, Sell, TokenAmountType},
};
use crate::{
    common::{
        accounts::PUBKEY_WSOL,
        trading_endpoint::{BatchTxItem, TradingEndpoint},
    },
    instruction::builder::{build_sol_buy_instructions, build_sol_sell_instructions, PriorityFee},
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

pub struct Boopfun {
    pub endpoint: Arc<TradingEndpoint>,
}

#[async_trait::async_trait]
impl DexTrait for Boopfun {
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
        let buy_token_amount = amm_buy_get_token_out(pool_account.virtual_sol_reserves, pool_account.virtual_token_reserves, sol_lamports);

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
        let instructions = build_sol_buy_instructions(payer, mint, instruction, create_ata)?;
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
        let sol_lamports = amm_sell_get_sol_out(pool_account.virtual_sol_reserves, pool_account.virtual_token_reserves, token_amount);
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
        let instructions = build_sol_sell_instructions(payer, mint, instruction, close_mint_ata)?;
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
        let mut pool_token_amount = pool_account.virtual_token_reserves;
        let mut pool_sol_amount = pool_account.virtual_sol_reserves;
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
            let instructions = build_sol_buy_instructions(&item.payer, mint, instruction, CreateATA::Idempotent)?;
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
        let mut pool_token_amount = pool_account.virtual_token_reserves;
        let mut pool_sol_amount = pool_account.virtual_sol_reserves;
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
            let instructions = build_sol_sell_instructions(&item.payer, mint, instruction, item.close_mint_ata)?;
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

impl Boopfun {
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

    pub async fn get_pool(&self, mint: &Pubkey) -> anyhow::Result<BondingCurveAccount> {
        let bonding_curve_pda = Self::get_bonding_curve_pda(mint).unwrap();
        let account = self.endpoint.rpc.get_account(&bonding_curve_pda).await?;
        if account.data.is_empty() {
            return Err(anyhow::anyhow!("Bonding curve not found: {}", mint.to_string()));
        }

        let bonding_curve = bincode::deserialize::<BondingCurveAccount>(&account.data)?;

        Ok(bonding_curve)
    }

    fn build_buy_instruction(&self, payer: &Keypair, mint: &Pubkey, buy: Buy) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let buy_info: BuyInfo = buy.into(); // TODO
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

    pub fn build_sell_instruction(&self, payer: &Keypair, mint: &Pubkey, sell: Sell) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let sell_info: SellInfo = sell.into(); // TODO
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
