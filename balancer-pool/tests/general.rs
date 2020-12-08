use near_sdk::AccountId;
use near_sdk::json_types::U128;
use near_test::test_user::{init_test_runtime, TestRuntime, to_yocto};
use near_test::token::TokenContract;
use serde_json::json;

const WETH: &str = "weth";
const MKR: &str = "mkr";
const DAI: &str = "dai";
const XXX: &str = "xxx";
const POOL: &str = "pool";

lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../../test-token/res/test_token.wasm").as_ref();
    static ref POOL_WASM_BYTES: &'static [u8] = include_bytes!("../res/balancer_pool.wasm").as_ref();
}

pub struct BPool {
    contract_id: AccountId,
}

impl BPool {
    pub fn new(runtime: &mut TestRuntime, signer_id: &AccountId, contract_id: AccountId) -> Self {
        let _ = runtime
            .deploy(signer_id.clone(), contract_id.clone(), &POOL_WASM_BYTES, json!({}))
            .unwrap();
        Self { contract_id }
    }

    pub fn getController(&self, runtime: &mut TestRuntime) -> AccountId {
        runtime.view(self.contract_id.clone(), "getController", json!({}))
            .as_str()
            .unwrap()
            .to_string()
    }

    pub fn getNumTokens(&self, runtime: &mut TestRuntime) -> u64 {
        runtime.view(self.contract_id.clone(), "getNumTokens", json!({}))
            .as_u64()
            .unwrap()
    }

    pub fn bind(&self, runtime: &mut TestRuntime, signer_id: &AccountId, token: AccountId, balance: &str, denorm: &str) {
        let _ = runtime.call(signer_id.clone(), self.contract_id.clone(), "bind", json!({"token": token, "balance": U128::from(to_yocto(balance)), "denorm": U128::from(to_yocto(denorm))}), 0).unwrap();
    }
}

fn setup_multi_token_pool() -> (
    TestRuntime,
    BPool,
    TokenContract,
    TokenContract,
    TokenContract,
    TokenContract,
) {
    let mut runtime = init_test_runtime();
    let root = "root".to_string();
    let user1 = "user1".to_string();
    let user2 = "user2".to_string();

    let pool = BPool::new(&mut runtime, &root, POOL.to_string());

    let weth = TokenContract::new(&mut runtime, &root, &TOKEN_WASM_BYTES, WETH.to_string(), &root, "50");
    let mkr = TokenContract::new(&mut runtime, &root, &TOKEN_WASM_BYTES, MKR.to_string(), &root, "20");
    let dai = TokenContract::new(
        &mut runtime,
        &root,
        &TOKEN_WASM_BYTES,
        DAI.to_string(),
        &root,
        "10000",
    );
    let xxx = TokenContract::new(&mut runtime, &root, &TOKEN_WASM_BYTES, XXX.to_string(), &root, "10");

    // User1 balances.
    weth.mint(&mut runtime, &root, &user1, "25");
    mkr.mint(&mut runtime, &root, &user1, "4");
    dai.mint(&mut runtime, &root, &user1, "40000");
    xxx.mint(&mut runtime, &root, &user1, "10");

    // User2 balances.
    weth.mint(&mut runtime, &root, &user2, "12.2222");
    mkr.mint(&mut runtime, &root, &user2, "1.015333");
    dai.mint(&mut runtime, &root, &user2, "0");
    xxx.mint(&mut runtime, &root, &user2, "51");

    (runtime, pool, weth, mkr, dai, xxx)
}

#[test]
fn multi_token_pool() {
    let (mut user, pool, weth, mkr, dai, xxx) = setup_multi_token_pool();
    let root = "root".to_string();
    assert_eq!(pool.getController(&mut user), root);
    assert_eq!(pool.getNumTokens(&mut user), 0);
}

#[test]
fn deposit_failure() {
    let (mut user, pool, weth, mkr, dai, xxx) = setup_multi_token_pool();
    let root = "root".to_string();
    pool.bind(&mut user, &root, weth.contract_id, "100", "1");
}
