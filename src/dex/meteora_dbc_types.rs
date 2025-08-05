use super::types::SwapInfo;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey, pubkey::Pubkey};

pub const PUBKEY_METEORA_DBC: Pubkey = pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
pub const PUBKEY_METEORA_DBC_POOL_AUTHORITY: Pubkey = pubkey!("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");
pub const PUBKEY_METEORA_DBC_EVENT_AUTHORITY: Pubkey = pubkey!("8Ks12pbrD6PXxfty1hVQiE9sc289zgU1zHkvXhrSdriF");

pub const VIRTUAL_POOL_SEED: &[u8] = b"pool";
pub const VIRTUAL_POOL_BASE_VAULT: &[u8] = b"base_vault";
pub const VIRTUAL_POOL_QUOTE_VAULT: &[u8] = b"quote_vault";

#[derive(Clone, Debug, Deserialize)]
pub struct VolatilityTracker {
    pub last_update_timestamp: u64,
    pub padding: [u8; 8],
    pub sqrt_price_reference: u128,
    pub volatility_accumulator: u128,
    pub volatility_reference: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolMetrics {
    pub total_protocol_base_fee: u64,
    pub total_protocol_quote_fee: u64,
    pub total_trading_base_fee: u64,
    pub total_trading_quote_fee: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct VirtualPool {
    pub discriminator: [u8; 8],
    /// 波动率追踪器
    pub volatility_tracker: VolatilityTracker,
    /// 配置键
    pub config: Pubkey,
    /// 创建者
    pub creator: Pubkey,
    /// 基础代币铸造地址
    pub base_mint: Pubkey,
    /// 基础代币保险库
    pub base_vault: Pubkey,
    /// 报价代币保险库
    pub quote_vault: Pubkey,
    /// 基础代币储备量
    pub base_reserve: u64,
    /// 报价代币储备量
    pub quote_reserve: u64,
    /// 协议基础代币费用
    pub protocol_base_fee: u64,
    /// 协议报价代币费用
    pub protocol_quote_fee: u64,
    /// 合作伙伴基础代币费用
    pub partner_base_fee: u64,
    /// 合作伙伴报价代币费用
    pub partner_quote_fee: u64,
    /// 当前价格的平方根
    pub sqrt_price: u128,
    /// 激活点
    pub activation_point: u64,
    /// 池类型（SPL Token 或 Token2022）
    pub pool_type: u8,
    /// 是否已迁移
    pub is_migrated: u8,
    /// 合作伙伴是否提取剩余
    pub is_partner_withdraw_surplus: u8,
    /// 协议是否提取剩余
    pub is_protocol_withdraw_surplus: u8,
    /// 迁移进度
    pub migration_progress: u8,
    /// 是否提取剩余
    pub is_withdraw_leftover: u8,
    /// 创建者是否提取剩余
    pub is_creator_withdraw_surplus: u8,
    /// 迁移费用提取状态（第一位表示合作伙伴，第二位表示创建者）
    pub migration_fee_withdraw_status: u8,
    /// 池指标
    pub metrics: PoolMetrics,
    /// 曲线完成时间戳
    pub finish_curve_timestamp: u64,
    /// 创建者基础代币费用
    pub creator_base_fee: u64,
    /// 创建者报价代币费用
    pub creator_quote_fee: u64,
    /// 填充字段，用于未来扩展
    pub _padding_1: [u64; 7],
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct SwapInstruction {
    pub discriminator: u64,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
}

impl SwapInstruction {
    pub fn from_swap_info(swap_info: &SwapInfo, is_buy: bool) -> Self {
        if is_buy {
            Self {
                discriminator: 14449647541112719096,
                amount_in: swap_info.sol_amount,
                minimum_amount_out: swap_info.token_amount,
            }
        } else {
            Self {
                discriminator: 14449647541112719096,
                amount_in: swap_info.token_amount,
                minimum_amount_out: swap_info.sol_amount,
            }
        }
    }

    pub fn to_buffer(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        self.serialize(&mut buffer)?;
        Ok(buffer)
    }
}
