Solana Trading SDK

Support:
Pumpfun
PumpSwap

SWQoS supported

```rust
pub async fn get_trading_client() -> anyhow::Result<TradingClient> {
    let rpc_url = "https://solana-rpc.publicnode.com".to_string();
    let client = TradingClient::new(TradingConfig {
        rpc_url: rpc_url.to_string(),
        swqos: vec![
            SWQoSType::Default("https://solana-rpc.publicnode.com".to_string(), None),
            SWQoSType::Jito(JITO_ENDPOINT_MAINNET.to_string()),
            SWQoSType::NextBlock(NEXTBLOCK_ENDPOINT_FRA.to_string(), "your_api_key".to_string()),
            SWQoSType::Blox(BLOX_ENDPOINT_FRA.to_string(), "your_api_key".to_string()),
            SWQoSType::ZeroSlot(ZEROSLOT_ENDPOINT_FRA.to_string(), "your_api_key".to_string()),
            SWQoSType::Temporal(TEMPORAL_ENDPOINT_FRA.to_string(), "your_api_key".to_string()),
        ],
    })?;

    client.initialize().await?;
    Ok(client)
}


pub async fn swap() -> anyhow::Result<()> {
    let client = get_trading_client().await?;
    let payer = Keypair::from_base58_string("your_payer_pubkey");
    let mint = Pubkey::from_str("token_mint_pubkey")?;
    let sol_amount = sol_to_lamports(1.0);
    let slippage_basis_points = 3000; // 30%
    let fee = PriorityFee {
        unit_limit: 100000,
        unit_price: 10000000,
    };
    let tip = sol_to_lamports(0.001);

    client.dexs[&DexType::Pumpfun]
        .buy(&payer, &mint, sol_amount, slippage_basis_points, Some(fee), Some(tip))
        .await?;

    client.dexs[&DexType::PumpSwap]
        .buy(&payer, &mint, sol_amount, slippage_basis_points, Some(fee), Some(tip))
        .await?;

    Ok(())
}

```

more example: main.rs

