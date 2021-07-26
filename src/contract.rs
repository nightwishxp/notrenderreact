/// This contract implements SNIP-20 standard:
/// https://github.com/SecretFoundation/SNIPs/blob/master/SNIP-20.md
use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg, Env, Extern,
    HandleResponse, HumanAddr, InitResponse, Querier, QueryResult, ReadonlyStorage, StdError,
    StdResult, Storage, Uint128,
};
use secret_toolkit::crypto::sha_256;

use crate::msg::{
    space_pad, ContractStatusLevel, HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg,
    ResponseStatus::Success,
};
use crate::receiver::Snip20ReceiveMsg;
use crate::state::{
    get_receiver_hash, get_transfers, read_allowance, read_viewing_key, set_receiver_hash,
    store_transfer, write_allowance, write_viewing_key, Balances, Config, Constants,
    ReadonlyBalances, ReadonlyConfig,
};
use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};

/// We make sure that responses from `handle` are padded to a multiple of this size.
pub const RESPONSE_BLOCK_SIZE: usize = 256;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let init_config = msg.config();
    let mut total_supply: u128 = 0;
    {
        let mut balances = Balances::from_storage(&mut deps.storage);
        let initial_balances = msg.initial_balances.unwrap