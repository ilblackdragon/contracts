use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::{env, near_bindgen, AccountId, Promise};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

const CODE: &[u8] = include_bytes!("../../sputnikdao/res/sputnikdao.wasm");

/// This gas spent on the call & account creation, the rest goes to the `new` call.
const CREATE_CALL_GAS: u64 = 30_000_000_000_000;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, Default)]
pub struct SputnikDAOFactory {}

#[near_bindgen]
impl SputnikDAOFactory {
    pub fn create(&self, name: AccountId, args: Base64VecU8) -> Promise {
        Promise::new(format!("{}.{}", name, env::current_account_id()))
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
