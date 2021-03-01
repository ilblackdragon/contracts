mod math;

use near_contract_standards::fungible_token::{
    FungibleToken, FungibleTokenCore, FungibleTokenMetadata, FungibleTokenMetadataProvider,
};
use near_contract_standards::storage_manager::{AccountStorageBalance, StorageManager};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, near_bindgen, Balance, PanicOnDefault, Promise};

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
struct Contract {
    token: FungibleToken,
    reserve_balance: Balance,
    reserve_ratio: u32,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(initial_amount: U128, reserve_ratio: u32) -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_IS_INITIALIZED");
        // Attached deposit and account balance must be larger than storage usage, otherwise tx fails anyway.
        let reserve_balance =
            env::account_balance() - (env::storage_usage() as u128) * env::storage_byte_cost();
        let mut this = Self {
            token: FungibleToken::new(),
            reserve_balance,
            reserve_ratio,
        };
        this.token
            .internal_register_account(&env::predecessor_account_id());
        this.token
            .internal_deposit(&env::predecessor_account_id(), initial_amount.into());
        this
    }

    #[payable]
    pub fn mint(&mut self, account_id: ValidAccountId) -> U128 {
        let deposit = env::attached_deposit();
        let amount = math::calc_purchase_amount(
            self.ft_total_supply().0,
            self.reserve_balance,
            self.reserve_ratio,
            deposit,
        );
        self.reserve_balance += deposit;
        self.token.internal_deposit(account_id.as_ref(), amount);
        amount.into()
    }

    pub fn burn(&mut self, amount: U128) -> Promise {
        let return_amount = math::calc_sale_amount(
            self.ft_total_supply().0,
            self.reserve_balance,
            self.reserve_ratio,
            amount.into(),
        );
        self.reserve_balance -= return_amount;
        self.token
            .internal_withdraw(&env::predecessor_account_id(), amount.into());
        Promise::new(env::predecessor_account_id()).transfer(return_amount)
    }
}

#[near_bindgen]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: ValidAccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> Promise {
        self.token.ft_transfer_call(receiver_id, amount, msg, memo)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: ValidAccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near_bindgen]
impl StorageManager for Contract {
    #[payable]
    fn storage_deposit(&mut self, account_id: Option<ValidAccountId>) -> AccountStorageBalance {
        self.token.storage_deposit(account_id)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: U128) -> AccountStorageBalance {
        self.token.storage_withdraw(amount)
    }

    fn storage_minimum_balance(&self) -> U128 {
        self.token.storage_minimum_balance()
    }

    fn storage_balance_of(&self, account_id: ValidAccountId) -> AccountStorageBalance {
        self.token.storage_balance_of(account_id)
    }
}

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata() -> FungibleTokenMetadata {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{env, testing_env, MockedBlockchain};

    use super::*;

    const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;

    #[test]
    fn test_basics() {
        let mut context = VMContextBuilder::new();
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .account_balance(1000 * env::storage_byte_cost())
            .storage_usage(1000)
            .attached_deposit(ONE_NEAR)
            .build());
        // Reserve 1/2, initial amount = 1e24 with 1e24N in reserve.
        let mut contract = Contract::new(ONE_NEAR.into(), 500_000);
        testing_env!(context
            .attached_deposit(125 * env::storage_byte_cost())
            .build());
        contract.storage_deposit(Some(accounts(0)));
        testing_env!(context.attached_deposit(ONE_NEAR).build());
        let minted_amount = contract.mint(accounts(0));
        assert_eq!(
            contract.ft_balance_of(accounts(0)),
            414213562373095139835904.into()
        );
        let rb = contract.reserve_balance;
        contract.burn(minted_amount);
        // After burning, the balance subtracted is around what was deposited.
        assert!(rb - contract.reserve_balance < ONE_NEAR + 10u128.pow(10));
    }
}
