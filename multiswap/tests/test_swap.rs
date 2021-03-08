use std::convert::TryFrom;

use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::AccountId;
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount};

use multiswap::{ContractContract as Multiswap, PoolInfo, SwapAction};
use std::collections::HashMap;
use test_token::ContractContract as TestToken;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../test-token/res/test_token.wasm",
    MUTLISWAP_WASM_BYTES => "res/multiswap.wasm",
}

fn test_token(root: &UserAccount, token_id: AccountId) -> ContractAccount<TestToken> {
    let t = deploy!(
        contract: TestToken,
        contract_id: token_id,
        bytes: &TEST_TOKEN_WASM_BYTES,
        signer_account: root
    );
    call!(root, t.new()).assert_success();
    call!(
        root,
        t.mint(to_va(root.account_id.clone()), to_yocto("1000").into())
    )
    .assert_success();
    t
}

fn dai() -> AccountId {
    "dai".to_string()
}

fn eth() -> AccountId {
    "eth".to_string()
}

fn swap() -> AccountId {
    "swap".to_string()
}

fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

#[test]
fn test_swap() {
    let root = init_simulator(None);
    let token1 = test_token(&root, dai());
    let token2 = test_token(&root, eth());
    let pool = deploy!(
        contract: Multiswap,
        contract_id: swap(),
        bytes: &MUTLISWAP_WASM_BYTES,
        signer_account: root
    );
    call!(root, pool.new());
    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(eth())], 30),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        root,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        token1.storage_deposit(Some(to_va(swap())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        token2.storage_deposit(Some(to_va(swap())), None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        root,
        token1.ft_transfer_call(to_va(swap()), to_yocto("105").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token2.ft_transfer_call(to_va(swap()), to_yocto("110").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        pool.add_liquidity(0, vec![U128(to_yocto("5")), U128(to_yocto("10"))])
    )
    .assert_success();
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            token_account_ids: vec![dai(), eth()],
            amounts: vec![to_yocto("5").into(), to_yocto("10").into()],
            fee: 30,
            shares_total_supply: to_yocto("1").into(),
        }
    );
    let balances =
        view!(pool.get_deposits(&root.account_id)).unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(to_yocto("100")), U128(to_yocto("100"))]);

    call!(
        root,
        pool.swap(vec![SwapAction {
            pool_id: 0,
            token_in: to_va(dai()),
            amount_in: Some(U128(to_yocto("1"))),
            token_out: to_va(eth()),
            min_amount_out: U128(1)
        }])
    )
    .assert_success();

    let balances =
        view!(pool.get_deposits(&root.account_id)).unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(
        balances.get(&eth()).unwrap(),
        &U128(to_yocto("100") + 1662497915624478906119726)
    );
    assert_eq!(balances.get(&dai()).unwrap(), &U128(to_yocto("99")));

    call!(
        root,
        pool.withdraw(to_va(eth()), U128(to_yocto("101"))),
        deposit = 1
    );
    call!(
        root,
        pool.withdraw(to_va(dai()), U128(to_yocto("99"))),
        deposit = 1
    );

    let balance1 = view!(token1.ft_balance_of(to_va(root.account_id.clone())))
        .unwrap_json::<U128>()
        .0;
    assert_eq!(balance1, to_yocto("994"));
    let balance2 = view!(token2.ft_balance_of(to_va(root.account_id.clone())))
        .unwrap_json::<U128>()
        .0;
    assert_eq!(balance2, to_yocto("991"));
}
