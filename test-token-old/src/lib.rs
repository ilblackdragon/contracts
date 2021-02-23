use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::{env, near_bindgen, AccountId};

use near_lib::token::{FungibleToken, Token};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct TToken {
    token: Token,
}

impl Default for TToken {
    fn default() -> Self {
        panic!("Test token should be initialized before usage")
    }
}

#[near_bindgen]
impl TToken {
    /// Initializes the contract with the given total supply owned by the given `owner_id`.
    #[init]
    pub fn new(owner_id: AccountId, total_supply: U128) -> Self {
        let total_supply = total_supply.into();
        assert!(!env::state_exists(), "Already initialized");
        Self {
            token: Token::new(owner_id, total_supply),
        }
    }

    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        self.token.mint(account_id, amount.into());
    }
}

#[near_bindgen]
impl FungibleToken for TToken {
    #[payable]
    fn inc_allowance(&mut self, escrow_account_id: String, amount: U128) {
        self.token.inc_allowance(escrow_account_id, amount.into());
    }

    #[payable]
    fn dec_allowance(&mut self, escrow_account_id: String, amount: U128) {
        self.token.dec_allowance(escrow_account_id, amount.into());
    }

    #[payable]
    fn transfer_from(&mut self, owner_id: String, new_owner_id: String, amount: U128) {
        self.token
            .transfer_from(owner_id, new_owner_id, amount.into());
    }

    #[payable]
    fn transfer(&mut self, new_owner_id: String, amount: U128) {
        self.token.transfer(new_owner_id, amount.into());
    }

    fn get_total_supply(&self) -> U128 {
        self.token.get_total_supply().into()
    }

    fn get_balance(&self, owner_id: String) -> U128 {
        self.token.get_balance(owner_id).into()
    }

    fn get_allowance(&self, owner_id: String, escrow_account_id: String) -> U128 {
        self.token.get_allowance(owner_id, escrow_account_id).into()
    }
}
