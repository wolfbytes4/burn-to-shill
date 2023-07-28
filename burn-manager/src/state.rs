use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::ContractInfo;
use cosmwasm_std::{Addr, CanonicalAddr, Uint128};
use secret_toolkit::{
    snip721::ViewerInfo,
    storage::{AppendStore, Item, Keymap},
};

pub static CONFIG_KEY: &[u8] = b"config";
pub static CONFIG_ITEM: Item<State> = Item::new(CONFIG_KEY);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub is_active: bool,
    pub burn_contracts: Vec<ContractInfo>,
}
