use std::collections::HashMap;

use near_lib::token::{FungibleToken, Token};
use near_lib::types::{Duration, Timestamp, WrappedDuration};
use near_lib::upgrade::{Upgradable, Upgrade};
use near_sdk::{AccountId, Balance, env, Promise, near_bindgen, init};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::{Base64VecU8, U128};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

/// Upgrade duration is 1 day.
const UPGRADE_STAGING_DURATION: Duration = 24 * 60 * 60 * 1_000_000_000;

/// Challenge duration.
const CHALLENGE_DURATION: Duration = 5 * 24 * 60 * 60 * 1_000_000_000;

/// Initial $TCR supply.
const INITIAL_SUPPLY: Balance = 1_000_000_000_000_000_000_000_000;

/// Keeps track how much NEAR this contract has received.
/// Accounts for storage usage and contract rewards.
#[derive(BorshSerialize, BorshDeserialize)]
struct Bank {
    balance: Balance,
    storage_usage: u64,
}

impl Bank {
    pub fn new() -> Self {
        Bank {
            balance: env::account_balance(),
            storage_usage: env::storage_usage(),
        }
    }

    /// Called at the start of the function at the state changing function.
    pub fn start_record(&mut self) {
        assert_eq!(self.storage_usage, env::storage_usage(), "Incorrect usage of Bank: was not called when storaged changed");
    }

    /// Called at the end of the function at the state changing function.
    pub fn end_record(&mut self) {
        self.storage_usage = env::storage_usage();
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
struct Row {
    owner: AccountId,
    fields: HashMap<String, String>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct Table {
    last_id: u64,
    rows: UnorderedMap<u64, Row>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            last_id: 0,
            rows: UnorderedMap::new(b"t".to_vec()),
        }
    }

    pub fn insert(&mut self, row: Row) -> u64 {
        self.rows.insert(&self.last_id, &row);
        self.last_id += 1;
        self.last_id - 1
    }

    pub fn delete(&mut self, id: u64) {
        self.rows.remove(&id);
    }

    pub fn update(&mut self, id: u64, new_row: Row) {
        self.rows.insert(&id, &new_row);
    }

    pub fn get(&self, id: u64) -> Option<Row> {
        self.rows.get(&id)
    }

    pub fn list(&self) -> Vec<(u64, Row)> {
        self.rows.to_vec()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
enum Vote {
    Null,
    Delete,
    Keep
}

#[derive(BorshSerialize, BorshDeserialize)]
struct Challenge {
    /// Initiator challenge.
    challenger: AccountId,
    /// Attached description: either link or short content.
    description: String,
    /// All the votes for given challenge.
    votes: HashMap<AccountId, (Vote, u128)>,
    /// When challenge concludes.
    end_time: Timestamp,
    /// Total votes for deleting.
    vote_delete: u128,
    /// Total votes for keeping.
    vote_keep: u128,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
struct TokenCuratedRegistry {
    upgrade: Upgrade,
    bank: Bank,
    token: Token,
    table: Table,
    challenges: UnorderedMap<u64, Challenge>,
}

impl TokenCuratedRegistry {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            upgrade: Upgrade::new(owner.clone(), UPGRADE_STAGING_DURATION),
            bank: Bank::new(),
            token: Token::new(owner, INITIAL_SUPPLY),
            table: Table::new(),
            challenges: UnorderedMap::new(b"c".to_vec()),
        }
    }

    pub fn get(&mut self, id: u64) -> Row {
        self.table.get(id).expect("Row is missing")
    }

    pub fn list(&mut self) -> Vec<(u64, Row)> {
        self.table.list()
    }

    // #[payable]
    pub fn insert(&mut self, fields: HashMap<String, String>) -> u64 {
        self.bank.start_record();
        let result = self.table.insert(Row { owner: env::predecessor_account_id(), fields });
        self.bank.end_record();
        result
    }

    // #[payable]
    pub fn update(&mut self, id: u64, fields: HashMap<String, String>) {
        self.bank.start_record();
        let mut row = self.get(id);
        assert_eq!(row.owner, env::predecessor_account_id());
        row.fields = fields;
        self.table.update(id, row);
        self.bank.end_record();
    }

    pub fn set_row_owner(&mut self, id: u64, new_owner: AccountId) {
        self.bank.start_record();
        let mut row = self.get(id);
        assert_eq!(row.owner, env::predecessor_account_id());
        row.owner = new_owner;
        self.table.update(id, row);
        self.bank.end_record();
    }

    pub fn get_challenge(&self, id: u64) -> Challenge {
        self.challenges.get(&id).expect("No challenge for given id")
    }

    pub fn get_challenge_list(&self) -> Vec<(u64, Challenge)> {
        self.challenges.to_vec()
    }

    /// Create new challenge. Must have CHALLENGE_DEPOSIT amount of $TCR to proceed.
    /// If the challenge is successful - $TCR is returned,
    /// if the challenge is unsuccessful - $TCR is burned.
//    #[payable]
    pub fn challenge(&mut self, id: u64, description: String) {
        assert!(self.challenges.get(&id).is_none(), "Given id already challenged");
        self.bank.start_record();
        let challenge = Challenge {
            challenger: env::predecessor_account_id(),
            description,
            votes: HashMap::default(),
            end_time: env::block_timestamp() + CHALLENGE_DURATION,
            vote_delete: 0,
            vote_keep: 0,
        };
        self.challenges.insert(&id, &challenge);
        self.bank.end_record();
    }

//    #[payable]
    pub fn challenge_vote(&mut self, id: u64, vote: Vote) {
        self.bank.start_record();
        let mut challenge = self.challenges.get(&id).expect("No challenge for given id");
        if challenge.votes.contains_key(&env::predecessor_account_id()) {
            env::panic(b"Already voted");
        }
        challenge.votes.insert(env::predecessor_account_id(), (vote.clone(), 1));
        match vote {
            Vote::Null => {},
            Vote::Delete => challenge.vote_delete += 1,
            Vote::Keep => challenge.vote_keep += 1,
        }
        self.challenges.insert(&id, &challenge);
        self.bank.end_record();
    }

    /// Anyone can call to finalize open challenge.
    pub fn finalize_challenge(&mut self, id: u64) {
        self.bank.start_record();
        let challenge = self.challenges.get(&id).expect("No challenge for given id");
        if challenge.end_time > env::block_timestamp() {
            env::panic(b"Challenge period didn't pass yet");
        }
        self.challenges.remove(&id);
        if challenge.vote_delete > challenge.vote_keep {
            self.table.delete(id);
            env::log(b"Challenge successful");
        } else {
            env::log(b"Challenge unsuccessful");
        }
        self.challenges.remove(&id);
        self.bank.end_record();
    }
}

#[near_bindgen]
impl Upgradable for TokenCuratedRegistry {
    fn get_owner(&self) -> AccountId {
        self.upgrade.get_owner()
    }

    fn set_owner(&mut self, owner: AccountId) {
        self.upgrade.set_owner(owner);
    }

    fn get_staging_duration(&self) -> WrappedDuration {
        self.upgrade.get_staging_duration()
    }

    fn stage_code(&mut self, code: Base64VecU8, timestamp: Timestamp) {
        self.upgrade.stage_code(code, timestamp);
    }

    fn deploy_code(&mut self) -> Promise {
        self.upgrade.deploy_code()
    }

    fn migrate(&mut self) {
    }
}

#[near_bindgen]
impl FungibleToken for TokenCuratedRegistry {
    fn inc_allowance(&mut self, escrow_account_id: String, amount: U128) {
        unimplemented!()
    }

    fn dec_allowance(&mut self, escrow_account_id: String, amount: U128) {
        unimplemented!()
    }

    fn transfer_from(&mut self, owner_id: String, new_owner_id: String, amount: U128) {
        unimplemented!()
    }

    fn transfer(&mut self, new_owner_id: String, amount: U128) {
        unimplemented!()
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

#[cfg(test)]
mod test {
    use near_lib::context::{accounts, VMContextBuilder};
    use near_sdk::{MockedBlockchain, testing_env};

    use super::*;

    #[test]
    fn test_edit_registry() {
        testing_env!(VMContextBuilder::new().finish());
        let mut registry = TokenCuratedRegistry::new(accounts(0));
        let id1 = registry.insert(vec![("name".to_string(), "123".to_string())].into_iter().collect());
        assert_eq!(registry.list().len(), 1);
        registry.challenge(id1, "test".to_string());
        assert_eq!(registry.get_challenge_list().len(), 1);
        assert_eq!(registry.get_challenge(id1).votes.len(), 0);
        registry.challenge_vote(id1, Vote::Delete);
        assert_eq!(registry.get_challenge(id1).votes.len(), 1);
        testing_env!(VMContextBuilder::new().block_timestamp(CHALLENGE_DURATION + 1).finish());
        registry.finalize_challenge(id1);
        assert_eq!(registry.get_challenge_list().len(), 0);
        assert_eq!(registry.list().len(), 0);
    }
}
