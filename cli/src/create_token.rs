
use alloy::providers::Provider;
use clap::Args;
use four_meme_sdk::{CreateTokenParams, FourMemeEvent, FourMemeSdk};
use eyre::Result;
use alloy::{
    signers::{local::PrivateKeySigner, Signer}
};

#[derive(Args)]
pub struct CreateTokenArgs {
    /// Private key file path
    #[arg(short, long)]
    private_key_path: String,

    /// Token name
    #[arg(short, long)]
    name: String,

    /// Token symbol
    #[arg(short, long)]
    short_name: String,

    /// Token description
    #[arg(short, long)]
    description: String,

    /// Token image URL
    #[arg(short, long)]
    img_url: String,
}

impl CreateTokenArgs {
    pub async fn execute(&self) -> Result<()> {
        // Read private key from file
        let private_key_hex = std::fs::read_to_string(&self.private_key_path)
            .map_err(|e| eyre::eyre!("Failed to read private key file {}: {}", self.private_key_path, e))?;
        
        let private_key_hex = private_key_hex.trim();
        let signer: PrivateKeySigner = private_key_hex.parse()
            .map_err(|e| eyre::eyre!("Invalid private key format: {}", e))?;

        let sdk = FourMemeSdk::new_with_rpc(
            "https://bsc.blockrazor.xyz",
            signer.clone(),
            56,
            None,
            None,
        )?;

        println!("Wallet address: {:?}", signer.address());

        let balance = sdk.provider.get_balance(signer.address()).await?;
        println!("BNB balance: {} BNB", balance);

        let message = sdk.build_signature_message(signer.address()).await?;
        let signature = signer.sign_message(message.as_bytes()).await?;
        let access_token = sdk.get_access_token(signature, signer.address()).await?;

        let (tx_req, token_id) = sdk.build_create_token_0_tx(
            CreateTokenParams {
                name: self.name.clone(),
                short_name: self.short_name.clone(), 
                description: self.description.clone(),
                img_url: self.img_url.clone(),
                total_supply: None,
                raised_amount: None,
                sale_rate: None,
                pre_sale: None,
            },
            access_token.clone(),
            signature,
            signer.address()
        ).await?;

        let pending = sdk.provider.send_transaction(tx_req).await?;
        println!("Transaction hash: {:?}", pending.tx_hash());


        sdk.subscribe_events().await?;
        let (_handle, mut rx) = sdk.subscribe_events().await?;

        tokio::spawn(async move {
            loop {
                let event = rx.recv().await.unwrap();
                match event {
                    FourMemeEvent::TokenPurchase(e) => {
                        println!("TokenPurchase event: account: {:?}, price: {:?}, amount: {:?}", e.account, e.price, e.amount);
                    }
                    FourMemeEvent::TokenSale(e) => {
                        println!("TokenSale event: account: {:?}, price: {:?}, amount: {:?}", e.account, e.price, e.amount);
                    }
                    FourMemeEvent::TokenCreate(e) => {
                        println!("TokenCreate event: launchTime: {:?}, name: {:?}", e.launchTime, e.name);
                    }
                }
            }
        });


        println!("Waiting for transaction confirmation...");
        let receipt = sdk.provider.get_transaction_receipt(*pending.tx_hash()).await?;
        while receipt.is_none() {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let receipt = sdk.provider.get_transaction_receipt(*pending.tx_hash()).await?;
            if receipt.is_some() {
                break;
            }
        }
        println!("Transaction confirmed!");


        let token_info = sdk.get_token_info_by_id(token_id, access_token).await?;
        println!("Token info: {}", serde_json::to_string_pretty(&token_info)?);

        Ok(())
    }
}