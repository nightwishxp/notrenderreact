use cosmwasm_std::{
    from_binary, to_binary, Api, BankMsg, Binary, Coin, Context, CosmosMsg, Env, Extern,
    HandleResponse, HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, Uint128,
    WasmMsg,
};

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg, Snip20Msg};
use crate::state::{config, config_read, State};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        count: msg.count,
        owner: deps.api.canonical_address(&env.message.sender)?,
        known_snip_20: vec![],
    };

    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Increment {} => try_increment(deps, env),
        HandleMsg::Reset { count } => try_reset(deps, env, count),
        HandleMsg::Register { reg_addr, reg_hash } => try_register(deps, env, reg_addr, reg_hash),
        HandleMsg::Receive {
            sender,
            from,
            amount,
            msg,
        } => try_receive(deps, env, sender, from, amount, msg),
        HandleMsg::Redeem {
            addr,
            hash,
            to,
            amount,
        } => try_redeem(deps, env, addr, hash, to, amount),
        HandleMsg::Fail {} => try_fail(),
