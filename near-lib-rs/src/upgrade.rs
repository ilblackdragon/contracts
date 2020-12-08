use near_sdk::{AccountId, env, Promise};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8};

use crate::types::{Duration, Timestamp, WrappedDuration};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Upgrade {
    pub owner: AccountId,
    pub staging_duration: Duration,
    pub staging_timestamp: Timestamp,
}

impl Upgrade {
    pub fn new(owner: AccountId, staging_duration: Duration) -> Self {
        Self {
            owner,
            staging_duration,
            staging_timestamp: 0,
        }
    }

    pub fn assert_owner(&self) {
        assert_eq!(env::predecessor_account_id(), self.owner);
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    pub fn set_owner(&mut self, owner: AccountId) {
        self.owner = owner;
    }

    pub fn get_staging_duration(&self) -> WrappedDuration {
        self.staging_duration.into()
    }

    pub fn stage_code(&mut self, code: Base64VecU8, timestamp: Timestamp) {
        self.assert_owner();
        assert!(env::block_timestamp() + self.staging_duration < timestamp, "Timestamp must be later than staging duration");
        // Writes directly into storage to avoid serialization penalty by using default struct.
        env::storage_write(b"upgrade", &code.0);
        self.staging_timestamp = timestamp;
    }

    pub fn deploy_code(&mut self) -> Promise {
        if self.staging_timestamp < env::block_timestamp() {
            env::panic(&format!("Deploy code too early: staging ends on {}", self.staging_timestamp + self.staging_duration).into_bytes());
        }
        let code = env::storage_read(b"upgrade").expect("No upgrade code available");
        env::storage_remove(b"upgrade");
        Promise::new(env::current_account_id()).deploy_contract(code)
    }
}

pub trait Upgradable {
    fn get_owner(&self) -> AccountId;
    fn set_owner(&mut self, owner: AccountId);
    fn get_staging_duration(&self) -> WrappedDuration;
    fn stage_code(&mut self, code: Base64VecU8, timestamp: Timestamp);
    fn deploy_code(&mut self) -> Promise;

    /// Implement migration for the next version.
    /// Should be empty for the new contract.
    /// TODO: consider adding version of the contract stored in the storage?
    fn migrate(&mut self);
}
