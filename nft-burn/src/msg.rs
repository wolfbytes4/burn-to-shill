use cosmwasm_std::{Addr, Binary, Uint128};
use schemars::JsonSchema;
use secret_toolkit::{
    permit::Permit,
    snip721::{Metadata, ViewerInfo},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub entropy: String,
    pub nft_contract: ContractInfo,
    pub reward_contracts: Vec<RewardsContractInfo>,
    pub trait_restriction: Option<String>,
    pub ranks: Vec<Rank>,
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
pub struct History {
    pub token_id: String,
    pub message: String,
    pub date: u64,
    pub rewards: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct HistoryFull {
    pub token_id: String,
    pub message: String,
    pub date: u64,
    pub meta_data: Metadata,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Rank {
    pub token_id: String,
    pub rank: u32,
    pub rank_rewards: Vec<RankReward>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RevokePermit {
        permit_name: String,
    },
    Receive {
        sender: Addr,
        from: Addr,
        amount: Uint128,
        msg: Option<Binary>,
    },
    BatchReceiveNft {
        from: Addr,
        token_ids: Vec<String>,
        msg: Option<Binary>,
    },
    UpdateRewardContract {
        contracts: Vec<RewardsContractInfo>,
    },
    UpdateRanks {
        ranks: Vec<Rank>,
    },
    RemoveRewards {},
    ResetBurnCounterDate {},
    SetViewingKey {
        key: String,
    },
    SetActiveState {
        is_active: bool,
    },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleReceiveMsg {
    ReceiveRewards {},
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleNftReceiveMsg {
    ClaimBurnRewards {
        expected_rewards: Vec<ExpectedCheck>,
        message: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBurnInfo {},
    GetExpectedRewards {
        token_ids: Vec<String>,
    },
    GetRewardBalance {
        viewer: ViewerInfo,
    },
    GetNumUserHistory {
        permit: Permit,
    },
    GetUserHistory {
        permit: Permit,
        start_page: u32,
        page_size: u32,
    },
    GetNumFullHistory {},
    GetFullHistory {
        start_page: u32,
        page_size: u32,
    },
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ExpectedReward {
    pub base_reward_expected: Uint128,
    pub rank_reward_expected: Uint128,
    pub bonus_expected: Uint128,
    pub total_expected: Uint128,
    pub token_id: String,
    pub rank: Option<u32>,
    pub reward_contract_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ExpectedCheck {
    pub base_reward_expected: Uint128,
    pub bonus_expected: Uint128,
    pub reward_contract_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Reward {
    pub base_reward: Uint128,
    pub bonus_reward: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct RankReward {
    pub rank_reward: Uint128,
    pub reward_contract_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ExpectedRewardResponse {
    pub expected_rewards: Vec<Vec<ExpectedReward>>,
}
