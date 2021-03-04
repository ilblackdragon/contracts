use std::collections::HashMap;
use std::convert::TryInto;

use near_contract_standards::account_registration::AccountRegistrar;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, log, near_bindgen, serde_json, AccountId, Balance, PanicOnDefault, Promise};

use crate::pool::{ext_fungible_token, Pool, GAS_FOR_FT_TRANSFER, NO_DEPOSIT};
use crate::utils::FungibleTokenReceiver;

mod pool;
mod utils;

near_sdk::setup_alloc!();

const MAX_ACCOUNT_LENGTH: u128 = 64;
const MAX_NUMBER_OF_TOKENS: u128 = 10;
const BYTES_PER_DEPOSIT_RECORD: u128 =
    MAX_NUMBER_OF_TOKENS * (MAX_ACCOUNT_LENGTH + 16) + 4 + MAX_ACCOUNT_LENGTH;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
struct Contract {
    pools: Vector<Pool>,
    /// Balances of deposited tokens for each account.
    deposited_amounts: LookupMap<AccountId, HashMap<AccountId, Balance>>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_IS_INITIALIZED");
        Self {
            pools: Vector::new(b"p".to_vec()),
            deposited_amounts: LookupMap::new(b"d".to_vec()),
        }
    }

    /// Adds new pool with given tokens and give fee.
    /// Attached NEAR should be enough to cover the added storage.
    #[payable]
    pub fn add_pool(&mut self, tokens: Vec<ValidAccountId>, fee: u32) -> u32 {
        let prev_storage = env::storage_usage();
        let id = self.pools.len() as u32;
        self.pools.push(&Pool::new(id, tokens, fee));
        assert!(
            (env::storage_usage() - prev_storage) as u128 * env::storage_byte_cost()
                <= env::attached_deposit(),
            "ERR_STORAGE_DEPOSIT"
        );
        id
    }

    fn internal_register_account(&mut self, account_id: &AccountId) {
        self.deposited_amounts
            .insert(&account_id, &HashMap::default());
    }

    /// Record deposit of some number of tokens to this contract.
    fn internal_deposit(&mut self, sender_id: &AccountId, token_id: &AccountId, amount: Balance) {
        let mut amounts = self
            .deposited_amounts
            .get(sender_id)
            .expect("ERR_NOT_REGISTERED");
        assert!(amounts.len() <= 10, "ERR_TOO_MANY_TOKENS");
        amounts.insert(token_id.clone(), amount);
        self.deposited_amounts.insert(sender_id, &amounts);
    }

    fn internal_get_deposits(&self, sender_id: &AccountId) -> HashMap<AccountId, Balance> {
        self.deposited_amounts
            .get(sender_id)
            .expect("ERR_NO_DEPOSIT")
            .clone()
    }

    fn internal_get_deposit(&self, sender_id: &AccountId, token_id: &AccountId) -> Balance {
        self.internal_get_deposits(sender_id)
            .get(token_id)
            .expect("ERR_NO_DEPOSIT_TOKEN")
            .clone()
    }

    /// Add liquidity from already deposited amounts to given pool.
    pub fn add_liquidity(&mut self, pool_id: u64) {
        let sender_id = env::predecessor_account_id();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        let mut amounts = Vec::new();
        let mut deposits = self.internal_get_deposits(&sender_id);
        for token_id in pool.tokens() {
            amounts.push(
                deposits
                    .remove(token_id)
                    .expect(&format!("ERR_MISSING_TOKEN:{}", token_id)),
            );
        }
        pool.add_liquidity(&sender_id, amounts);
        self.deposited_amounts.insert(&sender_id, &deposits);
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
        let mut deposits = self.internal_get_deposits(&sender_id);
        for i in 0..tokens.len() {
            *deposits.entry(tokens[i].clone()).or_default() += amounts[i];
        }
        self.deposited_amounts.insert(&sender_id, &deposits);
    }

    pub fn get_pool_shares(&self, pool_id: u64, account_id: ValidAccountId) -> U128 {
        self.pools
            .get(pool_id)
            .expect("ERR_NO_POOL")
            .share_balances(account_id.as_ref())
            .into()
    }

    pub fn get_pool_total_shares(&self, pool_id: u64) -> U128 {
        self.pools
            .get(pool_id)
            .expect("ERR_NO_POOL")
            .share_total_balance()
            .into()
    }

    /// Returns balances of the deposits for given user.
    pub fn get_deposits(&self, account_id: &AccountId) -> HashMap<AccountId, U128> {
        self.internal_get_deposits(account_id)
            .into_iter()
            .map(|(acc, bal)| (acc, U128(bal)))
            .collect()
    }

    /// Returns balance of the deposit for given user.
    pub fn get_deposit(&self, account_id: &AccountId, token_id: &AccountId) -> U128 {
        self.internal_get_deposit(account_id, token_id).into()
    }

    /// Withdraws given token from the deposits of given user.
    pub fn withdraw(&mut self, token_id: ValidAccountId, amount: U128) {
        let amount: u128 = amount.into();
        let sender_id = env::predecessor_account_id();
        let mut deposits = self.deposited_amounts.get(&sender_id).unwrap();
        let available_amount = deposits
            .get(token_id.as_ref())
            .expect("ERR_NO_TOKEN")
            .clone();
        assert!(available_amount >= amount, "ERR_NOT_ENOUGH");
        if available_amount == amount {
            deposits.remove(token_id.as_ref());
        } else {
            deposits.insert(token_id.as_ref().clone(), available_amount - amount);
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

    /// Given specific pool, returns amount of token_out recevied swapping amount_in of token_in.
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

impl AccountRegistrar for Contract {
    fn ar_register(&mut self, account_id: Option<ValidAccountId>) -> bool {
        let amount = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());
        if self.deposited_amounts.contains_key(&account_id) {
            log!("The account is already registered, refunding the deposit");
            if amount > 0 {
                Promise::new(account_id).transfer(amount);
            }
            return false;
        }
        let ar_registration_fee = self.ar_registration_fee().0;
        if amount < ar_registration_fee {
            env::panic(b"The attached deposit is less than the account registration fee");
        }

        self.internal_register_account(&account_id);
        let refund = amount - ar_registration_fee;
        if refund > 0 {
            Promise::new(account_id).transfer(refund);
        }
        true
    }

    fn ar_is_registered(&self, account_id: ValidAccountId) -> bool {
        self.deposited_amounts.contains_key(account_id.as_ref())
    }

    fn ar_unregister(&mut self, _force: Option<bool>) -> bool {
        unimplemented!()
    }

    fn ar_registration_fee(&self) -> U128 {
        (BYTES_PER_DEPOSIT_RECORD * env::storage_byte_cost()).into()
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// Message structure:
    ///  - deposit
    ///  - swap:pool_id:token_out:min_amount_out
    fn ft_on_transfer(&mut self, sender_id: ValidAccountId, amount: U128, msg: String) -> U128 {
        let token_in = env::predecessor_account_id();
        if msg == "deposit" {
            self.internal_deposit(sender_id.as_ref(), &token_in, amount.into());
        } else {
            let pieces: Vec<&str> = msg.split(":").collect();
            assert_eq!(pieces.len(), 4);
            assert_eq!(pieces[0], "swap");
            let pool_id = serde_json::from_str::<u64>(pieces[1]).expect("ERR_MSG_POOL_ID");
            let token_out = pieces[2].to_string();
            let min_amount_out = serde_json::from_str::<u128>(pieces[3]).expect("ERR_MSG_POOL_ID");
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

        // create 1st pool (1, 2) with 0.3% fee.
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(env::storage_byte_cost() * 300)
            .build());
        contract.add_pool(vec![accounts(1), accounts(2)], 3);

        // add liquidity of (1,2) tokens and create 1st pool.
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(contract.ar_registration_fee().into())
            .build());
        contract.ar_register(None);
        testing_env!(context.predecessor_account_id(accounts(1)).build());
        contract.ft_on_transfer(accounts(3), (5 * one_near).into(), "deposit".to_string());
        testing_env!(context.predecessor_account_id(accounts(2)).build());
        contract.ft_on_transfer(accounts(3), (10 * one_near).into(), "deposit".to_string());
        testing_env!(context.predecessor_account_id(accounts(3)).build());
        assert_eq!(
            contract.get_deposit(accounts(3).as_ref(), accounts(1).as_ref()),
            (5 * one_near).into()
        );
        assert_eq!(
            contract.get_deposit(accounts(3).as_ref(), accounts(2).as_ref()),
            (10 * one_near).into()
        );
        contract.add_liquidity(0);
        assert_eq!(
            contract.get_pool_total_shares(0),
            U128(1000000000000000000000)
        );

        // Get price from pool #0 1 -> 2 tokens.
        let price = contract.get_return(0, accounts(1), one_near.into(), accounts(2));
        assert_eq!(price, 1662497915624478906119726.into());

        testing_env!(context.predecessor_account_id(accounts(1)).build());
        // swap:pool_id:token_out:min_amount_out
        contract.ft_on_transfer(
            accounts(3),
            one_near.into(),
            format!("swap:{}:{}:{}", 0, accounts(2).as_ref(), 1),
        );

        testing_env!(context.predecessor_account_id(accounts(3)).build());
        contract.remove_liquidity(
            0,
            contract.get_pool_shares(0, accounts(3)),
            vec![1.into(), 2.into()],
        );
        assert_eq!(contract.get_pool_total_shares(0), U128(0));

        contract.withdraw(
            accounts(1),
            contract.get_deposit(accounts(3).as_ref(), accounts(1).as_ref()),
        );
    }

    /// Should deny creating a pool with duplicate tokens.
    #[test]
    fn test_deny_duplicate_tokens_pool() {}
}
