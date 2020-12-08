use near_sdk::Balance;

pub type Weight = u128;

pub const BONE: Balance = 1_000_000_000_000_000_000_000_000;

pub const MIN_BOUND_TOKENS: usize = 2;
pub const MAX_BOUND_TOKENS: usize = 8;

pub const MIN_FEE: Balance = BONE / 1_000_000;
pub const MAX_FEE: Balance = BONE / 10;
pub const EXIT_FEE: Balance = 0;

pub const MIN_WEIGHT: Weight = BONE;
pub const MAX_WEIGHT: Weight = BONE * 50;
pub const MAX_TOTAL_WEIGHT: Weight = BONE * 50;
pub const MIN_BALANCE: Balance = BONE / 1_000_000_000_000;

pub const INIT_POOL_SUPPLY: Balance = BONE * 100;
pub const MIN_BPOW_BASE: Balance = 1;
pub const MAX_BPOW_BASE: Balance = 2 * BONE - 1;
pub const BPOW_PRECISION: Balance = BONE / 10_000_000_000;

pub const MAX_IN_RATIO: Balance = BONE / 2;
pub const MAX_OUT_RATIO: Balance = BONE / 3 + 1;

pub const NO_DEPOSIT: Balance = 0;

pub mod gas {
    pub const BASE_GAS: u64 = 20_000_000_000_000;

    pub const NEP21_TRANSFER: u64 = BASE_GAS;

    pub const ON_PULL_CALLBACK: u64 = BASE_GAS;

    pub const NEP21_TRANSFER_FROM: u64 = BASE_GAS;

    pub const ON_PUSH_CALLBACK: u64 = BASE_GAS;
}
