use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{
    env, ext_contract, near_bindgen, serde_json, AccountId, Balance, Gas, PanicOnDefault, Promise,
};
use std::convert::TryInto;

const FEE_DIVISOR: u32 = 1_000;
const NO_DEPOSIT: Balance = 0;
const GAS_FOR_SWAP: Gas = 10_000_000_000_000;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
struct Contract {
    /// Account for the token.
    token_account_id: AccountId,
    /// Fee.
    fee: u32,
    /// Balances of NEAR that were deposited but not consumed yet.
    near_balances: LookupMap<AccountId, Balance>,
    /// Shares of the pool by liquidity providers.
    shares: LookupMap<AccountId, Balance>,
    shares_total_supply: Balance,
    /// How much NEAR this contract has.
    near_amount: Balance,
    /// How much token this contract has.
    token_amount: Balance,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(token_account_id: ValidAccountId, fee: u32) -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_IS_INITIALIZED");
        Self {
            token_account_id: token_account_id.into(),
            fee,
            near_balances: LookupMap::new(b"t".to_vec()),
            shares: LookupMap::new(b"s".to_vec()),
            shares_total_supply: 0,
            near_amount: 0,
            token_amount: 0,
        }
    }

    /// Adds liquidity to this pool.
    #[payable]
    pub fn add_liquidity(&mut self) {
        let amount = env::attached_deposit();
        add_to_collection(
            &mut self.near_balances,
            &env::predecessor_account_id(),
            amount,
        );
    }

    pub fn remove_liquidity(&mut self, _shares: Balance) {
        // TODO
    }

    /// Pricing between two reserves given input amount.
    fn get_input_price(
        &self,
        input_amount: Balance,
        input_reserve: Balance,
        output_reserve: Balance,
    ) -> Balance {
        assert!(input_reserve > 0 && output_reserve > 0, "ERR_NO_LIQUIDITY");
        let input_amount_with_fee = input_amount as f64 * (FEE_DIVISOR - self.fee) as f64;
        ((input_amount_with_fee * output_reserve as f64)
            / (input_reserve as f64 * FEE_DIVISOR as f64 + input_amount_with_fee))
            .floor() as u128
    }

    /// Pricing between two reserves to return given output amount.
    fn get_output_price(
        &self,
        output_amount: Balance,
        input_reserve: Balance,
        output_reserve: Balance,
    ) -> Balance {
        assert!(
            input_reserve > 0 && output_reserve > output_amount,
            "ERR_NO_LIQUIDITY"
        );
        ((input_reserve as f64 * output_amount as f64 * FEE_DIVISOR as f64)
            / ((output_reserve - output_amount) as f64 * (FEE_DIVISOR - self.fee) as f64))
            .ceil() as u128
        // (input_reserve * output_amount * self.fee as u128)
        //     / ((output_reserve - output_amount) * FEE_DIVISOR as u128)
    }

    /// Returns price of given amount of NEAR in token.
    pub fn get_near_to_token_price(&self, amount: Balance) -> Balance {
        self.get_output_price(amount, self.near_amount, self.token_amount)
    }

    /// Returns price of given amount of token in NEAR.
    pub fn get_token_to_near_price(&self, amount: Balance) -> Balance {
        self.get_output_price(amount, self.token_amount, self.near_amount)
    }

    #[payable]
    pub fn swap_near_to_token(&mut self, min_amount: Balance) -> Balance {
        let payed_amount = env::attached_deposit();
        let tokens_bought = self.get_input_price(payed_amount, self.near_amount, self.token_amount);
        assert!(tokens_bought >= min_amount, "ERR_MIN_AMOUNT");
        self.near_amount += payed_amount;
        self.token_amount -= tokens_bought;
        ext_fungible_token::ft_transfer(
            env::predecessor_account_id().try_into().unwrap(),
            U128(tokens_bought),
            None,
            &self.token_account_id,
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_SWAP,
        );
        // TODO: handle failure to transfer (e.g. no storage).
        tokens_bought
    }

    fn swap_token_to_near(
        &mut self,
        sender_id: &AccountId,
        token_amount: Balance,
        min_near_amount: Balance,
    ) -> Promise {
        let near_bought = self.get_input_price(token_amount, self.token_amount, self.near_amount);
        assert!(near_bought >= min_near_amount, "ERR_MIN_AMOUNT");
        self.near_amount -= near_bought;
        self.token_amount -= token_amount;
        Promise::new(sender_id.clone()).transfer(near_bought)
    }

    fn finish_add_liquidity(&mut self, sender_id: &AccountId, amount: U128) -> U128 {
        let near_amount = self
            .near_balances
            .remove(&sender_id)
            .expect("ERR_NOT_ADD_LIQUIDITY");
        if self.shares_total_supply > 0 {
            let expected_token_amount = near_amount * self.token_amount / self.near_amount;
            assert!(
                expected_token_amount <= amount.into(),
                "ERR_NOT_ENOUGH_TOKEN"
            );
            let liquidity_minted = near_amount * self.shares_total_supply / self.near_amount;
            add_to_collection(&mut self.near_balances, sender_id, liquidity_minted);
            self.shares_total_supply += liquidity_minted;
            self.near_amount += near_amount;
            self.token_amount += expected_token_amount;
            expected_token_amount.into()
        } else {
            self.shares_total_supply = near_amount;
            self.near_amount = near_amount;
            self.token_amount = amount.into();
            add_to_collection(&mut self.near_balances, sender_id, near_amount);
            amount
        }
    }
}

#[ext_contract(ext_fungible_token)]
trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: ValidAccountId, amount: U128, memo: Option<String>);
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
        assert_eq!(
            env::predecessor_account_id(),
            self.token_account_id,
            "ERR_WRONG_TOKEN"
        );
        if msg == "liquidity" {
            self.finish_add_liquidity(sender_id.as_ref(), amount)
        } else {
            self.swap_token_to_near(
                sender_id.as_ref(),
                amount.into(),
                serde_json::from_str::<U128>(&msg).expect("ERR_MSG").into(),
            );
            amount
        }
    }
}

pub fn add_to_collection(
    c: &mut LookupMap<AccountId, Balance>,
    account_id: &AccountId,
    amount: Balance,
) {
    let prev_amount = c.get(account_id).unwrap_or(0);
    c.insert(account_id, &(prev_amount + amount));
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
        let mut contract = Contract::new(accounts(1), 3);
        testing_env!(context.attached_deposit(5 * one_near).build());
        contract.add_liquidity();
        testing_env!(context.predecessor_account_id(accounts(1)).build());
        contract.ft_on_transfer(
            accounts(0).into(),
            (10 * one_near).into(),
            "liquidity".to_string(),
        );

        let price = contract.get_near_to_token_price(one_near);
        assert_eq!(price, 557227237267357694951424);
        let price = contract.get_token_to_near_price(one_near);
        assert_eq!(price, 2507522567703109526618112);

        testing_env!(context.attached_deposit(one_near).build());
        let result = contract.swap_near_to_token(1);

        assert_eq!(contract.near_amount, 6 * one_near);
        assert_eq!(contract.token_amount, 10 * one_near - result);
    }
}
