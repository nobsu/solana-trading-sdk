use super::types::SwapInfo;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey, pubkey::Pubkey};

pub const PUBKEY_BOOPFUN: Pubkey = pubkey!("boop8hVGQGqehUK2iVEMEnMrL5RbjywRzHKBmBE7ry4");
pub const PUBKEY_BOOPFUN_CONFIG: Pubkey = pubkey!("AbgFqRWjGWgUaVrZrLLWU5HDY5dktmAL6zT9aacQW7y1");
pub const PUBKEY_BOOPFUN_VAULT_AUTHORITY: Pubkey = pubkey!("GVVUi6DaocSEAp8ATnXFAPNF5irCWjCvmPCzoaGAf5eJ");

pub const BONDING_CURVE_SEED: &[u8] = b"bonding_curve";
pub const BONDING_CURVE_VAULT_SEED: &[u8] = b"bonding_curve_vault";
pub const BONDING_CURVE_SOL_VAULT_SEED: &[u8] = b"bonding_curve_sol_vault";
pub const TRADING_FEE_VAULT_SEED: &[u8] = b"trading_fees_vault";

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BondingCurveStatus {
    Trading = 0,
    Graduated = 1,
    PoolPriceCorrected = 2,
    LiquidityProvisioned = 3,
    LiquidityLocked = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct BondingCurveAccount {
    pub discriminator: u64,
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub graduation_target: u64,
    pub graduation_fee: u64,
    pub sol_reserves: u64,
    pub token_reserves: u64,
    pub damping_term: u8,
    pub swap_fee_basis_points: u8,
    pub token_for_stakers_basis_points: u16,
    pub status: BondingCurveStatus,
}

impl Serialize for BondingCurveStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(&(self.clone() as u8), serializer)
    }
}

impl TryFrom<u8> for BondingCurveStatus {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BondingCurveStatus::Trading),
            1 => Ok(BondingCurveStatus::Graduated),
            2 => Ok(BondingCurveStatus::PoolPriceCorrected),
            3 => Ok(BondingCurveStatus::LiquidityProvisioned),
            4 => Ok(BondingCurveStatus::LiquidityLocked),
            _ => Err(format!("Invalid BondingCurveStatus value: {}", value)),
        }
    }
}

impl<'de> Deserialize<'de> for BondingCurveStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <u8 as Deserialize>::deserialize(deserializer)?;
        BondingCurveStatus::try_from(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct BuyInfo {
    pub discriminator: u64,
    pub sol_amount: u64,
    pub token_amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SellInfo {
    pub discriminator: u64,
    pub token_amount: u64,
    pub sol_amount: u64,
}

impl From<SwapInfo> for BuyInfo {
    fn from(buy: SwapInfo) -> Self {
        Self {
            discriminator: 7598512818552209290,
            token_amount: buy.token_amount,
            sol_amount: buy.sol_amount,
        }
    }
}

impl From<SwapInfo> for SellInfo {
    fn from(sell: SwapInfo) -> Self {
        Self {
            discriminator: 12576214989484342637,
            token_amount: sell.token_amount,
            sol_amount: sell.sol_amount,
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
