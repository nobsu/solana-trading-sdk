use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;

pub const PUBKEY_MOONIT: Pubkey = pubkey!("MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG");
pub const PUBKEY_MOONIT_DEX_FEE: Pubkey = pubkey!("3udvfL24waJcLhskRAsStNMoNUvtyXdxrWQz4hgi953N");
pub const PUBKEY_MOONIT_HELIO_FEE: Pubkey = pubkey!("5K5RtTWzzLp4P8Npi84ocf7F1vBsAu29N1irG4iiUnzt");
pub const PUBKEY_MOONIT_CONFIG: Pubkey = pubkey!("36Eru7v11oU5Pfrojyn5oY3nETA1a1iqsw2WUu6afkM9");

pub const INITIAL_VIRTUAL_TOKEN_RESERVES: u64 = 1_073_000_000_000_000;
pub const INITIAL_VIRTUAL_SOL_RESERVES: u64 = 30_000_000_000;
pub const BONDING_CURVE_SEED: &[u8] = b"token";

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub enum Currency {
    Sol,
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub enum CurveType {
    LinearV1,
    ConstantProductV1,
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub enum MigrationTarget {
    Raydium,
    Meteora,
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct CurveAccount {
    pub discriminator: u64,
    pub total_supply: u64,
    pub curve_amount: u64,
    pub mint: Pubkey,
    pub decimals: u8,
    pub collateral_currency: Currency,
    pub curve_type: CurveType,
    pub marketcap_threshold: u64,
    pub marketcap_currency: Currency,
    pub migration_fee: u64,
    pub coef_b: u32,
    pub bump: u8,
    pub migration_target: MigrationTarget,
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct TradeParams {
    pub discriminator: u64,
    pub token_amount: u64,
    pub collateral_amount: u64,
    pub fixed_side: FixedSide,
    pub slippage_bps: u64,
}

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub enum FixedSide {
    ExactIn,
    ExactOut,
}

impl TradeParams {
    pub fn to_buffer(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        self.serialize(&mut buffer)?;
        Ok(buffer)
    }
}
