use borsh::{BorshDeserialize, BorshSerialize};

use super::types::{Buy, Sell};

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
