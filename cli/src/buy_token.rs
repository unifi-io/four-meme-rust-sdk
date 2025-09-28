
use alloy::{primitives::{Address, U256}, providers::Provider};
use clap::Args;
use four_meme_sdk::{BuyAmapParams, FourMemeEvent, FourMemeSdk};
use eyre::Result;
use alloy::{
    signers::local::PrivateKeySigner
};

#[derive(Args)]
pub struct BuyTokenArgs {
    /// Private key file path
    #[arg(short, long)]
    private_key_path: String,

    #[arg(short, long)]
    token: Address,

    #[arg(short, long)]
    min_amount: U256,

    #[arg(short, long)]
    funds: U256,

    /// Recipient address (optional)
    #[arg(long)]
    to: Option<Address>,
}

impl BuyTokenArgs {
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


        let token_info = sdk.token_info(self.token).await?;

        println!("token_info: base: {:?}, totalSupply: {:?}, funds: {:?}", token_info.base, token_info.totalSupply, token_info.funds);

        let estimated_buy_tokens = sdk.calc_buy_cost(token_info.clone(), self.funds).await?;
        println!("estimated_buy_tokens: {:?}", estimated_buy_tokens);
        
        let estimated_sell_tokens = sdk.calc_sell_cost(token_info.clone(), self.funds).await?;
        println!("estimated_sell_tokens: {:?}", estimated_sell_tokens);


        let tx_hash = sdk.buy_token_amap(BuyAmapParams {
            token: self.token,
            funds: self.funds,
            min_amount: self.min_amount,
            to: self.to,
        }).await?;

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
        println!("Transaction confirmed!, tx_hash: {:?}", tx_hash);

        Ok(())
    }
}