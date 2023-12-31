use crate::error::ContractError;
use crate::msg::{
    BurnInfoResponse, ExecuteMsg, ExpectedReward, ExpectedRewardResponse, HandleNftReceiveMsg,
    HandleReceiveMsg, History, HistoryFull, InstantiateMsg, QueryMsg, Rank, Reward,
    RewardsContractInfo,
};
use crate::rand::sha_256;
use crate::state::{
    State, ADMIN_VIEWING_KEY_ITEM, BURN_HISTORY_STORE, CONFIG_ITEM, HISTORY_STORE,
    PREFIX_REVOKED_PERMITS, RANK_STORE,
};
use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, CanonicalAddr, CosmosMsg, Deps,
    DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use secret_toolkit::{
    permit::{validate, Permit, RevokedPermits},
    snip20::{balance_query, set_viewing_key_msg, transfer_msg, Balance},
    snip721::{
        batch_burn_nft_msg, nft_dossier_query, register_receive_nft_msg, Burn, NftDossier,
        ViewerInfo,
    },
};

pub const BLOCK_SIZE: usize = 256;
///  Add function to get balance
/// Add message to burn

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    let prng_seed: Vec<u8> = sha_256(base64::encode(msg.entropy).as_bytes()).to_vec();
    let viewing_key = base64::encode(&prng_seed);

    // create initial state
    let state = State {
        viewing_key: Some(viewing_key),
        owner: info.sender.clone(),
        nft_contract: msg.nft_contract,
        reward_contracts: msg.reward_contracts,
        total_burned_amount: 0,
        is_active: true,
        trait_restriction: msg.trait_restriction,
        burn_counter_date: _env.block.time.seconds(),
    };

    //Save Contract state
    CONFIG_ITEM.save(deps.storage, &state)?;
    for rank in msg.ranks.iter() {
        RANK_STORE.insert(deps.storage, &rank.token_id, &rank)?;
    }

    let mut response_msgs: Vec<CosmosMsg> = Vec::new();

    deps.api
        .debug(&format!("Contract was initialized by {}", info.sender));

    let vk = state.viewing_key.unwrap();

    response_msgs.push(register_receive_nft_msg(
        _env.contract.code_hash,
        Some(true),
        None,
        BLOCK_SIZE,
        state.nft_contract.code_hash.clone(),
        state.nft_contract.address.to_string(),
    )?);

    response_msgs.push(set_viewing_key_msg(
        vk.to_string(),
        None,
        BLOCK_SIZE,
        state.nft_contract.code_hash,
        state.nft_contract.address.to_string(),
    )?);

    for reward_contract in state.reward_contracts.iter() {
        response_msgs.push(set_viewing_key_msg(
            vk.to_string(),
            None,
            BLOCK_SIZE,
            reward_contract.code_hash.to_string(),
            reward_contract.address.to_string(),
        )?);
    }

    Ok(Response::new().add_messages(response_msgs))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RevokePermit { permit_name } => {
            try_revoke_permit(deps, &info.sender, &permit_name)
        }
        ExecuteMsg::UpdateRewardContract { contracts } => {
            try_update_reward_contract(deps, &info.sender, contracts)
        }

        ExecuteMsg::UpdateRanks { ranks } => {
            try_update_ranks(deps, &info.sender, ranks)
        }
        ExecuteMsg::RemoveRewards {} => try_remove_rewards(deps, &info.sender),
        ExecuteMsg::BatchReceiveNft {
            from,
            token_ids,
            msg,
        } => try_batch_receive(deps, _env, &info.sender, &from, token_ids, msg),
        ExecuteMsg::Receive {
            sender,
            from,
            amount,
            msg,
        } => receive(deps, _env, &info.sender, &sender, &from, amount, msg),
        ExecuteMsg::SetViewingKey { key } => try_set_viewing_key(deps, _env, &info.sender, key),
        ExecuteMsg::SetActiveState { is_active } => {
            try_set_active_state(deps, _env, &info.sender, is_active)
        }
        ExecuteMsg::ResetBurnCounterDate {} => {
            try_reset_burn_counter_date(deps, _env, &info.sender)
        }
    }
}
fn receive(
    deps: DepsMut,
    _env: Env,
    info_sender: &Addr, //snip contract
    sender: &Addr,      //for snip 20 sender and from are the same. Wth??
    from: &Addr,        //user
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    deps.api.debug(&format!("Receive received"));
    let response_msgs: Vec<CosmosMsg> = Vec::new();
    let mut state = CONFIG_ITEM.load(deps.storage)?;
    if let Some(bin_msg) = msg {
        match from_binary(&bin_msg)? {
            HandleReceiveMsg::ReceiveRewards {} => {
                let reward_contract_index = state
                    .reward_contracts
                    .iter()
                    .position(|x| x.address == info_sender.to_string());

                if reward_contract_index.is_none() {
                    return Err(ContractError::CustomError {
                        val: info_sender.to_string()
                            + &" Address is not correct reward snip contract".to_string(),
                    });
                }
                let reward_contract = &mut state.reward_contracts[reward_contract_index.unwrap()];
                reward_contract.total_rewards += amount;

                CONFIG_ITEM.save(deps.storage, &state)?;
            }
        }
    } else {
        return Err(ContractError::CustomError {
            val: "data should be given".to_string(),
        });
    }

    Ok(Response::new().add_messages(response_msgs))
}
fn try_batch_receive(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    from: &Addr,
    token_ids: Vec<String>,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    deps.api.debug(&format!("Receive received"));
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();
    let mut state = CONFIG_ITEM.load(deps.storage)?;
    if !state.is_active {
        return Err(ContractError::CustomError {
            val: "You cannot perform this action right now".to_string(),
        });
    }

    if sender != &state.nft_contract.address {
        return Err(ContractError::CustomError {
            val: sender.to_string() + &" Address is not correct snip contract".to_string(),
        });
    }

    if let Some(bin_msg) = msg {
        match from_binary(&bin_msg)? {
            HandleNftReceiveMsg::ClaimBurnRewards {
                expected_rewards,
                message,
            } => {
                let history_store = HISTORY_STORE.add_suffix(from.to_string().as_bytes());
                let current_time = _env.block.time.seconds();
                let mut rewards_map = std::collections::HashMap::new();
                for reward_contract in state.reward_contracts.iter() {
                    let reward = {
                        Reward {
                            base_reward: Uint128::from(0u128),
                            bonus_reward: Uint128::from(0u128),
                        }
                    };
                    rewards_map.insert(reward_contract.name.to_string(), reward);
                }
                let mut index = 0;
                for token_id in token_ids.iter() {
                    let response =
                        get_estimated_rewards_mut(&token_id, &current_time, &state, &deps).unwrap();
                    for res in response.iter() {
                        if let Some(value) = rewards_map.get_mut(&res.reward_contract_name) {
                            if index == 0 {
                                value.bonus_reward = res.bonus_expected;
                            }

                            value.base_reward +=
                                res.base_reward_expected + res.rank_reward_expected;

                            let meta: NftDossier = nft_dossier_query(
                                deps.querier,
                                token_id.to_string(),
                                None,
                                None,
                                BLOCK_SIZE,
                                state.nft_contract.code_hash.clone(),
                                state.nft_contract.address.to_string(),
                            )?;
                            if state.trait_restriction.is_some() {
                                let trait_to_check = state.trait_restriction.as_ref().unwrap();
                                let restricted_trait = meta
                                    .public_metadata
                                    .as_ref()
                                    .unwrap()
                                    .extension
                                    .as_ref()
                                    .unwrap()
                                    .attributes
                                    .as_ref()
                                    .unwrap()
                                    .iter()
                                    .find(|&x| x.trait_type == Some(trait_to_check.to_string()));
                                if restricted_trait.is_none() {
                                    return Err(ContractError::CustomError {
                                        val: "This NFT does not meet the requirements".to_string(),
                                    });
                                }
                            }
                            let history_rewards = if index == 0 {
                                value.bonus_reward + value.base_reward
                            } else {
                                value.base_reward
                            };
                            let claim_history: History = {
                                History {
                                    token_id: token_id.to_string(),
                                    date: current_time,
                                    rewards: history_rewards,
                                    message: message.to_string(),
                                }
                            };
                            history_store.push(deps.storage, &claim_history)?;
                            let full_history: HistoryFull = {
                                HistoryFull {
                                    date: current_time,
                                    token_id: token_id.to_string(),
                                    meta_data: meta.public_metadata.unwrap(),
                                    message: message.to_string(),
                                }
                            };
                            BURN_HISTORY_STORE.push(deps.storage, &full_history);
                            state.total_burned_amount += 1;
                            index = index + 1;
                        }
                    }
                }

                for expected_reward in expected_rewards.iter() {
                    if let Some(value) = rewards_map.get_mut(&expected_reward.reward_contract_name)
                    {
                        if value.base_reward >= expected_reward.base_reward_expected
                            && value.bonus_reward >= expected_reward.bonus_expected
                        {
                            let rewards_to_claim = value.base_reward + value.bonus_reward;
                            let reward_contract_index =
                                state.reward_contracts.iter().position(|x| {
                                    x.name == expected_reward.reward_contract_name.to_string()
                                });
                            let reward_contract =
                                &mut state.reward_contracts[reward_contract_index.unwrap()];

                            if rewards_to_claim < reward_contract.total_rewards {
                                //claim rewards
                                if value.bonus_reward > Uint128::from(0u128) {
                                    state.burn_counter_date = current_time;
                                }

                                reward_contract.total_rewards -= rewards_to_claim;

                                let cosmos_msg = transfer_msg(
                                    from.to_string(),
                                    rewards_to_claim,
                                    None,
                                    None,
                                    BLOCK_SIZE,
                                    reward_contract.code_hash.to_string(),
                                    reward_contract.address.to_string(),
                                )?;

                                response_msgs.push(cosmos_msg);
                            } else {
                                return Err(ContractError::CustomError {
                                    val: "Not enough rewards left".to_string(),
                                });
                            }
                        } else {
                            return Err(ContractError::CustomError {
                                val: "Actual reward less than Expected reward".to_string(),
                            });
                        }
                    } else {
                        return Err(ContractError::CustomError {
                            val: "Reward not in map".to_string(),
                        });
                    }
                }

                CONFIG_ITEM.save(deps.storage, &state)?;

                let mut burns: Vec<Burn> = Vec::new();
                burns.push(Burn {
                    token_ids: token_ids.clone(),
                    memo: None,
                });

                let cosmos_batch_msg = batch_burn_nft_msg(
                    burns,
                    None,
                    BLOCK_SIZE,
                    state.nft_contract.code_hash.clone(),
                    state.nft_contract.address.to_string(),
                )?;
                response_msgs.push(cosmos_batch_msg);
            }
        }
    } else {
        return Err(ContractError::CustomError {
            val: "data should be given".to_string(),
        });
    }

    Ok(Response::new().add_messages(response_msgs))
}

fn try_revoke_permit(
    deps: DepsMut,
    sender: &Addr,
    permit_name: &str,
) -> Result<Response, ContractError> {
    RevokedPermits::revoke_permit(
        deps.storage,
        PREFIX_REVOKED_PERMITS,
        &sender.to_string(),
        permit_name,
    );

    Ok(Response::default())
}

fn try_update_reward_contract(
    deps: DepsMut,
    sender: &Addr,
    contracts: Vec<RewardsContractInfo>,
) -> Result<Response, ContractError> {
    let mut state = CONFIG_ITEM.load(deps.storage)?;
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();

    if sender.clone() != state.owner {
        return Err(ContractError::CustomError {
            val: "You don't have the permissions to execute this command".to_string(),
        });
    }

    for reward_contract in state.reward_contracts.iter() {
        if reward_contract.total_rewards != Uint128::from(0u128) {
            return Err(ContractError::CustomError {
                val: "Clear out rewards first before updating".to_string(),
            });
        }
    }

    for reward_contract in contracts.iter() {
        response_msgs.push(set_viewing_key_msg(
            state.viewing_key.clone().unwrap().to_string(),
            None,
            BLOCK_SIZE,
            reward_contract.code_hash.to_string(),
            reward_contract.address.to_string(),
        )?);
    }

    state.reward_contracts = contracts;
    CONFIG_ITEM.save(deps.storage, &state)?;

    Ok(Response::new().add_messages(response_msgs))
}

fn try_update_ranks(
    deps: DepsMut,
    sender: &Addr,
    ranks: Vec<Rank>,
) -> Result<Response, ContractError> {
    let state = CONFIG_ITEM.load(deps.storage)?;
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();

    if sender.clone() != state.owner {
        return Err(ContractError::CustomError {
            val: "You don't have the permissions to execute this command".to_string(),
        });
    }

    for rank in ranks.iter() {
        RANK_STORE.insert(deps.storage, &rank.token_id, &rank)?;
    } 
 
    Ok(Response::default())
}

fn try_remove_rewards(deps: DepsMut, sender: &Addr) -> Result<Response, ContractError> {
    let mut state = CONFIG_ITEM.load(deps.storage)?;
    let mut response_msgs: Vec<CosmosMsg> = Vec::new();

    if sender.clone() != state.owner {
        return Err(ContractError::CustomError {
            val: "You don't have the permissions to execute this command".to_string(),
        });
    }

    for reward_contract in state.reward_contracts.iter_mut() {
        let cosmos_msg = transfer_msg(
            sender.to_string(),
            reward_contract.total_rewards.clone(),
            None,
            None,
            BLOCK_SIZE,
            reward_contract.code_hash.to_string(),
            reward_contract.address.to_string(),
        )?;
        response_msgs.push(cosmos_msg);

        reward_contract.total_rewards = Uint128::from(0u128);
    }
    CONFIG_ITEM.save(deps.storage, &state)?;
    Ok(Response::new().add_messages(response_msgs))
}

pub fn try_set_viewing_key(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    key: String,
) -> Result<Response, ContractError> {
    let state = CONFIG_ITEM.load(deps.storage)?;
    let prng_seed: Vec<u8> = sha_256(base64::encode(key).as_bytes()).to_vec();
    let viewing_key = base64::encode(&prng_seed);

    let vk: ViewerInfo = {
        ViewerInfo {
            address: sender.to_string(),
            viewing_key: viewing_key,
        }
    };

    if sender.clone() == state.owner {
        ADMIN_VIEWING_KEY_ITEM.save(deps.storage, &vk)?;
    } else {
        return Err(ContractError::CustomError {
            val: "You don't have the permissions to execute this command".to_string(),
        });
    }
    Ok(Response::default())
}

pub fn try_set_active_state(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
    is_active: bool,
) -> Result<Response, ContractError> {
    let mut state = CONFIG_ITEM.load(deps.storage)?;

    if sender.clone() != state.owner {
        return Err(ContractError::CustomError {
            val: "You don't have the permissions to execute this command".to_string(),
        });
    }
    state.is_active = is_active;

    CONFIG_ITEM.save(deps.storage, &state)?;

    Ok(Response::default())
}

pub fn try_reset_burn_counter_date(
    deps: DepsMut,
    _env: Env,
    sender: &Addr,
) -> Result<Response, ContractError> {
    let mut state = CONFIG_ITEM.load(deps.storage)?;

    if sender.clone() != state.owner {
        return Err(ContractError::CustomError {
            val: "You don't have the permissions to execute this command".to_string(),
        });
    }
    state.burn_counter_date = _env.block.time.seconds();

    CONFIG_ITEM.save(deps.storage, &state)?;

    Ok(Response::default())
}

fn get_estimated_rewards(
    token_id: &String,
    current_time: &u64,
    state: &State,
    deps: Deps,
) -> StdResult<Vec<ExpectedReward>> {
    let mut expected_rewards: Vec<ExpectedReward> = Vec::new();

    for reward_contract in state.reward_contracts.iter() {
        let mut bonus_reward = Uint128::from(0u128);
        let mut rank_reward = Uint128::from(0u128);
        let mut token_rank = None;

        if reward_contract.burn_type == "rank" {
            let rank_entity: Option<Rank> = RANK_STORE.get(deps.storage, &token_id);
            //.ok_or_else(|| StdError::generic_err("Rank pool doesn't have token"))?;
            if rank_entity.is_some() {
                token_rank = Some(rank_entity.as_ref().unwrap().rank);
                let r_reward = rank_entity
                    .as_ref()
                    .unwrap()
                    .rank_rewards
                    .iter()
                    .find(|&x| x.reward_contract_name == reward_contract.name);
                if r_reward.is_some() {
                    rank_reward = r_reward.unwrap().rank_reward;
                }
            }
        }

        if reward_contract.bonus_hourly > Uint128::from(0u128) {
            if current_time > &state.burn_counter_date {
                let duration_seconds = current_time - state.burn_counter_date;
                let hours = duration_seconds / 3600;
                bonus_reward = Uint128::from(hours) * reward_contract.bonus_hourly;
            }
        }

        let expected_reward: ExpectedReward = {
            ExpectedReward {
                base_reward_expected: reward_contract.base_reward,
                rank_reward_expected: rank_reward,
                bonus_expected: bonus_reward,
                total_expected: bonus_reward + rank_reward + reward_contract.base_reward,
                rank: token_rank,
                token_id: token_id.to_string(),
                reward_contract_name: reward_contract.name.to_string(),
            }
        };

        expected_rewards.push(expected_reward);
    }
    return Ok(expected_rewards);
}
fn get_estimated_rewards_mut(
    token_id: &String,
    current_time: &u64,
    state: &State,
    deps: &DepsMut,
) -> StdResult<Vec<ExpectedReward>> {
    let mut expected_rewards: Vec<ExpectedReward> = Vec::new();

    for reward_contract in state.reward_contracts.iter() {
        let mut bonus_reward = Uint128::from(0u128);
        let mut rank_reward = Uint128::from(0u128);
        let mut token_rank = None;

        if reward_contract.burn_type == "rank" {
            let rank_entity: Option<Rank> = RANK_STORE.get(deps.storage, &token_id);
            //.ok_or_else(|| StdError::generic_err("Rank pool doesn't have token"))?;
            if rank_entity.is_some() {
                token_rank = Some(rank_entity.as_ref().unwrap().rank);
                let r_reward = rank_entity
                    .as_ref()
                    .unwrap()
                    .rank_rewards
                    .iter()
                    .find(|&x| x.reward_contract_name == reward_contract.name);
                if r_reward.is_some() {
                    rank_reward = r_reward.unwrap().rank_reward;
                }
            }
        }

        if reward_contract.bonus_hourly > Uint128::from(0u128) {
            if current_time > &state.burn_counter_date {
                let duration_seconds = current_time - state.burn_counter_date;
                let hours = duration_seconds / 3600;
                bonus_reward = Uint128::from(hours) * reward_contract.bonus_hourly;
            }
        }

        let expected_reward: ExpectedReward = {
            ExpectedReward {
                base_reward_expected: reward_contract.base_reward,
                rank_reward_expected: rank_reward,
                bonus_expected: bonus_reward,
                total_expected: bonus_reward + rank_reward + reward_contract.base_reward,
                rank: token_rank,
                token_id: token_id.to_string(),
                reward_contract_name: reward_contract.name.to_string(),
            }
        };
        expected_rewards.push(expected_reward);
    }
    return Ok(expected_rewards);
}
//TODO: ADD QUERY FOR FULL HISTORY
#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBurnInfo {} => to_binary(&query_burn_info(deps)?),
        QueryMsg::GetExpectedRewards { token_ids } => {
            to_binary(&query_expected_rewards(deps, _env, token_ids)?)
        }
        QueryMsg::GetNumUserHistory { permit } => {
            to_binary(&query_num_user_history(deps, _env, permit)?)
        }
        QueryMsg::GetUserHistory {
            permit,
            start_page,
            page_size,
        } => to_binary(&query_user_history(
            deps, _env, permit, start_page, page_size,
        )?),
        QueryMsg::GetNumFullHistory {} => to_binary(&query_num_full_history(deps, _env)?),
        QueryMsg::GetFullHistory {
            start_page,
            page_size,
        } => to_binary(&query_full_history(deps, _env, start_page, page_size)?),
        QueryMsg::GetRewardBalance { viewer } => {
            to_binary(&query_reward_balance(deps, _env, viewer)?)
        }
    }
}

fn query_burn_info(deps: Deps) -> StdResult<BurnInfoResponse> {
    let state = CONFIG_ITEM.load(deps.storage)?;
    Ok(BurnInfoResponse {
        total_burned_amount: state.total_burned_amount,
        nft_contract: state.nft_contract,
        reward_contracts: state.reward_contracts,
        trait_restriction: state.trait_restriction,
        is_active: state.is_active,
        burn_counter_date: state.burn_counter_date,
    })
}

fn query_expected_rewards(
    deps: Deps,
    env: Env,
    token_ids: Vec<String>,
) -> StdResult<ExpectedRewardResponse> {
    let state = CONFIG_ITEM.load(deps.storage)?;
    let current_time = env.block.time.seconds();
    let mut estimated_rewards: Vec<Vec<ExpectedReward>> = Vec::new();
    for token_id in token_ids.iter() {
        let response = get_estimated_rewards(&token_id, &current_time, &state, deps);
        estimated_rewards.push(response.unwrap());
    }

    Ok(ExpectedRewardResponse {
        expected_rewards: estimated_rewards,
    })
}

fn query_num_user_history(deps: Deps, env: Env, permit: Permit) -> StdResult<u32> {
    let user_raw = get_querier(deps, permit, env.contract.address)?;
    let history_store = HISTORY_STORE.add_suffix(&user_raw);
    let num = history_store.get_len(deps.storage)?;
    Ok(num)
}

fn query_user_history(
    deps: Deps,
    env: Env,
    permit: Permit,
    start_page: u32,
    page_size: u32,
) -> StdResult<Vec<History>> {
    let user_raw = get_querier(deps, permit, env.contract.address)?;
    let history_store = HISTORY_STORE.add_suffix(&user_raw);
    let history = history_store.paging(deps.storage, start_page, page_size)?;
    Ok(history)
}

fn query_num_full_history(deps: Deps, env: Env) -> StdResult<u32> {
    let num = BURN_HISTORY_STORE.get_len(deps.storage)?;
    Ok(num)
}

fn query_full_history(
    deps: Deps,
    env: Env,
    start_page: u32,
    page_size: u32,
) -> StdResult<Vec<HistoryFull>> {
    let history = BURN_HISTORY_STORE.paging(deps.storage, start_page, page_size)?;
    Ok(history)
}

fn query_reward_balance(deps: Deps, env: Env, viewer: ViewerInfo) -> StdResult<Vec<Balance>> {
    check_admin_key(deps, viewer)?;
    let state = CONFIG_ITEM.load(deps.storage)?;
    let mut balances: Vec<Balance> = Vec::new();
    for reward_contract in state.reward_contracts.iter() {
        let balance = balance_query(
            deps.querier,
            env.contract.address.to_string(),
            state.viewing_key.clone().unwrap(),
            BLOCK_SIZE,
            reward_contract.code_hash.to_string(),
            reward_contract.address.to_string(),
        );
        balances.push(balance.unwrap())
    }
    Ok(balances)
}

fn check_admin_key(deps: Deps, viewer: ViewerInfo) -> StdResult<()> {
    let admin_viewing_key = ADMIN_VIEWING_KEY_ITEM.load(deps.storage)?;
    let prng_seed: Vec<u8> = sha_256(base64::encode(viewer.viewing_key).as_bytes()).to_vec();
    let vk = base64::encode(&prng_seed);

    if vk != admin_viewing_key.viewing_key || viewer.address != admin_viewing_key.address {
        return Err(StdError::generic_err(
            "Wrong viewing key for this address or viewing key not set",
        ));
    }

    return Ok(());
}

fn get_querier(deps: Deps, permit: Permit, contract_address: Addr) -> StdResult<CanonicalAddr> {
    if let pmt = permit {
        let querier = deps.api.addr_canonicalize(&validate(
            deps,
            PREFIX_REVOKED_PERMITS,
            &pmt,
            contract_address.to_string(),
            None,
        )?)?;
        if !pmt.check_permission(&secret_toolkit::permit::TokenPermissions::Owner) {
            return Err(StdError::generic_err(format!(
                "Owner permission is required for history queries, got permissions {:?}",
                pmt.params.permissions
            )));
        }
        return Ok(querier);
    }
    return Err(StdError::generic_err("Unauthorized"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::ContractInfo;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn rewards_calc() {
        let mut deps = mock_dependencies();

        let mut expected = Uint128::from(650000000u128);

        let current_time = 1686675096;
        let state: State = {
            State {
                owner: Addr::unchecked(""),
                is_active: true,
                nft_contract: {
                    ContractInfo {
                        code_hash: "".to_string(),
                        address: Addr::unchecked(""),
                        name: "".to_string(),
                    }
                },
                reward_contracts: vec![RewardsContractInfo {
                    code_hash: "".to_string(),
                    address: Addr::unchecked(""),
                    base_reward: Uint128::from(50000000u128),
                    bonus_hourly: Uint128::from(25000000u128),
                    name: "shill".to_string(),
                    burn_type: "normal".to_string(),
                    total_rewards: Uint128::from(10000000000000u128),
                }],
                viewing_key: None,
                total_burned_amount: 200u32,
                trait_restriction: None,
                burn_counter_date: 1686588696,
            }
        };

        let x = get_estimated_rewards(&"1".to_string(), &current_time, &state, deps.as_ref());
        println!("{:?}", x);
        println!("{:?}", "HIII");

        for value in x.unwrap().iter() {
            assert_eq!(value.total_expected, expected);
        }
        //TODO TEST RANK BASED REWARDS

        // staked.staked_amount = Uint128::from(100u128);
        // expected = Uint128::from(1369499999u128);
        // let y = get_estimated_rewards(&staked, &current_time, &state);
        // assert_eq!(y.unwrap(), expected);
    }
}
