use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Env;
use secret_toolkit::crypto::{sha_256, Prng};

use crate::utils::{create_hashed_password, ct_slice_compare};

pub const VIEWING_KEY_SIZE: usize = 32;
pub const VIEWING_KEY_PREFIX: &str = "api_key_";

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ViewingKey(pub String);

impl ViewingKey {
    pub fn check_viewing_key(&self, hashed_pw: &[u8]) -> bool {
        let mine_hashed = 