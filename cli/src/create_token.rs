
use alloy::{primitives::U256, providers::Provider, rpc::types::TransactionRequest};
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

    #[arg(long)]
    pre_sale: Option<U256>,

    #[arg(long)]
    label: Option<String>,
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
        let cloned_sdk = sdk.clone();

        println!("Wallet address: {:?}", signer.address());

        let balance = sdk.provider.get_balance(signer.address()).await?;
        println!("BNB balance: {} BNB", balance);

        let message = sdk.build_signature_message(signer.address()).await?;
        let signature = signer.sign_message(message.as_bytes()).await?;
        let access_token = sdk.get_access_token(signature, signer.address()).await?;

        let (tx, token_id) = sdk.build_create_token_0_tx(
            CreateTokenParams {
                name: self.name.clone(),
                short_name: self.short_name.clone(), 
                description: self.description.clone(),
                img_url: self.img_url.clone(),
                total_supply: None,
                raised_amount: None,
                sale_rate: None,
                pre_sale: self.pre_sale,
                label: self.label.clone(),
            },
            access_token.clone(),
            signature,
            signer.address()
        ).await?;

        let tx = TransactionRequest::default()
            .from(signer.address())
            .to(*sdk.contract.address())
            .input(tx.into());

        let pending= sdk.provider.send_transaction(tx).await?;
        let tx_hash = *pending.tx_hash();

        println!("Transaction hash: {:?}, token_id: {:?}", tx_hash, token_id);



        sdk.subscribe_events().await?;
        let (_handle, mut rx) = sdk.subscribe_events().await?;

        tokio::spawn(async move {
            loop {
                let event = rx.recv().await.unwrap();
                match event {
                    FourMemeEvent::TokenPurchase(e) => {
                        println!("TokenPurchase event: token: {:?}, account: {:?}, price: {:?}, amount: {:?}", e.token, e.account, e.price, e.amount);
                    }
                    FourMemeEvent::TokenSale(e) => {
                        println!("TokenSale event: token: {:?}, account: {:?}, price: {:?}, amount: {:?}", e.token, e.account, e.price, e.amount);
                    }
                    FourMemeEvent::TokenCreate(e) => {
                        println!("TokenCreate event: requestId: {:?}, token: {:?}, launchTime: {:?}, name: {:?}", e.requestId, e.token, e.launchTime, e.name);

                        let token_info = cloned_sdk.get_token_info_by_id(e.requestId, access_token.clone()).await.unwrap();
                        println!("Token info: {}", serde_json::to_string_pretty(&token_info).unwrap());
                    }
                }
            }
        });


        println!("Waiting for transaction confirmation...");
        let receipt = sdk.provider.get_transaction_receipt(tx_hash).await?;
        while receipt.is_none() {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let receipt = sdk.provider.get_transaction_receipt(tx_hash).await?;
            if receipt.is_some() {
                break;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        println!("Transaction confirmed!");

        Ok(())
    }
}