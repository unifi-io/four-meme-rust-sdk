use alloy::primitives::{address, U256, Address};
use four_meme_sdk::FourMemeSdk;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    // ==== Environment Variables ====
    // Example: BSC Mainnet
    let rpc   = std::env::var("RPC_URL")?;          // e.g. https://bsc-dataseed1.binance.org
    let pk    = std::env::var("PRIVATE_KEY")?;      // With or without 0x prefix
    let chain = std::env::var("CHAIN_ID")?.parse::<u64>()?; // BSC=56, ETH=1, Base=8453, OP=10

    // FourMeme contract address
    let four_meme = std::env::var("FOUR_MEME_ADDR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(address!("0000000000000000000000000000000000000000"));

    let signer = pk.parse().unwrap();

    // ==== Initialize SDK ====
    let _sdk = FourMemeSdk::new_with_rpc(
        &rpc, 
        signer, 
        chain, 
        Some(four_meme),
        None,
    );

    Ok(())
}