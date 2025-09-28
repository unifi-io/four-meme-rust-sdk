# Four Meme Rust SDK

A comprehensive Rust SDK for interacting with the Four Meme protocol on BSC (Binance Smart Chain) and Ethereum networks.

## Features

- 🚀 **Token Creation**: Create meme tokens with custom parameters
- 💰 **Token Trading**: Buy and sell tokens with slippage protection
- 📊 **Token Information**: Query token details and market data
- 🔄 **Event Subscription**: Real-time event monitoring for token activities
- 🔐 **Wallet Integration**: Support for private key and mnemonic-based wallets
- 🌐 **Multi-Network**: Support for both BSC and Ethereum networks
- 📡 **API Integration**: Seamless integration with Four Meme backend APIs

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
four-meme-sdk = "0.1.5"
```

## Quick Start

### Basic Usage

```rust
use four_meme_sdk::{FourMemeSdk, CreateTokenParams};
use alloy_signer_local::PrivateKeySigner;
use alloy::primitives::Address;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Create a signer from private key
    let private_key = "your_private_key_here";
    let signer: PrivateKeySigner = private_key.parse()?;
    
    // Initialize SDK
    let sdk = FourMemeSdk::new_with_rpc(
        "https://bsc.blockrazor.xyz", // BSC RPC URL
        signer,
        56, // BSC chain ID
        None, // Use default contract address
        None, // Use default API base URL
    )?;
    
    // Get token information
    let token_address = "0x...".parse::<Address>()?;
    let token_info = sdk.token_info(token_address).await?;
    println!("Token info: {:?}", token_info);
    
    Ok(())
}
```

### Creating a Token

```rust
use four_meme_sdk::{CreateTokenParams, FourMemeSdk};
use alloy_signer_local::PrivateKeySigner;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let signer: PrivateKeySigner = "your_private_key".parse()?;
    let sdk = FourMemeSdk::new_with_rpc(
        "https://bsc.blockrazor.xyz",
        signer.clone(),
        56,
        None,
        None,
    )?;
    
    // Build signature message and get access token
    let message = sdk.build_signature_message(signer.address()).await?;
    let signature = signer.sign_message(message.as_bytes()).await?;
    let access_token = sdk.get_access_token(signature, signer.address()).await?;
    
    // Create token
    let params = CreateTokenParams {
        name: "My Meme Token".to_string(),
        short_name: "MMT".to_string(),
        description: "A cool meme token".to_string(),
        img_url: "https://example.com/image.png".to_string(),
        total_supply: None, // Use default
        raised_amount: None, // Use default
        sale_rate: None, // Use default
        pre_sale: None, // Use default
    };
    
    let tx_hash = sdk.create_token_0(params, access_token, signature, signer.address()).await?;
    println!("Token created! Transaction hash: {:?}", tx_hash);
    
    Ok(())
}
```

### Event Subscription

```rust
use four_meme_sdk::{FourMemeSdk, FourMemeEvent};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let sdk = FourMemeSdk::new_with_rpc(/* ... */)?;
    
    // Subscribe to events
    let (handle, mut event_receiver) = sdk.subscribe_events().await?;
    
    // Listen for events
    while let Some(event) = event_receiver.recv().await {
        match event {
            FourMemeEvent::TokenPurchase(purchase) => {
                println!("Token purchased: {:?}", purchase);
            }
            FourMemeEvent::TokenSale(sale) => {
                println!("Token sold: {:?}", sale);
            }
            FourMemeEvent::TokenCreate(create) => {
                println!("Token created: {:?}", create);
            }
        }
    }
    
    Ok(())
}
```

## CLI Tool

The project includes a command-line interface for easy interaction:

### Installation

```bash
cargo install --path cli
```

### Usage

#### Create a Token

```bash
four-meme-cli create-token \
  --private-key-path ~/.config/bsc/private_key.txt \
  --name "My Token" \
  --short-name "MTK" \
  --description "A meme token" \
  --img-url "https://example.com/image.png"
```

#### Export Private Key from Mnemonic

```bash
four-meme-cli export-private-key \
  --mnemonic "your mnemonic phrase here" \
  --output ~/.config/bsc/private_key.txt
```

## API Reference

### Core Methods

- `new_with_rpc()` - Create SDK instance with RPC provider
- `token_info()` - Get token information
- `create_token_0()` - Create a new token
- `buy_token_0()` / `buy_token_1()` - Buy tokens
- `subscribe_events()` - Subscribe to contract events

### Event Types

- `TokenPurchase` - Token purchase events
- `TokenSale` - Token sale events  
- `TokenCreate` - Token creation events

## Configuration

### Environment Variables

- `BSC_RPC_URL` - BSC RPC endpoint (default: https://bsc.blockrazor.xyz)
- `FOUR_MEME_API_BASE` - Four Meme API base URL (default: https://four.meme/meme-api/v1)

### Network Support

- **BSC (Binance Smart Chain)**: Chain ID 56
- **Ethereum**: Chain ID 1

## Examples

Check the `examples/` directory for more detailed usage examples.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.

## References

- [Four Meme Protocol](https://four.meme)
- [BlinkAI Protocol Adapter](https://github.com/0xaldric/blinkai)