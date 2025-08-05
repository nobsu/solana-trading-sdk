use super::{dex_traits::DexTrait, moonit_types::*, types::Create};
use crate::{
    common::trading_endpoint::TradingEndpoint,
    dex::types::{PoolInfo, SwapInfo},
    instruction::builder::PriorityFee,
};
use borsh::BorshDeserialize;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use std::sync::Arc;

pub struct Moonit {
    pub endpoint: Arc<TradingEndpoint>,
}

#[async_trait::async_trait]
impl DexTrait for Moonit {
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

        let bonding_curve = CurveAccount::deserialize(&mut account.data.as_slice())?;

        Ok(PoolInfo {
            pool: bonding_curve_pda,
            creator: None,
            creator_vault: None,
            config: None,
            token_reserves: bonding_curve.curve_amount,
            sol_reserves: INITIAL_VIRTUAL_SOL_RESERVES + account.lamports,
        })
    }

    async fn create(&self, _: Keypair, _: Create, _: Option<PriorityFee>, _: Option<u64>) -> anyhow::Result<Vec<Signature>> {
        Err(anyhow::anyhow!("Not supported"))
    }

    fn build_buy_instruction(&self, payer: &Keypair, mint: &Pubkey, _: Option<&Pubkey>, buy: SwapInfo) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let trade_info: TradeParams = TradeParams {
            discriminator: 16927863322537952870,
            token_amount: buy.token_amount,
            collateral_amount: buy.sol_amount,
            fixed_side: FixedSide::ExactIn,
            slippage_bps: 0,
        };

        let buffer = trade_info.to_buffer()?;
        let bonding_curve = Self::get_bonding_curve_pda(mint)?;

        Ok(Instruction::new_with_bytes(
            PUBKEY_MOONIT,
            &buffer,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(bonding_curve, false),
                AccountMeta::new(get_associated_token_address(&bonding_curve, mint), false),
                AccountMeta::new(PUBKEY_MOONIT_DEX_FEE, false),
                AccountMeta::new(PUBKEY_MOONIT_HELIO_FEE, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new_readonly(PUBKEY_MOONIT_CONFIG, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
            ],
        ))
    }

    fn build_sell_instruction(&self, payer: &Keypair, mint: &Pubkey, _: Option<&Pubkey>, sell: SwapInfo) -> anyhow::Result<Instruction> {
        self.initialized()?;

        let trade_info: TradeParams = TradeParams {
            discriminator: 12502976635542562355,
            token_amount: sell.token_amount,
            collateral_amount: sell.sol_amount,
            fixed_side: FixedSide::ExactIn,
            slippage_bps: 0,
        };

        let buffer = trade_info.to_buffer()?;
        let bonding_curve = Self::get_bonding_curve_pda(mint)?;

        Ok(Instruction::new_with_bytes(
            PUBKEY_MOONIT,
            &buffer,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
                AccountMeta::new(bonding_curve, false),
                AccountMeta::new(get_associated_token_address(&bonding_curve, mint), false),
                AccountMeta::new(PUBKEY_MOONIT_DEX_FEE, false),
                AccountMeta::new(PUBKEY_MOONIT_HELIO_FEE, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new_readonly(PUBKEY_MOONIT_CONFIG, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
            ],
        ))
    }
}

impl Moonit {
    pub fn new(endpoint: Arc<TradingEndpoint>) -> Self {
        Self { endpoint }
    }

    pub fn get_bonding_curve_pda(mint: &Pubkey) -> anyhow::Result<Pubkey> {
        let seeds: &[&[u8]; 2] = &[BONDING_CURVE_SEED, mint.as_ref()];
        let pda = Pubkey::try_find_program_address(seeds, &PUBKEY_MOONIT).ok_or_else(|| anyhow::anyhow!("Failed to find bonding curve PDA"))?;
        Ok(pda.0)
    }
}
