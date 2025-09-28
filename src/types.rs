use alloy::primitives::{Address, U256};
use serde::{Deserialize, Serialize};

use crate::IFourMeme;



#[derive(Clone)]
pub enum FourMemeEvent {
    TokenPurchase(IFourMeme::TokenPurchase),
    TokenSale(IFourMeme::TokenSale),
    TokenCreate(IFourMeme::TokenCreate),
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyParams {
    pub token: Address,
    pub amount: U256,
    pub max_funds: U256,
    pub funds: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellParams {
    pub origin: u32,
    pub token: Address,
    pub from: Address,               // Must equal tx.origin
    pub amount_tokens: U256,         // Amount of tokens to sell
    pub min_funds_out: U256,
    pub fee_rate_bp: u32,
    pub fee_recipient: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenParams {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub img_url: String,
    pub total_supply: Option<U256>,
    pub raised_amount: Option<U256>,
    pub sale_rate: Option<f64>,
    pub pre_sale: Option<U256>,

}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub token: Address,
    pub owner: Address,
    pub token_manager: Address,      // Specific TM address for V1/V2
    pub is_bonded: bool,
    pub total_supply: U256,
    pub reserve: U256,
    pub price: U256,                 // Specific fields depend on helper ABI
}





#[derive(Debug, Serialize)]
pub struct CreateTokenApiParams {
    pub access_token: String,
    pub name: String,
    pub short_name: String,
    pub desc: String,
    pub total_supply: U256,
    pub raised_amount: U256,
    pub sale_rate: f64,
    pub signature: String,
    pub user_address: String,
    pub network: String,
    pub img_url: String,
    pub pre_sale: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemeResponse {
    pub code: i64,
    pub msg: String,
    pub data: CreateMemeResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemeResponseData {
    pub token_id: u64,
    pub total_amount: U256,
    pub sale_amount: U256,
    pub template: u32,
    pub launch_time: i64,
    pub server_time: i64,
    pub create_arg: String, 
    pub signature: String,
    pub bamount: String,
    pub tamount: String,
    // ... other fields
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTokenInfoByIdResponse {
    pub code: i64,
    pub msg: String,
    pub data: GetTokenInfoByIdResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTokenInfoByIdResponseData {
    pub id: u64,
    pub address: String,
    pub image: String,
    pub name: String,
    pub short_name: String,
    pub symbol: String,
    pub descr: String,
    pub total_amount: String,
    pub sale_amount: String,
    pub b0: String,
    pub t0: String,
    pub launch_time: i64,
    pub min_buy: String,
    pub max_buy: String,
    pub user_id: i64,
    pub user_address: Address,
    pub user_name: String,
    pub user_avatar: String,
    pub status: String,
    pub show_status: String,
    pub token_price: TokenPrice,
    pub oscar_status: String,
    pub progress_tag: bool,
    pub cto_tag: bool,
    pub version: String,
    pub click_fun_check: bool,
    pub reserve_amount: String,
    pub raised_amount: String,
    pub network_code: String,
    pub label: String,
    pub create_date: String,
    pub modify_date: String,
    pub is_rush: bool,
    pub dex_type: String,
    pub last_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPrice {
    pub price: String,
    pub max_price: String,
    pub increase: String,
    pub amount: String,
    pub market_cap: String,
    pub trading: String,
    pub day_increase: String,
    pub day_trading: String,
    pub raised_amount: String,
    pub progress: String,
    pub liquidity: String,
    pub trading_usd: String,
    pub create_date: String,
    pub modify_date: String,
    pub bamount: String,
    pub tamount: String,
}