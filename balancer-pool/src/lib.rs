use borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::{env, ext_contract, near_bindgen, AccountId, Balance, Promise};

mod bconst;
mod bmath;

use bconst::*;
use bmath::calc_spot_price;
use near_lib::token::{ext_nep21, FungibleToken, Token};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Record {
    bound: bool,
    index: u64,
    denorm: Weight,
    balance: Balance,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct BPool {
    controller: AccountId,
    factory: AccountId,
    swap_fee: Balance,
    finalized: bool,
    public_swap: bool,
    records: UnorderedMap<AccountId, Record>,
    tokens: Vec<AccountId>,
    total_weight: Weight,
    token: Token,
}

impl Default for BPool {
    fn default() -> Self {
        panic!("BPool should be initialized before usage")
    }
}

#[near_bindgen]
impl BPool {
    #[init]
    pub fn new() -> Self {
        Self {
            controller: env::predecessor_account_id(),
            factory: env::predecessor_account_id(),
            swap_fee: MIN_FEE,
            public_swap: false,
            finalized: false,
            records: UnorderedMap::new(b"r".to_vec()),
            tokens: Vec::new(),
            total_weight: 0,
            token: Token::new(env::signer_account_id(), 0u128),
        }
    }

    // Getters

    pub fn isPublicSwap(&self) -> bool {
        self.public_swap
    }

    pub fn isFinalized(&self) -> bool {
        self.finalized
    }

    pub fn isBound(&self, token: AccountId) -> bool {
        self.records
            .get(&token)
            .map(|record| record.bound)
            .unwrap_or(false)
    }

    pub fn getNumTokens(&self) -> u64 {
        self.tokens.len() as u64
    }

    pub fn getCurrentTokens(&self) -> Vec<AccountId> {
        self.tokens.clone()
    }

    pub fn getFinalTokens(&self) -> Vec<AccountId> {
        assert!(self.finalized, "ERR_NOT_FINALIZED");
        self.tokens.clone()
    }

    pub fn getDenormalizedWeight(&self, token: AccountId) -> U128 {
        assert!(self.isBound(token.clone()), "ERR_NOT_BOUND");
        self.records.get(&token).unwrap().denorm.into()
    }

    pub fn getTotalDenormalizedWeight(&self) -> U128 {
        self.total_weight.into()
    }

    pub fn getNormalizedWeight(&self, token: AccountId) -> U128 {
        assert!(self.isBound(token.clone()), "ERR_NOT_BOUND");
        let denorm = self.records.get(&token).unwrap().denorm;
        // TODO: this division is special?
        (denorm / self.total_weight).into()
    }

    pub fn getBalance(&self, token: AccountId) -> U128 {
        assert!(self.isBound(token.clone()), "ERR_NOT_BOUND");
        self.records.get(&token).unwrap().balance.into()
    }

    pub fn getSwapFee(&self) -> U128 {
        self.swap_fee.into()
    }

    pub fn getController(&self) -> AccountId {
        self.controller.clone()
    }

    // Setters.

    pub fn setSwapFee(&mut self, swapFee: U128) {
        let swap_fee = swapFee.into();
        assert!(!self.finalized, "ERR_IS_FINALIZED");
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "ERR_NOT_CONTROLLER"
        );
        assert!(swap_fee >= MIN_FEE, "ERR_MIN_FEE");
        assert!(swap_fee <= MAX_FEE, "ERR_MIN_FEE");
        self.swap_fee = swap_fee;
    }

    pub fn setController(&mut self, controller: AccountId) {
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "ERR_NOT_CONTROLLER"
        );
        self.controller = controller;
    }

    pub fn setPublicSwap(&mut self, public: bool) {
        assert!(!self.finalized, "ERR_IS_FINALIZED");
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "ERR_NOT_CONTROLLER"
        );
        self.public_swap = public;
    }

    pub fn finalize(&mut self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "ERR_NOT_CONTROLLER"
        );
        assert!(!self.finalized, "ERR_IS_FINALIZED");
        assert!(self.tokens.len() >= MIN_BOUND_TOKENS, "ERR_MIN_TOKENS");

        self.finalized = true;
        self.public_swap = true;

        self.mint_pool_share(INIT_POOL_SUPPLY);
        self.push_pool_share(env::predecessor_account_id(), INIT_POOL_SUPPLY);
    }

    pub fn bind(&mut self, token: AccountId, balance: U128, denorm: U128) {
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "ERR_NOT_CONTROLLER"
        );
        assert!(!self.isBound(token.clone()), "ERR_IS_BOUND");
        assert!(!self.finalized, "ERR_IS_FINALIZED");
        assert!(self.tokens.len() < MAX_BOUND_TOKENS, "ERR_MAX_TOKENS");

        self.records.insert(
            &token,
            &Record {
                bound: true,
                index: self.tokens.len() as u64,
                denorm: 0,
                balance: 0,
            },
        );
        self.tokens.push(token.clone());
        self.rebind(token, balance.into(), denorm.into());
    }

    pub fn rebind(&mut self, token: AccountId, balance: Balance, denorm: Weight) {
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "ERR_NOT_CONTROLLER"
        );
        assert!(self.isBound(token.clone()), "ERR_NOT_BOUND");
        assert!(!self.finalized, "ERR_IS_FINALIZED");

        assert!(denorm >= MIN_WEIGHT, "ERR_MIN_WEIGHT");
        assert!(denorm <= MAX_WEIGHT, "ERR_MAX_WEIGHT");
        assert!(balance >= MIN_BALANCE, "ERR_MIN_BALANCE");

        let mut record = self.records.get(&token).unwrap();
        let old_weight = record.denorm;
        record.denorm = denorm;
        if denorm > old_weight {
            self.total_weight = self.total_weight + (denorm - old_weight);
        } else {
            self.total_weight = self.total_weight - (old_weight - denorm);
        }

        let old_balance = record.balance;
        record.balance = balance;
        if balance > old_balance {
            self.pull_underlying(
                &token,
                &env::predecessor_account_id(),
                balance - old_balance,
            );
        } else {
            let token_balance_withdrawn = old_balance - balance;
            let token_exit_fee = token_balance_withdrawn * EXIT_FEE;
            self.push_underlying(
                token.clone(),
                env::predecessor_account_id(),
                token_balance_withdrawn - token_exit_fee,
            );
            self.push_underlying(token.clone(), self.factory.clone(), token_exit_fee);
        }
        // TODO: move this into the on_pull or else clause.
        self.records.insert(&token, &record);
    }

    pub fn unbind(&mut self, token: AccountId) {}

    /// Absorb any tokens that have been sent to this contract into the pool.
    pub fn gulp(&mut self, token: AccountId) {
        // TODO: call ext_nep21.balance(env::current_account_id(), token.clone()).then()
    }

    pub fn getSpotPrice(&self, tokenIn: AccountId, tokenOut: AccountId) -> Balance {
        assert!(self.isBound(tokenIn.clone()), "ERR_NOT_BOUND");
        assert!(self.isBound(tokenOut.clone()), "ERR_NOT_BOUND");
        let in_record = self.records.get(&tokenIn).unwrap();
        let out_record = self.records.get(&tokenOut).unwrap();
        calc_spot_price(
            in_record.balance,
            in_record.denorm,
            out_record.balance,
            out_record.denorm,
            self.swap_fee,
        )
    }

    pub fn getSpotPriceSansFee(&self, tokenIn: AccountId, tokenOut: AccountId) -> Balance {
        assert!(self.isBound(tokenIn.clone()), "ERR_NOT_BOUND");
        assert!(self.isBound(tokenOut.clone()), "ERR_NOT_BOUND");
        let in_record = self.records.get(&tokenIn).unwrap();
        let out_record = self.records.get(&tokenOut).unwrap();
        calc_spot_price(
            in_record.balance,
            in_record.denorm,
            out_record.balance,
            out_record.denorm,
            0,
        )
    }

    pub fn joinPool(&mut self, poolAmountOut: Balance, maxAmountsIn: Vec<Balance>) {
        assert!(self.finalized, "ERR_NOT_FINALIZED");
        let pool_total = self.token.get_total_supply();
        let ratio = poolAmountOut / pool_total;
        assert_ne!(ratio, 0, "ERR_MAX_APPROX");

        for i in 0..self.tokens.len() {
            let mut record = self.records.get(&self.tokens[i]).unwrap();
            let token_amount_in = ratio * record.balance;
            assert_ne!(token_amount_in, 0, "ERR_MATH_APPROX");
            assert!(token_amount_in <= maxAmountsIn[i], "ERR_LIMIT_IN");
            record.balance += token_amount_in;
            self.pull_underlying(
                &self.tokens[i].clone(),
                &env::predecessor_account_id(),
                token_amount_in,
            );
            // TODO: join all promises and only save records / mint shares on success.
        }
        self.mint_pool_share(poolAmountOut);
        self.push_pool_share(env::predecessor_account_id(), poolAmountOut);
    }

    pub fn exitPool(&mut self, poolAmountIn: Balance, minAmountsOut: Vec<Balance>) {
        assert!(self.finalized, "ERR_NOT_FINALIZED");

        let pool_total = self.token.get_total_supply();
        let exit_fee = poolAmountIn * EXIT_FEE;
        let p_ai_after_exit_fee = poolAmountIn - exit_fee;
        let ratio = p_ai_after_exit_fee / pool_total;
        assert_ne!(ratio, 0, "ERR_MATH_APPROX");

        self.pull_pool_share(env::predecessor_account_id(), poolAmountIn);
        self.push_pool_share(self.factory.clone(), exit_fee);
        self.burn_pool_share(p_ai_after_exit_fee);

        for i in 0..self.tokens.len() {
            let mut record = self.records.get(&self.tokens[i]).unwrap();
            let token_amount_out = ratio * record.balance;
            assert_ne!(token_amount_out, 0, "ERR_MATH_APPROX");
            assert!(token_amount_out >= minAmountsOut[i], "ERR_LIMIT_OUT");
            record.balance += token_amount_out;
            self.push_underlying(
                self.tokens[i].clone(),
                env::predecessor_account_id(),
                token_amount_out,
            );
        }
    }

    pub fn on_pull(&mut self) -> bool {
        true
    }

    pub fn on_push(&mut self) -> bool {
        true
    }
}

#[near_bindgen]
impl FungibleToken for BPool {
    fn inc_allowance(&mut self, escrow_account_id: String, amount: U128) {
        self.token.inc_allowance(escrow_account_id, amount.into());
    }

    fn dec_allowance(&mut self, escrow_account_id: String, amount: U128) {
        self.token.dec_allowance(escrow_account_id, amount.into());
    }

    fn transfer_from(&mut self, owner_id: String, new_owner_id: String, amount: U128) {
        self.token
            .transfer_from(owner_id, new_owner_id, amount.into());
    }

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

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn on_pull(&mut self) -> bool;

    fn on_push(&mut self) -> bool;
}

impl BPool {
    fn pull_underlying(&mut self, token: &AccountId, from: &AccountId, amount: Balance) -> Promise {
        ext_nep21::transfer_from(
            from.clone(),
            env::current_account_id(),
            amount.into(),
            token,
            NO_DEPOSIT,
            gas::NEP21_TRANSFER_FROM,
        )
        .then(ext_self::on_pull(
            &env::current_account_id(),
            NO_DEPOSIT,
            gas::ON_PULL_CALLBACK,
        ))
    }

    fn push_underlying(&mut self, token: AccountId, to: AccountId, amount: Balance) -> Promise {
        ext_nep21::transfer(
            to.clone(),
            amount.into(),
            &token,
            NO_DEPOSIT,
            gas::NEP21_TRANSFER,
        )
        .then(ext_self::on_push(
            &env::current_account_id(),
            NO_DEPOSIT,
            gas::ON_PUSH_CALLBACK,
        ))
    }

    fn mint_pool_share(&mut self, amount: Balance) {
        self.token.mint(env::current_account_id(), amount)
    }

    fn burn_pool_share(&mut self, amount: Balance) {
        self.token.burn(env::current_account_id(), amount)
    }

    fn pull_pool_share(&mut self, from: AccountId, amount: Balance) {
        self.token
            .transfer_from(from, env::current_account_id(), amount)
    }

    fn push_pool_share(&mut self, to: AccountId, amount: Balance) {
        self.token
            .transfer_from(env::current_account_id(), to, amount)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::{testing_env, MockedBlockchain, VMContext};

    fn pool_account() -> AccountId {
        "pool".to_string()
    }
    fn factory_account() -> AccountId {
        "factory".to_string()
    }
    fn token1_account() -> AccountId {
        "token1".to_string()
    }
    fn token2_account() -> AccountId {
        "token2".to_string()
    }

    pub fn get_context(
        predecessor_account_id: AccountId,
        account_balance: u128,
        account_locked_balance: u128,
        is_view: bool,
    ) -> VMContext {
        VMContext {
            current_account_id: pool_account(),
            signer_account_id: predecessor_account_id.clone(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 1,
            block_timestamp: 0,
            epoch_height: 1,
            account_balance,
            account_locked_balance,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(15),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
        }
    }

    fn to_yocto(amount: Balance) -> Balance {
        amount * 10u128.pow(24)
    }

    #[test]
    fn test_setup_pool() {
        let context = get_context(factory_account(), to_yocto(10), 0, false);
        testing_env!(context.clone());
        let mut pool = BPool::new();
        assert_eq!(pool.getController(), factory_account());
        pool.bind(
            token1_account(),
            to_yocto(50_000).into(),
            to_yocto(10).into(),
        );
        pool.bind(
            token2_account(),
            to_yocto(1_000_000).into(),
            to_yocto(10).into(),
        );
        pool.finalize();
        assert_eq!(pool.getSpotPrice(token1_account(), token2_account()), 1);
    }
}
