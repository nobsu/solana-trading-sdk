use super::types::SwapInfo;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey, pubkey::Pubkey};

pub const PUBKEY_RAYDIUM_BONK: Pubkey = pubkey!("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj");
pub const PUBKEY_RAYDIUM_BONK_GLOBAL_CONFIG: Pubkey = pubkey!("6s1xP3hpbAfFoNtUNF8mfHsjr2Bd97JxFJRWLbL6aHuX");
pub const PUBKEY_RAYDIUM_BONK_PLATFORM_CONFIG: Pubkey = pubkey!("FfYek5vEz23cMkWsdJwG2oa6EphsvXSHrGpdALN4g6W1");
pub const PUBKEY_RAYDIUM_BONK_AUTHORITY: Pubkey = pubkey!("WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh");
pub const PUBKEY_RAYDIUM_BONK_EVENT_AUTHORITY: Pubkey = pubkey!("2DPAtwB8L12vrMRExbLuyGnC7n2J5LNoZQSejeQGpwkr");

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct BuyInfo {
    pub discriminator: u64,
    pub token_amount: u64,
    pub sol_amount: u64,
    pub share_fee_rate: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SellInfo {
    pub discriminator: u64,
    pub token_amount: u64,
    pub sol_amount: u64,
    pub share_fee_rate: u64,
}

impl From<SwapInfo> for BuyInfo {
    fn from(buy: SwapInfo) -> Self {
        Self {
            discriminator: 17011112658214972154,
            token_amount: buy.token_amount,
            sol_amount: buy.sol_amount,
            share_fee_rate: 0,
        }
    }
}

impl From<SwapInfo> for SellInfo {
    fn from(sell: SwapInfo) -> Self {
        Self {
            discriminator: 1916418889741117333,
            token_amount: sell.token_amount,
            sol_amount: sell.sol_amount,
            share_fee_rate: 0,
        }
    }
}

impl BuyInfo {
    pub fn to_buffer(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        self.serialize(&mut buffer)?;
        Ok(buffer)
    }
}

impl SellInfo {
    pub fn to_buffer(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        self.serialize(&mut buffer)?;
        Ok(buffer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VestingSchedule {
    pub total_locked_amount: u64,
    pub cliff_period: u64,
    pub unlock_period: u64,
    pub start_time: u64,
    pub allocated_share_amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolState {
    pub discriminator: u64,
    pub epoch: u64,
    pub auth_bump: u8,
    pub status: u8,
    pub base_decimals: u8,
    pub quote_decimals: u8,
    pub migrate_type: u8,
    pub supply: u64,
    pub total_base_sell: u64,
    pub virtual_base: u64,
    pub virtual_quote: u64,
    pub real_base: u64,
    pub real_quote: u64,
    pub total_quote_fund_raising: u64,
    pub quote_protocol_fee: u64,
    pub platform_fee: u64,
    pub migrate_fee: u64,
    pub vesting_schedule: VestingSchedule,
    pub global_config: Pubkey,
    pub platform_config: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub creator: Pubkey,
    pub padding: [u64; 8],
}
