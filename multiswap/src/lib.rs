use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, near_bindgen, serde_json, AccountId, Balance, PanicOnDefault};
use uint::construct_uint;

use crate::pool::{add_to_collection, ext_fungible_token, Pool, GAS_FOR_FT_TRANSFER, NO_DEPOSIT};
use std::convert::TryInto;

mod pool;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
struct Contract {
    pools: Vector<Pool>,
    /// Balances of liquidity adding in progress in the form of "<token_id>:<account_id>".
    liquidity_amounts: LookupMap<String, Balance>,
}

pub fn token_account_key(token_id: &AccountId, account_id: &AccountId) -> String {
    format!("{}:{}", token_id, account_id)
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_IS_INITIALIZED");
        Self {
            pools: Vector::new(b"p".to_vec()),
            liquidity_amounts: LookupMap::new(b"l".to_vec()),
        }
    }

    /// Adds new pool with given tokens and give fee.
    pub fn add_pool(&mut self, tokens: Vec<ValidAccountId>, fee: u32) -> u32 {
        let id = self.pools.len() as u32;
        self.pools.push(&Pool::new(id, tokens, fee));
        id
    }

    /// Record deposit of some number of tokens to this contract.
    fn deposit(&mut self, sender_id: &AccountId, token_id: &AccountId, amount: Balance) {
        add_to_collection(
            &mut self.liquidity_amounts,
            &token_account_key(token_id, sender_id),
            amount,
        );
    }

    pub fn add_liquidity(&mut self, pool_id: u64) {
        let sender_id = env::predecessor_account_id();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        let mut amounts = Vec::new();
        // TODO: Handle storage.
        for token_id in pool.tokens() {
            amounts.push(
                self.liquidity_amounts
                    .remove(&token_account_key(token_id, &sender_id))
                    .expect("ERR_MISSING_TOKEN"),
            );
        }
        pool.add_liquidity(sender_id, amounts);
        self.pools.replace(pool_id, &pool);
    }

    /// Remove liquidity from the pool into general pool of liquidity.
    pub fn remove_liquidity(&mut self, pool_id: u64, shares: U128, min_amounts: Vec<U128>) {
        let sender_id = env::predecessor_account_id();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        let amounts = pool.remove_liquidity(
            &sender_id,
            shares.into(),
            min_amounts
                .into_iter()
                .map(|amount| amount.into())
                .collect(),
        );
        self.pools.replace(pool_id, &pool);
        let tokens = pool.tokens();
        for i in 0..tokens.len() {
            add_to_collection(
                &mut self.liquidity_amounts,
                &token_account_key(&tokens[i], &sender_id),
                amounts[i],
            );
        }
    }

    /// Withdraws given token from the free funds of given user.
    pub fn withdraw(&mut self, token_id: ValidAccountId, amount: U128) {
        let amount: u128 = amount.into();
        let sender_id = env::predecessor_account_id();
        let key = token_account_key(token_id.as_ref(), &sender_id);
        let available_amount = self.liquidity_amounts.get(&key).expect("ERR_MISSING_TOKEN");
        assert!(available_amount >= amount, "ERR_NOT_ENOUGH");
        // TODO: Handle storage.
        if available_amount == amount {
            self.liquidity_amounts.remove(&key);
        } else {
            self.liquidity_amounts
                .insert(&key, &(available_amount - amount));
        }
        ext_fungible_token::ft_transfer(
            sender_id.try_into().unwrap(),
            amount.into(),
            None,
            token_id.as_ref(),
            NO_DEPOSIT,
            GAS_FOR_FT_TRANSFER,
        );
    }

    pub fn get_return(
        &self,
        pool_id: u64,
        token_in: ValidAccountId,
        amount_in: U128,
        token_out: ValidAccountId,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        pool.get_return(token_in, amount_in.into(), token_out)
            .into()
    }
}

trait FungibleTokenReceiver {
    /// Called by fungible token contract after `ft_transfer_call` was initiated by
    /// `sender_id` of the given `amount` with the transfer message given in `msg` field.
    /// The `amount` of tokens were already transferred to this contract account and ready to be used.
    ///
    /// The method must return the amount of tokens that are *not* used/accepted by this contract from the transferred
    /// amount. Examples:
    /// - The transferred amount was `500`, the contract completely takes it and must return `0`.
    /// - The transferred amount was `500`, but this transfer call only needs `450` for the action passed in the `msg`
    ///   field, then the method must return `50`.
    /// - The transferred amount was `500`, but the action in `msg` field has expired and the transfer must be
    ///   cancelled. The method must return `500` or panic.
    ///
    /// Arguments:
    /// - `sender_id` - the account ID that initiated the transfer.
    /// - `amount` - the amount of tokens that were transferred to this account in a decimal string representation.
    /// - `msg` - a string message that was passed with this transfer call.
    ///
    /// Returns the amount of unused tokens that should be returned to sender, in a decimal string representation.
    fn ft_on_transfer(&mut self, sender_id: ValidAccountId, amount: U128, msg: String) -> U128;
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(&mut self, sender_id: ValidAccountId, amount: U128, msg: String) -> U128 {
        // Message structure:
        //  - deposit
        //  - swap:pool_id:token_out:min_amount_out
        let token_in = env::predecessor_account_id();
        if msg == "deposit" {
            self.deposit(sender_id.as_ref(), &token_in, amount.into());
        } else {
            let pieces: Vec<&str> = msg.split(":").collect();
            assert_eq!(pieces.len(), 4);
            assert_eq!(pieces[0], "swap");
            let pool_id = serde_json::from_str::<u64>(pieces[1]).expect("ERR_MSG_POOL_ID");
            let token_out = pieces[2].to_string();
            let min_amount_out = serde_json::from_str::<U128>(pieces[3]).expect("ERR_MSG_POOL_ID");
            let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
            let amount_out = pool.swap(
                sender_id.as_ref(),
                &token_in,
                amount.into(),
                &token_out,
                min_amount_out.into(),
            );
            self.pools.replace(pool_id, &pool);
            env::log(
                format!(
                    "Swapped {} {} for {} {}",
                    u128::from(amount),
                    token_in,
                    amount_out,
                    token_out
                )
                .as_bytes(),
            );
        }
        amount
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    use super::*;

    #[test]
    fn test_basics() {
        let one_near = 10u128.pow(24);
        let mut context = VMContextBuilder::new();
        context.predecessor_account_id(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new();
    }
}
