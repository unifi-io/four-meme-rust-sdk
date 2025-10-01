
use alloy::{primitives::{Address, U256}, providers::Provider};
use clap::Args;
use four_meme_sdk::{FourMemeEvent, FourMemeSdk, SellAmapParams};
use eyre::Result;
use alloy::{
    signers::local::PrivateKeySigner
};

#[derive(Args)]
pub struct SellTokenArgs {
    /// Private key file path
    #[arg(short, long)]
    private_key_path: String,

    #[arg(short, long)]
    token: Address,

    #[arg(short, long)]
    amount: U256,

    #[arg(short, long)]
    min_funds: Option<U256>,
}

impl SellTokenArgs {
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


        // Get token info to check if approval is needed
        let token_info = sdk.token_info(self.token).await?;
        println!("Token info: base: {:?}, totalSupply: {:?}, funds: {:?}", token_info.base, token_info.totalSupply, token_info.funds);

        // Calculate estimated sell cost
        let estimated_sell_cost = sdk.calc_sell_cost(token_info.clone(), self.amount).await?;
        println!("Estimated sell cost: {:?}", estimated_sell_cost);


        if let Some(approve_tx) = sdk.build_ensure_allowance_tx(self.token, signer.address(), self.amount).await? {
            let pending = sdk.provider.send_transaction(approve_tx).await?;
            println!("approve tx: {:?}", pending.tx_hash());

            loop {
                if sdk.provider.get_transaction_receipt(*pending.tx_hash()).await?.is_some() { break; }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }

        let tx_hash = sdk.sell_token_amap(SellAmapParams {
            token: self.token,
            amount: self.amount,
            min_funds: self.min_funds,
            from: None,
            fee_rate: None,
            fee_recipient: None,
            origin: None,
        }, signer.address()).await?;

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