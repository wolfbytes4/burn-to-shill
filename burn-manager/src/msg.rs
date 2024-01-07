use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use secret_toolkit::utils::Query;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub burn_contracts: Vec<ContractInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractInfo {
    pub code_hash: String,
    pub address: Addr,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct RewardsContractInfo {
    pub code_hash: String,
    pub address: Addr,
    pub base_reward: Uint128,
    pub bonus_hourly: Uint128,
    pub name: String,
    pub burn_type: String,
    pub total_rewards: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddContract { contract: ContractInfo },
    RemoveContract { contract: ContractInfo },
    SetActiveState { is_active: bool },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetContracts {},
    GetContractsWithInfo {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractsWithInfoResponse {
    pub contract_info: ContractInfo,
    pub burn_info: BurnInfoResponse,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct BurnInfoResponse {
    pub total_burned_amount: u32,
    pub nft_contract: ContractInfo,
    pub reward_contracts: Vec<RewardsContractInfo>,
    pub trait_restriction: Option<String>,
    pub is_active: bool,
    pub burn_counter_date: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BurnInfoQueryMsg {
    GetBurnInfo {},
}

impl Query for BurnInfoQueryMsg {
    const BLOCK_SIZE: usize = 256;
}
