use crate::{tx_context::TxContext, BuyAmapParams, BuyParams, CreateMemeResponse, CreateTokenApiParams, CreateTokenParams, FourMemeEvent, GetTokenInfoByIdResponse, SellAmapParams, TokenManager3::TokenInfo};
use alloy::{
    eips::BlockNumberOrTag, hex, primitives::{address, Address, Bytes, FixedBytes, TxKind, U256}, providers::{DynProvider, Provider, ProviderBuilder}, rpc::types::TransactionRequest, signers::{local::PrivateKeySigner, Signature, Signer}, sol
};
use reqwest;
use futures::StreamExt;
use tokio::sync::mpsc;



//pub const FOUR_MEME_CONTRACT_ADDRESS: Address = address!("ec4549cadce5da21df6e6422d448034b5233bfbc");
pub const FOUR_MEME_CONTRACT_ADDRESS: Address = address!("0x5c952063c7fc8610ffdb798152d69f0b9550762b");


sol!(
    #[sol(rpc)]
    IFourMeme,
    "src/abi/four_meme.json"
);


sol! {
    #[sol(rpc)]
    interface IERC20 {
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 amount) external returns (bool);
        function balanceOf(address who) external view returns (uint256);
        function decimals() external view returns (uint8);
    }
}



const MIN_GAS_PRICE_WEI: u128 = 50_000_000;


pub async fn supports_eip1559(provider: &DynProvider) -> eyre::Result<bool> {
    // 读取 latest 区块，若有 base_fee 则表示节点支持 EIP-1559
    let blk = provider.get_block_by_number(BlockNumberOrTag::Latest).await?;
    
    match blk {
        Some(b) => Ok(b.header.base_fee_per_gas.is_some()),
        None => Ok(false),
    }
}



#[derive(Clone)]
pub struct FourMemeSdk {
    pub provider: DynProvider,
    pub address: Address,
    pub contract: IFourMeme::IFourMemeInstance<DynProvider>,
    pub four_meme_api_base: String,
}

impl FourMemeSdk {
    pub fn new_with_rpc(
        rpc_url: &str, 
        signer: PrivateKeySigner,
        chain_id: u64, 
        contract_address: Option<Address>,
        four_meme_api_base: Option<String>,
    ) -> eyre::Result<Self> {
        let contract_address = contract_address.unwrap_or(FOUR_MEME_CONTRACT_ADDRESS);

        // signer
        let signer = signer.with_chain_id(Some(chain_id));

        let provider = ProviderBuilder::new()
            .wallet(signer)
            .connect_http(rpc_url.parse()?);

        let provider = DynProvider::new(provider);


        let four_meme_api_base = four_meme_api_base.unwrap_or("https://four.meme/meme-api/v1".to_string());


        let contract = IFourMeme::new(contract_address, provider.clone());
        Ok(Self { provider, address: contract_address, contract, four_meme_api_base })
    }

    pub async fn new_with_provider(
        provider: DynProvider,
        contract_address: Option<Address>,
        four_meme_api_base: Option<String>,
    ) -> eyre::Result<Self> {
        let contract_address = contract_address.unwrap_or(FOUR_MEME_CONTRACT_ADDRESS);
        let contract = IFourMeme::new(contract_address, provider.clone());
        let four_meme_api_base = four_meme_api_base.unwrap_or("https://four.meme/meme-api/v1".to_string());

        Ok(Self { provider, address: contract_address, contract, four_meme_api_base })
    }
}

impl FourMemeSdk {
    pub async fn token_info(&self, token: Address) -> eyre::Result<TokenInfo> {
        
        let res = self.contract._tokenInfos(token).call().await?;

        Ok(TokenInfo{
            base: res.base,
            quote: res.quote,
            template: res.template,
            totalSupply: res.totalSupply,
            maxOffers: res.maxOffers,
            maxRaising: res.maxRaising,
            launchTime: res.launchTime,
            offers: res.offers,
            funds: res.funds,
            lastPrice: res.lastPrice,
            K: res.K,
            T: res.T,
            status: res.status,
        })
    }

    async fn add_liquidity(&self, token_address: Address) -> eyre::Result<IFourMeme::addLiquidityReturn> {
        Ok(self.contract.addLiquidity(token_address).call().await?)
    }

    async fn add_template(
        &self, 
        quote: Address, 
        initial_liquidity: U256,
        max_raising: U256,
        total_supply: U256,
        max_offers: U256,
        min_trading_fee: U256,
    ) -> eyre::Result<IFourMeme::addTemplateReturn> {
        Ok(self.contract.addTemplate(
            quote,
            initial_liquidity,
            max_raising,
            total_supply,
            max_offers,
            min_trading_fee,
        ).call().await?)
    }
    
    async fn buy_token(
        &self,
        params: BuyParams,
    ) -> eyre::Result<alloy::primitives::TxHash> {
        let calldata = self.build_buy_token_tx(params.clone()).await?;
        let ctx = self.fetch_tx_context().await?;

        let tx = TransactionRequest::default()
            .to(*self.contract.address())
            .value(params.max_funds)
            .max_priority_fee_per_gas(ctx.max_priority_fee_per_gas)
            .max_fee_per_gas(ctx.max_fee_per_gas)
            .input(calldata.into());
        
        let pending = self.provider.send_transaction(tx).await?;

        Ok(*pending.tx_hash())
    }

    async fn build_buy_token_tx(
        &self,
        params: BuyParams,
    ) -> eyre::Result<Bytes> {
        let calldata = match params.to {
            Some(to) => self.contract.buyToken_0(params.token, to, params.amount, params.max_funds)
                .calldata()
                .to_owned(),
            None => self.contract.buyToken_1(params.token, params.amount, params.max_funds)
                .calldata()
                .to_owned()
        };

        Ok(calldata)
    }

    pub async fn build_ensure_allowance_tx(
        &self,
        token: Address,        // 要卖的ERC20地址（通常是 token_info.base）
        owner: Address,        // 你的地址
        needed: U256,          // 卖出数量
    ) -> eyre::Result<Option<TransactionRequest>> {
        let erc20 = IERC20::new(token, self.provider.clone());
        let current = erc20.allowance(owner, *self.contract.address()).call().await?;

        if current >= needed {
            return Ok(None);
        }

        let calldata = erc20.approve(*self.contract.address(), needed)
            .calldata()
            .to_owned();

        let gas_price = self.provider.get_gas_price().await.unwrap_or(MIN_GAS_PRICE_WEI.into());

        let tx = TransactionRequest::default()
            .from(owner)
            .to(token)                 
            .gas_price(gas_price)
            .input(calldata.into());

        Ok(Some(tx))
    }

    pub async fn buy_token_amap(
        &self,
        params: BuyAmapParams,
    ) -> eyre::Result<alloy::primitives::TxHash> {
        let calldata = self.build_buy_token_amap_tx(params.clone()).await?;

        let ctx = self.fetch_tx_context().await?;
        
        let tx = TransactionRequest::default()
            .to(*self.contract.address())
            .value(params.funds)
            .max_priority_fee_per_gas(ctx.max_priority_fee_per_gas)
            .max_fee_per_gas(ctx.max_fee_per_gas)
            .input(calldata.into());


        let pending = self.provider.send_transaction(tx).await?;

        Ok(*pending.tx_hash())    
    }



    pub async fn build_buy_token_amap_tx(
        &self,
        params: BuyAmapParams,
    ) -> eyre::Result<Bytes> {
        let calldata = match params.to {
            Some(to) => self.contract.buyTokenAMAP_0(params.token, to, params.funds, params.min_amount)
                .calldata()
                .to_owned(),
            None => self.contract.buyTokenAMAP_1(params.token, params.funds, params.min_amount)
                .calldata()
                .to_owned()
        };

        Ok(calldata)
    }

    pub async fn fetch_tx_context(&self) -> eyre::Result<TxContext> {
        let min = u128::from(MIN_GAS_PRICE_WEI); // 0.05 gwei
        let base = self.provider.get_gas_price().await.unwrap_or(min); // Fallback to legacy gas price
        let max_priority = min.max(u128::from(1_000_000_000u64)); // 1 gwei
        let max_fee = base * u128::from(2u64) + max_priority;

        Ok(TxContext{
            max_priority_fee_per_gas: max_priority,
            max_fee_per_gas: max_fee,
        }) 
    }

    pub async fn get_nonce_1(&self, address: Address) -> eyre::Result<u64> {
        Ok(self.provider.get_transaction_count(address).await?)
    }

    pub async fn sell_token_amap(
        &self,
        params: SellAmapParams,
        user_address: Address,
    ) -> eyre::Result<alloy::primitives::TxHash> {
        let calldata = self.build_sell_token_amap_calldata(params).await?;

        // let nonce = self.get_nonce_1(user_address).await?;

        let mut tx = TransactionRequest::default()
            .from(user_address)
            .to(*self.contract.address())
            .value(U256::from(0))
            .input(calldata.into())
            .gas_limit(500000 * 2);

        let chain_id = self.provider.get_chain_id().await?;
        let is_bsc = chain_id == 56 || chain_id == 97;
        let eip1559 = !is_bsc && supports_eip1559(&self.provider).await.unwrap_or(false);

        if eip1559 {
            // 仅在支持 EIP-1559 的链上设置
            let ctx = self.fetch_tx_context().await?; // 你已有的 max_fee / max_priority 计算
            tx = tx
                .max_priority_fee_per_gas(ctx.max_priority_fee_per_gas)
                .max_fee_per_gas(ctx.max_fee_per_gas);
        } else {
            // BSC 或不支持 EIP-1559 的链：使用 legacy gas_price
            let gas_price = self
                .provider
                .get_gas_price()
                .await
                .unwrap_or(5_000_000_000u128);
            tx = tx.gas_price(gas_price);
        }

        let pending = self.provider.send_transaction(tx).await?;

        Ok(*pending.tx_hash())    
    }

    pub async fn calc_sell_cost(
        &self,
        token_info: TokenInfo,
        amount: U256,
    ) -> eyre::Result<alloy::primitives::U256> {
        Ok(self.contract.calcSellCost(token_info, amount).call().await?)
    }

    pub async fn calc_buy_cost(
        &self,
        token_info: TokenInfo,
        amount: U256,
    ) -> eyre::Result<alloy::primitives::U256> {
        Ok(self.contract.calcBuyCost(token_info, amount).call().await?)
    }

    pub async fn build_sell_token_amap_calldata(
        &self,
        params: SellAmapParams,
    ) -> eyre::Result<Bytes> {
        let calldata = match params.min_funds {
            Some(min_funds) => match params.from {
                Some(from) => {
                    match params.origin {
                        Some(origin) => {
                            match params.fee_rate {
                                Some(fee_rate) => {
                                    match params.fee_recipient {
                                        Some(fee_recipient) => {
                                            self.contract.sellToken_4(
                                                origin,
                                                params.token, 
                                                from,
                                                params.amount, 
                                                min_funds,
                                                fee_rate,
                                                fee_recipient)
                                                    .calldata()
                                                    .to_owned()
                                        },
                                        None => Err(eyre::eyre!("Fee recipient is required when fee rate is provided"))?
                                    }
                                },
                                None => Err(eyre::eyre!("Fee rate is required when fee recipient is provided"))?
                            }
                        },
                        None => Err(eyre::eyre!("Origin is required when from is provided"))?
                    }
                },
                None => {
                    match params.origin {
                        Some(origin) => {
                            match params.fee_rate {
                                Some(fee_rate) => {
                                    match params.fee_recipient {
                                        Some(fee_recipient) => {
                                            self.contract.sellToken_0(
                                                origin,
                                                params.token, 
                                                params.amount, 
                                                min_funds,
                                                fee_rate,
                                                fee_recipient)
                                                    .calldata()
                                                    .to_owned()
                                        },
                                        None => Err(eyre::eyre!("Fee recipient is required when fee rate is provided"))?
                                    }
                                },
                                None => {
                                    self.contract.sellToken_1(
                                        origin,
                                        params.token, 
                                        params.amount,
                                        min_funds,
                                    )
                                        .calldata()
                                        .to_owned()
                                }
                            }
                        },
                        None => {
                            println!("sellToken_3");
                            self.contract.sellToken_3(
                                params.token, 
                                params.amount,
                                min_funds,
                            )
                                .calldata()
                                .to_owned()
                        }
                    }
                }
            }
            None => {
                match params.origin {
                    Some(origin) => {
                        
                        self.contract.sellToken_2(
                            origin,
                            params.token, 
                            params.amount
                        )
                                .calldata()
                                .to_owned()

                    },
                    None => {
                        self.contract.sellToken_5(
                            params.token, 
                            params.amount
                        )
                                .calldata()
                                .to_owned()
                    }
                }
            }
        };

        Ok(calldata)
    }

    pub async fn create_token_0(
        &self,
        params: CreateTokenParams,
        access_token: String,
        signature: Signature,
        user_address: Address,
    ) -> eyre::Result<alloy::primitives::TxHash> {
        let (tx, _) = self.build_create_token_0_tx(params, access_token, signature, user_address).await?;

        let tx = TransactionRequest::default()
            .from(user_address)
            .to(*self.contract.address())
            .input(tx.into());

        let pending= self.provider.send_transaction(tx).await?;
        
        Ok(*pending.tx_hash())
    }

    pub async fn build_create_token_0_tx(
        &self,
        params: CreateTokenParams,
        access_token: String,
        signature: Signature,
        user_address: Address,
    ) -> eyre::Result<(Bytes, U256)> {   
        let chain_id = self.provider.get_chain_id().await?;
        let network = if chain_id == 56 { "BSC" } else { "ETH" };

        let res = self.call_create_token_api(
            CreateTokenApiParams{
            access_token: access_token,
            name: params.name,
            short_name: params.short_name,
            desc: params.description,
            total_supply: params.total_supply.unwrap_or(U256::from(1000000000)),
            raised_amount: params.raised_amount.unwrap_or(U256::from(24)),
            pre_sale: params.pre_sale.unwrap_or(U256::from(0)),
            sale_rate: params.sale_rate.unwrap_or(0.8),
            signature: signature.to_string(),
            user_address: user_address.to_string(),
            img_url: params.img_url,
            network: network.to_string(),
        }).await?;

        let args = hex::decode(res.data.create_arg.trim_start_matches("0x")).unwrap().into();
        let signature = hex::decode(res.data.signature.trim_start_matches("0x")).unwrap().into();

        let calldata = self.contract.createToken_0(args, signature)
            .calldata()
            .to_owned();
                
        Ok((calldata, U256::from(res.data.token_id)))
    }


    pub async fn build_signature_message(
        &self,
        user_address: Address,
    ) -> eyre::Result<String> {
        // Step 1: Get nonce from API
        let nonce = self.get_nonce(user_address).await?;

        // Return the message to be signed
        let message = format!("You are sign in Meme {}", nonce);
        
        Ok(message)
    }

    async fn get_nonce(&self, account_address: Address) -> eyre::Result<String> {
        let chain_id = self.provider.get_chain_id().await?;
        let network_code = if chain_id == 56 { "BSC" } else { "ETH" };

        let client = reqwest::Client::new();
        
        let request_body = serde_json::json!({
            "accountAddress": account_address,
            "verifyType": "LOGIN", 
            "networkCode": network_code
        });

        let response = client
            .post(format!("{}/private/user/nonce/generate", self.four_meme_api_base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json") 
            .header("origin", "https://four.meme")
            .header("referer", "https://four.meme/create-token")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre::eyre!(
                "Get nonce API request failed with status {}",
                response.status()
            ));
        }

        let nonce_response = response.json::<serde_json::Value>().await?;
        Ok(nonce_response["data"].as_str().unwrap_or_default().to_string())
    }


    pub async fn get_access_token(
        &self,
        signature: Signature, 
        address: Address,
    ) -> eyre::Result<String> {
        let client = reqwest::Client::new();

        let verify_info = serde_json::json!({
            "signature": signature.to_string(),
            "address": address, 
            "networkCode": "BSC",
            "verifyType": "LOGIN"
        });

        let request_body = serde_json::json!({
            "verifyInfo": verify_info
        });

        let response = client
            .post(format!("{}/private/user/login/dex", self.four_meme_api_base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("origin", "https://four.meme")
            .header("referer", "https://four.meme/create-token")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre::eyre!(
                "Get access token API request failed with status {}",
                response.status()
            ));
        }

        let access_token_response = response.json::<serde_json::Value>().await?;
        Ok(access_token_response["data"].as_str().unwrap_or_default().to_string())
    }
  
       

    async fn call_create_token_api(
        &self,
        params: CreateTokenApiParams,
    ) -> eyre::Result<CreateMemeResponse> {
        let launch_time = chrono::Utc::now().timestamp_millis();

        let raised_token = serde_json::json!({
            "symbol": "BNB",
            "nativeSymbol": "BNB", 
            "symbolAddress": "0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c",
            "deployCost": "0",
            "buyFee": "0.01",
            "sellFee": "0.01",
            "minTradeFee": "0",
            "b0Amount": "8",
            "totalBAmount": "24",
            "totalAmount": "1000000000",
            "logoUrl": "https://static.four.meme/market/68b871b6-96f7-408c-b8d0-388d804b34275092658264263839640.png",
            "tradeLevel": ["0.1", "0.5", "1"],
            "status": "PUBLISH",
            "buyTokenLink": "https://pancakeswap.finance/swap",
            "reservedNumber": 10,
            "saleRate": "0.8",
            "networkCode": "BSC",
            "platform": "MEME"
        });

        let request_body = serde_json::json!({
            "name": params.name,
            "shortName": params.short_name,
            "desc": params.desc,
            "totalSupply": params.total_supply.to_string(),
            "raisedAmount": params.raised_amount.to_string(),
            "saleRate": params.sale_rate.to_string(),
            "reserveRate": 0,
            "imgUrl": params.img_url,
            "raisedToken": raised_token,
            "launchTime": launch_time,
            "funGroup": false,
            "preSale": params.pre_sale.to_string(),
            "clickFun": false,
            "symbol": "BNB",
            "label": "Meme"
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/private/token/create", self.four_meme_api_base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json") 
            .header("meme-web-access", params.access_token)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre::eyre!(
                "Create token API request failed with status {}",
                response.status()
            ));
        }

        let response_bytes = response.bytes().await?;
        
        let value: serde_json::Value = serde_json::from_slice(&response_bytes)?;
        // println!("response json: {}", serde_json::to_string_pretty(&value)?);
        
        let response_data: CreateMemeResponse = serde_json::from_value(value)?;
        Ok(response_data)
    }


    pub async fn create_token_1(
        &self,
        args: Bytes,
    ) -> eyre::Result<IFourMeme::createToken_1Return> {   
        Ok(self.contract.createToken_1(args).call().await?)
    }

    pub async fn grant_deployer(
        &self,
        account: Address,
    ) -> eyre::Result<IFourMeme::grantDeployerReturn> {
        Ok(self.contract.grantDeployer(account).call().await?)
    }

    pub async fn grant_operator(
        &self,
        account: Address,
    ) -> eyre::Result<IFourMeme::grantOperatorReturn> {
        Ok(self.contract.grantOperator(account).call().await?)
    }

    pub async fn grant_role(
        &self,
        role: FixedBytes<32>,
        account: Address,
    ) -> eyre::Result<IFourMeme::grantRoleReturn> {
        Ok(self.contract.grantRole(role, account).call().await?)
    }

    pub async fn initialize_0(
        &self,
    ) -> eyre::Result<IFourMeme::initialize_0Return> {
        Ok(self.contract.initialize_0().call().await?)
    }

    pub async fn initialize_1(
        &self,
        signer: Address,
        fee_recipient: Address,
        token_creator: Address,
        referral_reward_keeper: Address,
        launch_fee: U256,
    ) -> eyre::Result<IFourMeme::initialize_1Return> {
        Ok(self.contract.initialize_1(
            signer,
            fee_recipient,
            token_creator,
            referral_reward_keeper,
            launch_fee,
        ).call().await?)
    }



    /*
    pub async fn calc_last_price(&self, token: Address) -> eyre::Result<U256> {
        Ok(self.contract.calcLastPrice(token).call().await?)
    }*/

    // Pure functions: Calculate buy/sell costs using on-chain TokenInfo (local execution, no transaction)
    /*
    pub async fn quote_buy_cost(&self, token: Address, amount: U256) -> eyre::Result<U256> {
        let ti = self.token_info(token).await?;
        Ok(self.contract._calcBuyCost(ti, amount).call().await?)
    }

    pub async fn quote_sell_cost(&self, token: Address, amount: U256) -> eyre::Result<U256> {
        let ti = self.token_info(token).await?;
        Ok(self.contract._calcSellCost(ti, amount).call().await?)
    }*/

    
    pub async fn get_token_info_by_id(
        &self,
        token_id: U256,
        access_token: String,
    ) -> eyre::Result<GetTokenInfoByIdResponse> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/private/token/getById", self.four_meme_api_base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json") 
            .header("meme-web-access", access_token)
            .query(&[("id", token_id.to_string())])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(eyre::eyre!(
                "Get token info API request failed with status {}",
                response.status()
            ));
        }
        
        let response_data = response.json::<GetTokenInfoByIdResponse>().await?;
        Ok(response_data)
    }

    pub async fn subscribe_events(&self) -> eyre::Result<(tokio::task::JoinHandle<()>, mpsc::Receiver<FourMemeEvent>)> {
        let token_purchase_filter = self.contract.TokenPurchase_filter().watch().await?;
        let token_sale_filter = self.contract.TokenSale_filter().watch().await?;
        let token_created_filter = self.contract.TokenCreate_filter().watch().await?;
      
        let mut token_purchase_stream = token_purchase_filter.into_stream();
        let mut token_sale_stream = token_sale_filter.into_stream();
        let mut token_created_stream = token_created_filter.into_stream();

        let (tx, rx) = mpsc::channel::<FourMemeEvent>(1024);

        // Use tokio spawn to handle event streams
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = token_purchase_stream.next() => {
                        if let Ok((purchase_event, _)) = event {
                            let _ = tx.send(FourMemeEvent::TokenPurchase(purchase_event)).await;
                        }
                    }
                    Some(event) = token_sale_stream.next() => {
                        if let Ok((sale_event, _)) = event {
                            let _ = tx.send(FourMemeEvent::TokenSale(sale_event)).await;

                        }
                    }
                    Some(event) = token_created_stream.next() => {
                        if let Ok((created_event, _)) = event {
                            let _ = tx.send(FourMemeEvent::TokenCreate(created_event)).await;
                        }
                    }
                    else => break,
                }
            }
        });

        Ok((handle, rx))
    }
    
}


#[cfg(test)]
mod tests {
    use alloy::hex;

    use super::*;

    fn create_sdk() -> eyre::Result<FourMemeSdk> {
        let signer = PrivateKeySigner::random();
        let private_key_hex = format!("0x{}", hex::encode(signer.to_bytes()));
        let signer = private_key_hex.parse()?;
        

        let sdk = FourMemeSdk::new_with_rpc(
            // "https://bsc-dataseed.bnbchain.org", 
            "https://bsc.blockrazor.xyz", 
            signer, 
            56, 
            Some(FOUR_MEME_CONTRACT_ADDRESS),
            None,
        );

        sdk
    }

    #[tokio::test]
    async fn test_create_token_0() {
        let private_key_hex = std::fs::read_to_string(
            dirs::home_dir()
                .unwrap()
                .join(".config/bsc/four_meme_test.txt")
                .to_str()
                .unwrap()
        )
            .expect("Failed to read private key file")
            .trim()
            .to_string();
        let signer: PrivateKeySigner = private_key_hex.parse()
            .expect("Invalid private key format");

        println!("Address generated from private key file: {:?}", signer.address());


        // let signer = PrivateKeySigner::random();
        // let private_key_hex = format!("0x{}", hex::encode(signer.to_bytes()));
        // let signer: PrivateKeySigner = private_key_hex.parse().unwrap();

        let sdk = FourMemeSdk::new_with_rpc(
            // "https://bsc-dataseed.bnbchain.org", 
            "https://bsc.blockrazor.xyz", 
            signer.clone(), 
            56, 
            Some(FOUR_MEME_CONTRACT_ADDRESS),
            None,
        ).unwrap();

        let balance = sdk.provider.get_balance(signer.address()).await.unwrap();
        println!("BNB Balance: {} BNB", balance);

        let message = sdk.build_signature_message(signer.address()).await.unwrap();
        println!("message: {:?}, address: {:?}", message, signer.address());
        
        let signature = signer.sign_message(message.as_bytes()).await.unwrap();
        println!("signature: {:?}", signature.to_string());

        let access_token = sdk.get_access_token(signature, signer.address()).await.unwrap();
        println!("access_token: {:?}", signature.to_string());

        let (calldata, token_id) = sdk.build_create_token_0_tx(
            CreateTokenParams {
                name: "Aster mama".to_string(),
                short_name: "aster".to_string(),
                description: "robot".to_string(),
                img_url: "https://static.four.meme/market/0ad70de8-3340-455a-ad32-fd9220afbe8d9659231122035551820.jpg".to_string(),
                total_supply: None,
                raised_amount: None,
                sale_rate: None,
                pre_sale: None,
            },
            access_token.clone(),
            signature, 
            signer.address()
        ).await.unwrap();

        let tx = TransactionRequest::default()
            .from(signer.address())
            .to(*sdk.contract.address())
            .input(calldata.into());


        let pending = sdk.provider.send_transaction(tx).await.unwrap();
        println!("pending: {:?}", pending.tx_hash());

        let token_info = sdk.get_token_info_by_id(token_id, access_token).await.unwrap();
        println!("token_info: {:?}", token_info);
    }

    #[tokio::test]
    async fn test_add_liquidity() {
        let sdk = create_sdk().unwrap();

        sdk.add_liquidity("0x3a833aa7c4f1ce660e8dc7f49cfbced4e50d4444".parse::<Address>().unwrap()).await.unwrap();

    }

    #[tokio::test]
    async fn test_token_info() {
        let signer = PrivateKeySigner::random();
        let private_key_hex = format!("0x{}", hex::encode(signer.to_bytes()));
        let signer = private_key_hex.parse::<PrivateKeySigner>().unwrap();
        

        let sdk = FourMemeSdk::new_with_rpc(
            // "https://bsc-dataseed.bnbchain.org", 
            "https://bsc.blockrazor.xyz", 
            signer.clone(), 
            56, 
            Some(FOUR_MEME_CONTRACT_ADDRESS),
            None,
        ).unwrap();


        let message = sdk.build_signature_message(signer.address()).await.unwrap();
        let signature = signer.sign_message(message.as_bytes()).await.unwrap();

        let access_token = sdk.get_access_token(signature, signer.address()).await.unwrap();
        let token_info_1 = sdk.get_token_info_by_id(U256::from(100640609), access_token).await.unwrap();
        println!("token_info_1: {:?}", token_info_1);

        let token_info = sdk.token_info("0x3a833aa7c4f1ce660e8dc7f49cfbced4e50d4444".parse::<Address>().unwrap()).await.unwrap();
        let launch_time = chrono::DateTime::from_timestamp(token_info.launchTime.to::<i64>(), 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        println!("Launch time: {}, Total supply: {}, Last price: {}", launch_time, token_info.totalSupply, token_info.lastPrice);
    }

    #[tokio::test]
    async fn test_buy() {    
        let private_key_hex = std::fs::read_to_string(
            dirs::home_dir()
                .unwrap()
                .join(".config/bsc/four_meme_test.txt")
                .to_str()
                .unwrap()
        )
            .expect("Failed to read private key file")
            .trim()
            .to_string();
        let signer: PrivateKeySigner = private_key_hex.parse()
            .expect("Invalid private key format");

        let sdk = FourMemeSdk::new_with_rpc(
            // "https://bsc-dataseed.bnbchain.org", 
            "https://bsc.blockrazor.xyz", 
            signer.clone(), 
            56, 
            Some(FOUR_MEME_CONTRACT_ADDRESS),
            None,
        ).unwrap();

        /*
        let last_price = sdk.last_price("0x3a833aa7c4f1ce660e8dc7f49cfbced4e50d4444".parse::<Address>().unwrap()).await;
        match last_price {
            Ok(price) => println!("last_price: {:?}", price),
            Err(e) => println!("last_price error: {:?}", e),
        }
        */

        let tx = sdk.buy_token_amap(BuyAmapParams {
            token: "0x857076784c8fa3ab66b27c9a4db4814603ab4444".parse::<Address>().unwrap(),
            funds: U256::from(100),
            min_amount: U256::from(0),
            to: None,
        }).await.unwrap();

        println!("tx: {:?}", tx);
    }

    #[tokio::test]
    async fn test_sell() {    
        let private_key_hex = std::fs::read_to_string(
            dirs::home_dir()
                .unwrap()
                .join(".config/bsc/four_meme_test.txt")
                .to_str()
                .unwrap()
        )
            .expect("Failed to read private key file")
            .trim()
            .to_string();
        let signer: PrivateKeySigner = private_key_hex.parse()
            .expect("Invalid private key format");

        let sdk = FourMemeSdk::new_with_rpc(
            // "https://bsc-dataseed.bnbchain.org", 
            "https://bsc.blockrazor.xyz", 
            signer.clone(), 
            56, 
            Some(FOUR_MEME_CONTRACT_ADDRESS),
            None,
        ).unwrap();

        /*
        let last_price = sdk.last_price("0x3a833aa7c4f1ce660e8dc7f49cfbced4e50d4444".parse::<Address>().unwrap()).await;
        match last_price {
            Ok(price) => println!("last_price: {:?}", price),
            Err(e) => println!("last_price error: {:?}", e),
        }
        */

        let token = "0x857076784c8fa3ab66b27c9a4db4814603ab4444".parse::<Address>().unwrap();
        let amount = "10000000000".parse::<U256>().unwrap();

        if let Some(approve_tx) = sdk.build_ensure_allowance_tx(token, signer.address(), amount).await.unwrap() {
            let pending = sdk.provider.send_transaction(approve_tx).await.unwrap();
            println!("approve tx: {:?}", pending.tx_hash());

            loop {
                if sdk.provider.get_transaction_receipt(*pending.tx_hash()).await.unwrap().is_some() { break; }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }

        let tx = sdk.sell_token_amap(SellAmapParams {
            token,
            amount,
            min_funds: None,
            origin: Some(U256::from(0)),
            from: None,
            fee_rate: None,
            fee_recipient: None,
        }, signer.address()).await.unwrap();

        println!("tx: {:?}", tx);
    }
}