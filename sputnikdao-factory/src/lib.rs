use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::{env, near_bindgen, AccountId, Promise};
use near_sdk::collections::UnorderedSet;

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

const CODE: &[u8] = include_bytes!("../../sputnikdao/res/sputnikdao.wasm");

/// This gas spent on the call & account creation, the rest goes to the `new` call.
const CREATE_CALL_GAS: u64 = 30_000_000_000_000;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct SputnikDAOFactory {
    daos: UnorderedSet<AccountId>,
}

impl Default for SputnikDAOFactory {
    fn default() -> Self {
        env::panic(b"SputnikDAOFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl SputnikDAOFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            daos: UnorderedSet::new(b"d".to_vec()),
        }
    }

    pub fn get_dao_list(&self) -> Vec<AccountId> {
        self.daos.to_vec()
    }

    pub fn create(&mut self, name: AccountId, args: Base64VecU8) -> Promise {
        let account_id= format!("{}.{}", name, env::current_account_id());
        self.daos.insert(&account_id);
        Promise::new(account_id)
            .create_account()
            .deploy_contract(CODE.to_vec())
            .transfer(env::attached_deposit())
            .function_call(
                b"new".to_vec(),
                args.into(),
                0,
                env::prepaid_gas() - CREATE_CALL_GAS,
            )
    }
}
