use alloy::hex;
use alloy::signers::local::{MnemonicBuilder};
use alloy_signer_local::coins_bip39::English;
use clap::Args;
use eyre::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct ExportPrivateKeyArgs {
    /// Mnemonic phrase
    #[arg(short, long)]
    mnemonic: String,

    /// Derivation path
    #[arg(short, long, default_value = "m/44'/60'/0'/0/0")]
    path: String,

    /// Output private key file path
    #[arg(short, long)]
    output: PathBuf,
}

impl ExportPrivateKeyArgs {
    pub async fn execute(&self) -> Result<()> {
        // Build private key from mnemonic
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(self.mnemonic.as_str())
            .derivation_path(&self.path)?
            .build()?;

        let private_key = wallet.to_bytes();
        let private_key_hex = hex::encode(private_key);

        // Write private key to file
        fs::write(&self.output, private_key_hex)
            .map_err(|e| eyre::eyre!("Failed to write private key file {}: {}", self.output.display(), e))?;

        println!("Private key successfully exported to: {}", self.output.display());
        println!("Wallet address: 0x{}", wallet.address());

        Ok(())
    }
}
