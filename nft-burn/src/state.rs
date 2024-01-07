use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{ContractInfo, History, HistoryFull, Rank, RewardsContractInfo};
use cosmwasm_std::{Addr};
use secret_toolkit::{
    snip721::ViewerInfo,
    storage::{AppendStore, Item, Keymap},
};

pub static CONFIG_KEY: &[u8] = b"config";
pub const PREFIX_REVOKED_PERMITS: &str = "revoke";
pub const HISTORY_KEY: &[u8] = b"history";
pub const BURN_HISTORY_KEY: &[u8] = b"burn_history";
pub const ADMIN_VIEWING_KEY: &[u8] = b"admin_viewing_key";
pub const RANK_KEY: &[u8] = b"rank_key";

pub static CONFIG_ITEM: Item<State> = Item::new(CONFIG_KEY);
pub static HISTORY_STORE: AppendStore<History> = AppendStore::new(HISTORY_KEY);
pub static BURN_HISTORY_STORE: AppendStore<HistoryFull> = AppendStore::new(BURN_HISTORY_KEY);
pub static ADMIN_VIEWING_KEY_ITEM: Item<ViewerInfo> = Item::new(ADMIN_VIEWING_KEY);
pub static RANK_STORE: Keymap<String, Rank> = Keymap::new(RANK_KEY);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub is_active: bool,
    pub nft_contract: ContractInfo,
    pub reward_contracts: Vec<RewardsContractInfo>,
    pub viewing_key: Option<String>,
    pub total_burned_amount: u32,
    pub trait_restriction: Option<String>,
    pub burn_counter_date: u64,
}
