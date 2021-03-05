use uint::construct_uint;

use near_sdk::collections::LookupMap;
use near_sdk::{AccountId, Balance};

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

pub fn add_to_collection(c: &mut LookupMap<AccountId, Balance>, key: &String, amount: Balance) {
    let prev_amount = c.get(key).unwrap_or(0);
    c.insert(key, &(prev_amount + amount));
}
