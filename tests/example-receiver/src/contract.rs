use cosmwasm_std::{
    from_binary, to_binary, Api, BankMsg, Binary, Coin, Context, CosmosMsg, Env, Extern,
    HandleResponse, HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, Uint128,
    WasmMsg,
};

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg, Snip20Msg};
use crate::state::{config, config_read, State};

pub fn init<S: Storage, A: Api, Q: Querier>