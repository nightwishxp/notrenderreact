
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

fn append_tx<S: Storage>(
    store: &mut S,
    tx: StoredTx,
    for_address: &CanonicalAddr,
) -> StdResult<()> {
    let mut store = PrefixedStorage::multilevel(&[PREFIX_TXS, for_address.as_slice()], store);
    let mut store = AppendStoreMut::attach_or_create(&mut store)?;
    store.push(&tx)
}

pub fn get_transfers<A: Api, S: ReadonlyStorage>(
    api: &A,
    storage: &S,
    for_address: &CanonicalAddr,
    page: u32,
    page_size: u32,
) -> StdResult<Vec<Tx>> {
    let store = ReadonlyPrefixedStorage::multilevel(&[PREFIX_TXS, for_address.as_slice()], storage);

    // Try to access the storage of txs for the account.
    // If it doesn't exist yet, return an empty list of transfers.
    let store = if let Some(result) = AppendStore::<StoredTx, _>::attach(&store) {
        result?
    } else {
        return Ok(vec![]);
    };

    // Take `page_size` txs starting from the latest tx, potentially skipping `page * page_size`
    // txs from the start.
    let tx_iter = store
        .iter()
        .rev()
        .skip((page * page_size) as _)
        .take(page_size as _);
    // The `and_then` here flattens the `StdResult<StdResult<Tx>>` to an `StdResult<Tx>`
    let txs: StdResult<Vec<Tx>> = tx_iter
        .map(|tx| tx.map(|tx| tx.into_humanized(api)).and_then(|x| x))
        .collect();
    txs
}

// Config

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct Constants {
    pub name: String,
    pub admin: HumanAddr,
    pub symbol: String,
    pub decimals: u8,
    pub prng_seed: Vec<u8>,
    // privacy configuration
    pub total_supply_is_public: bool,
}

pub struct ReadonlyConfig<'a, S: ReadonlyStorage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: ReadonlyStorage> ReadonlyConfig<'a, S> {
    pub fn from_storage(storage: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(PREFIX_CONFIG, storage),
        }
    }

    fn as_readonly(&self) -> ReadonlyConfigImpl<ReadonlyPrefixedStorage<S>> {
        ReadonlyConfigImpl(&self.storage)
    }

    pub fn constants(&self) -> StdResult<Constants> {
        self.as_readonly().constants()
    }

    pub fn total_supply(&self) -> u128 {
        self.as_readonly().total_supply()
    }

    pub fn contract_status(&self) -> ContractStatusLevel {
        self.as_readonly().contract_status()
    }

    pub fn tx_count(&self) -> u64 {
        self.as_readonly().tx_count()
    }
}

fn set_bin_data<T: Serialize, S: Storage>(storage: &mut S, key: &[u8], data: &T) -> StdResult<()> {
    let bin_data =
        bincode2::serialize(&data).map_err(|e| StdError::serialize_err(type_name::<T>(), e))?;

    storage.set(key, &bin_data);
    Ok(())
}

fn get_bin_data<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    let bin_data = storage.get(key);

    match bin_data {
        None => Err(StdError::not_found("Key not found in storage")),
        Some(bin_data) => Ok(bincode2::deserialize::<T>(&bin_data)
            .map_err(|e| StdError::serialize_err(type_name::<T>(), e))?),
    }
}

pub struct Config<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> Config<'a, S> {
    pub fn from_storage(storage: &'a mut S) -> Self {
        Self {
            storage: PrefixedStorage::new(PREFIX_CONFIG, storage),
        }
    }

    fn as_readonly(&self) -> ReadonlyConfigImpl<PrefixedStorage<S>> {
        ReadonlyConfigImpl(&self.storage)
    }

    pub fn constants(&self) -> StdResult<Constants> {
        self.as_readonly().constants()
    }

    pub fn set_constants(&mut self, constants: &Constants) -> StdResult<()> {
        set_bin_data(&mut self.storage, KEY_CONSTANTS, constants)
    }

    pub fn total_supply(&self) -> u128 {
        self.as_readonly().total_supply()
    }

    pub fn set_total_supply(&mut self, supply: u128) {
        self.storage.set(KEY_TOTAL_SUPPLY, &supply.to_be_bytes());
    }

    pub fn contract_status(&self) -> ContractStatusLevel {
        self.as_readonly().contract_status()
    }

    pub fn set_contract_status(&mut self, status: ContractStatusLevel) {
        let status_u8 = status_level_to_u8(status);
        self.storage
            .set(KEY_CONTRACT_STATUS, &status_u8.to_be_bytes());
    }

    pub fn tx_count(&self) -> u64 {
        self.as_readonly().tx_count()
    }

    pub fn set_tx_count(&mut self, count: u64) -> StdResult<()> {
        set_bin_data(&mut self.storage, KEY_TX_COUNT, &count)
    }
}

/// This struct refactors out the readonly methods that we need for `Config` and `ReadonlyConfig`
/// in a way that is generic over their mutability.
///
/// This was the only way to prevent code duplication of these methods because of the way
/// that `ReadonlyPrefixedStorage` and `PrefixedStorage` are implemented in `cosmwasm-std`
struct ReadonlyConfigImpl<'a, S: ReadonlyStorage>(&'a S);

impl<'a, S: ReadonlyStorage> ReadonlyConfigImpl<'a, S> {
    fn constants(&self) -> StdResult<Constants> {
        let consts_bytes = self
            .0
            .get(KEY_CONSTANTS)
            .ok_or_else(|| StdError::generic_err("no constants stored in configuration"))?;
        bincode2::deserialize::<Constants>(&consts_bytes)
            .map_err(|e| StdError::serialize_err(type_name::<Constants>(), e))
    }

    fn total_supply(&self) -> u128 {
        let supply_bytes = self
            .0
            .get(KEY_TOTAL_SUPPLY)
            .expect("no total supply stored in config");
        // This unwrap is ok because we know we stored things correctly
        slice_to_u128(&supply_bytes).unwrap()
    }

    fn contract_status(&self) -> ContractStatusLevel {
        let supply_bytes = self
            .0
            .get(KEY_CONTRACT_STATUS)
            .expect("no contract status stored in config");

        // These unwraps are ok because we know we stored things correctly
        let status = slice_to_u8(&supply_bytes).unwrap();