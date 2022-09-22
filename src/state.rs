
use std::any::type_name;
use std::convert::TryFrom;

use cosmwasm_std::{
    Api, CanonicalAddr, Coin, HumanAddr, ReadonlyStorage, StdError, StdResult, Storage, Uint128,
};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use secret_toolkit::storage::{AppendStore, AppendStoreMut, TypedStore, TypedStoreMut};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{status_level_to_u8, u8_to_status_level, ContractStatusLevel};
use crate::viewing_key::ViewingKey;
use serde::de::DeserializeOwned;

pub static CONFIG_KEY: &[u8] = b"config";
pub const PREFIX_TXS: &[u8] = b"transfers";

pub const KEY_CONSTANTS: &[u8] = b"constants";
pub const KEY_TOTAL_SUPPLY: &[u8] = b"total_supply";
pub const KEY_CONTRACT_STATUS: &[u8] = b"contract_status";
pub const KEY_TX_COUNT: &[u8] = b"tx-count";

pub const PREFIX_CONFIG: &[u8] = b"config";
pub const PREFIX_BALANCES: &[u8] = b"balances";
pub const PREFIX_ALLOWANCES: &[u8] = b"allowances";
pub const PREFIX_VIEW_KEY: &[u8] = b"viewingkey";
pub const PREFIX_RECEIVERS: &[u8] = b"receivers";

// Note that id is a globally incrementing counter.
// Since it's 64 bits long, even at 50 tx/s it would take
// over 11 billion years for it to rollback. I'm pretty sure
// we'll have bigger issues by then.
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Tx {
    pub id: u64,
    pub from: HumanAddr,
    pub sender: HumanAddr,
    pub receiver: HumanAddr,
    pub coins: Coin,
}

impl Tx {
    pub fn into_stored<A: Api>(self, api: &A) -> StdResult<StoredTx> {
        let tx = StoredTx {
            id: self.id,
            from: api.canonical_address(&self.from)?,
            sender: api.canonical_address(&self.sender)?,
            receiver: api.canonical_address(&self.receiver)?,
            coins: self.coins,
        };
        Ok(tx)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoredTx {
    pub id: u64,
    pub from: CanonicalAddr,
    pub sender: CanonicalAddr,
    pub receiver: CanonicalAddr,
    pub coins: Coin,
}

impl StoredTx {
    pub fn into_humanized<A: Api>(self, api: &A) -> StdResult<Tx> {
        let tx = Tx {
            id: self.id,
            from: api.human_address(&self.from)?,
            sender: api.human_address(&self.sender)?,
            receiver: api.human_address(&self.receiver)?,
            coins: self.coins,
        };
        Ok(tx)
    }
}

pub fn store_transfer<S: Storage>(
    store: &mut S,
    owner: &CanonicalAddr,
    sender: &CanonicalAddr,
    receiver: &CanonicalAddr,
    amount: Uint128,
    denom: String,
) -> StdResult<()> {
    let mut config = Config::from_storage(store);
    let id = config.tx_count() + 1;
    config.set_tx_count(id)?;

    let coins = Coin { denom, amount };
    let tx = StoredTx {
        id,
        from: owner.clone(),
        sender: sender.clone(),
        receiver: receiver.clone(),
        coins,
    };

    if owner != sender {
        append_tx(store, tx.clone(), &owner)?;
    }
    append_tx(store, tx.clone(), &sender)?;
    append_tx(store, tx, &receiver)?;

    Ok(())
}