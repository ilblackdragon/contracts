use near_lib::upgrade::Ownable;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::{env, near_bindgen, AccountId, Promise};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

const CODE_KEY: &[u8; 4] = b"code";

/// This gas spent on the call & account creation, the rest goes to the `new` call.
const CREATE_CALL_GAS: u64 = 5_000_000_000_000;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct GenericFactory {
    owner: AccountId,
}

impl Default for GenericFactory {
    fn default() -> Self {
        env::panic(b"GenericFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl GenericFactory {
    #[init]
    pub fn new(#[serializer(borsh)] owner: AccountId, #[serializer(borsh)] code: Vec<u8>) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        env::storage_write(CODE_KEY, &code);
        Self { owner }
    }

    pub fn create(&self, name: AccountId, args: Base64VecU8) -> Promise {
        let code = env::storage_read(CODE_KEY).expect("Code must be present");
        Promise::new(format!("{}.{}", name, env::current_account_id()))
            .create_account()
            .deploy_contract(code)
            .function_call(
                b"new".to_vec(),
                args.into(),
                env::attached_deposit(),
                env::prepaid_gas() - CREATE_CALL_GAS,
            )
    }

    pub fn upgrade(&self, #[serializer(borsh)] code: Vec<u8>) {
        self.assert_owner();
        env::storage_write(CODE_KEY, &code);
    }
}

impl Ownable for GenericFactory {
    fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }
    fn set_owner(&mut self, owner: AccountId) {
        self.assert_owner();
        self.owner = owner;
    }
}

#[cfg(test)]
mod tests {
    use near_lib::context::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    use super::*;

    #[test]
    fn test_basics() {
        testing_env!(VMContextBuilder::new().finish());
        let factory = GenericFactory::new(accounts(0), vec![].into());
        assert_eq!(factory.get_owner(), accounts(0));
        factory.create("test".to_string(), vec![].into());
    }
}
