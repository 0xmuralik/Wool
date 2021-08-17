use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Coin;
use cw_storage_plus::Map;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Pool {
    pub id: String,
    pub name: String,
    pub coins: Vec<Coin>,
}

pub const POOLS: Map<&str, Pool> = Map::new("pool");
