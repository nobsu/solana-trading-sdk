# Solana Trading SDK

A comprehensive Rust SDK for trading on the Solana blockchain, with support for multiple DEXs and advanced transaction submission strategies.

## Features

- **Multi-DEX Support**: Trade on Pump.fun, PumpSwap, and other Solana DEXs
- **Smart Transaction Routing**: Multiple SWQoS (Solana Web Quality of Service) providers for optimal transaction submission
- **Token Creation**: Create and deploy new tokens with metadata on IPFS
- **Priority Fees & MEV Protection**: Built-in support for priority fees and MEV protection through Jito bundles
- **Comprehensive Trading**: Buy, sell, and create tokens with customizable slippage and fees

## Supported DEXs

- **Pumpfun**
- **PumpSwap**
- **RaydiumLaunchpad**
- **Boopfun**
- **Moonshot**: Comming soon
- **Believe**: Comming soon

## Supported SWQoS Providers

- **Default RPC**: Standard Solana RPC endpoints
- **Jito**: MEV protection and bundle submission
- **NextBlock**: High-performance transaction processing
- **Blox**: Advanced routing and execution
- **ZeroSlot**: Fast transaction confirmation
- **Temporal**: Time-based transaction optimization

## Installation

- **cargo add solana-trading-sdk**

## Quick Start

### Basic Setup

```rust
use solana_trading_sdk::{
    common::{TradingClient, TradingConfig},
    swqos::SWQoSType,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = TradingClient::new(TradingConfig {
        rpc_url: "https://solana-rpc.publicnode.com".to_string(),
        swqos: vec![
            SWQoSType::Default("https://solana-rpc.publicnode.com".to_string(), None),
            SWQoSType::Jito("https://mainnet.block-engine.jito.wtf".to_string()),
        ],
    })?;
    
    client.initialize().await?;
    Ok(())
}
```

### Token Trading

```rust
use solana_trading_sdk::{
    dex::types::DexType,
    instruction::builder::PriorityFee,
};
use solana_sdk::{native_token::sol_to_lamports, pubkey::Pubkey, signature::Keypair};

async fn buy_token() -> anyhow::Result<()> {
    let client = get_trading_client().await?;
    let payer = Keypair::from_base58_string("your_private_key");
    let mint = Pubkey::from_str("token_mint_address")?;
    
    let sol_amount = sol_to_lamports(1.0); // 1 SOL
    let slippage_basis_points = 3000; // 30%
    let fee = PriorityFee {
        unit_limit: 100000,
        unit_price: 10000000,
    };
    let tip = sol_to_lamports(0.001); // 0.001 SOL tip
    
    // Buy on Pump.fun
    client.dexs[&DexType::Pumpfun]
        .buy(&payer, &mint, sol_amount, slippage_basis_points, Some(fee), Some(tip))
        .await?;
    
    Ok(())
}
```

### Token Creation

```rust
use solana_trading_sdk::{
    ipfs::{metadata::create_token_metadata, types::CreateTokenMetadata},
    dex::{pumpfun::Pumpfun, types::Create},
};

async fn create_token() -> anyhow::Result<()> {
    // 1. Create metadata
    let token_info = CreateTokenMetadata {
        name: "My Token".to_string(),
        symbol: "MTK".to_string(),
        description: "A revolutionary new token".to_string(),
        file: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string(),
        twitter: Some("@mytoken".to_string()),
        telegram: Some("@mytokengroup".to_string()),
        website: Some("https://mytoken.com".to_string()),
        metadata_uri: None,
    };
    
    let metadata = create_token_metadata(token_info, "your_pinata_jwt_token").await?;
    
    // 2. Create token on Pump.fun
    let payer = Keypair::from_base58_string("your_private_key");
    let mint = Keypair::new();
    
    let create = Create {
        name: metadata.metadata.name,
        symbol: metadata.metadata.symbol,
        uri: metadata.metadata_uri,
        mint: mint.pubkey(),
        buy_sol_amount: Some(sol_to_lamports(0.1)),
        slippage_basis_points: Some(3000),
    };
    
    let pumpfun_client = get_pumpfun_client().await?;
    pumpfun_client.create(payer, create, Some(fee), Some(tip)).await?;
    
    Ok(())
}
```

### SOL and Token Transfers

```rust
async fn transfer_sol() -> anyhow::Result<()> {
    let from = Keypair::from_base58_string("sender_private_key");
    let to = Pubkey::from_str("recipient_address")?;
    let amount = sol_to_lamports(0.1); // 0.1 SOL
    
    let swqos_client = get_swqos_client();
    swqos_client.transfer(&from, &to, amount, Some(fee)).await?;
    Ok(())
}

async fn transfer_token() -> anyhow::Result<()> {
    let from = Keypair::from_base58_string("sender_private_key");
    let to = Pubkey::from_str("recipient_address")?;
    let mint = Pubkey::from_str("token_mint_address")?;
    let amount = 1000; // Token amount in smallest units
    
    let swqos_client = get_swqos_client();
    swqos_client.spl_transfer(&from, &to, &mint, amount, Some(fee)).await?;
    Ok(())
}
```

## Configuration

### SWQoS Providers

Configure multiple SWQoS providers for optimal transaction routing:

```rust
let swqos = vec![
    SWQoSType::Default("https://solana-rpc.publicnode.com".to_string(), None),
    SWQoSType::Jito("https://mainnet.block-engine.jito.wtf".to_string()),
    SWQoSType::NextBlock("https://fra.nextblock.io".to_string(), "your_api_key".to_string()),
    SWQoSType::Blox("https://fra.blox.so".to_string(), "your_api_key".to_string()),
    SWQoSType::ZeroSlot("https://fra.zeroslot.io".to_string(), "your_api_key".to_string()),
    SWQoSType::Temporal("https://fra.temporal.io".to_string(), "your_api_key".to_string()),
];
```

### Priority Fees

Set custom priority fees for faster transaction confirmation:

```rust
let fee = PriorityFee {
    unit_limit: 100000,    // Compute unit limit
    unit_price: 10000000,  // Micro-lamports per compute unit
};
```

## Examples

Check the [`main.rs`](src/main.rs) file for complete working examples of:

- Setting up trading clients
- Buying and selling tokens
- Creating new tokens
- Transferring SOL and SPL tokens
- Using different SWQoS providers

## API Reference

### Core Components

- [`TradingClient`](src/common/trading_client.rs) - Main client for trading operations
- [`TradingEndpoint`](src/common/trading_endpoint.rs) - RPC and SWQoS endpoint management
- [`DexTrait`](src/dex/dex_traits.rs) - Common interface for all DEX implementations

### DEX Implementations

- [`Pumpfun`](src/dex/pumpfun.rs) - Pump.fun DEX implementation
- [`PumpSwap`](src/dex/pumpswap.rs) - PumpSwap DEX implementation

### SWQoS Providers

- [`DefaultSWQoSClient`](src/swqos/default.rs) - Standard RPC client
- [`JitoClient`](src/swqos/jito.rs) - Jito MEV protection
- [`NextBlockClient`](src/swqos/nextblock.rs) - NextBlock routing
- [`BloxClient`](src/swqos/blox.rs) - Blox execution
- [`ZeroSlotClient`](src/swqos/zeroslot.rs) - ZeroSlot confirmation
- [`TemporalClient`](src/swqos/temporal.rs) - Temporal optimization

### IPFS Integration

- [`create_token_metadata`](src/ipfs/metadata.rs) - Upload token metadata to IPFS
- [`CreateTokenMetadata`](src/ipfs/types.rs) - Token metadata structure

## Environment Setup

1. **RPC Endpoint**: Use a reliable Solana RPC endpoint
2. **Pinata JWT Token**: Required for IPFS metadata uploads
3. **SWQoS API Keys**: Optional API keys for premium routing services

## Error Handling

The SDK uses `anyhow::Result` for comprehensive error handling. All functions return detailed error information for debugging.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

This SDK is for educational and development purposes. Always test thoroughly on devnet before using on mainnet. Trading cryptocurrencies involves risk of loss.