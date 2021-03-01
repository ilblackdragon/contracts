use near_sdk::Balance;

const MAX_RESERVE_RATIO: u32 = 1_000_000;

/// Given continues token supply, reserve balance and reserve ratio, return how much tokens will be purchased with given `deposit_amount`.
/// Formula:
///     return = supply * ((1 + deposit_amount / reserve_balance) ^ (reserve_ratio / MAX_RESERVE_RATIO) - 1)
pub(crate) fn calc_purchase_amount(
    supply: Balance,
    reserve_balance: Balance,
    reserve_ratio: u32,
    deposit_amount: Balance,
) -> Balance {
    assert!(
        supply > 0 && reserve_balance > 0 && reserve_ratio > 0 && deposit_amount > 0,
        "ERR_INPUT_ZERO"
    );
    if reserve_ratio == MAX_RESERVE_RATIO {
        return supply * deposit_amount / reserve_balance;
    }

    (supply as f64
        * ((1f64 + deposit_amount as f64 / reserve_balance as f64)
            .powf(reserve_ratio as f64 / MAX_RESERVE_RATIO as f64)
            - 1f64))
        .ceil() as u128
}

/// Given total supply, reserve balance and reserve ratio, calculate how much reserve to return for given number of tokens to sell.
/// Formula:
///     return = reserve_balance * (1 - (1 - sell_amount / supply) ^ (1 / (reserve_ration / MAX_RESERVE_RATIO)))
pub(crate) fn calc_sale_amount(
    supply: Balance,
    reserve_balance: Balance,
    reserve_ratio: u32,
    sell_amount: Balance,
) -> Balance {
    assert!(
        supply > 0 && reserve_balance > 0 && reserve_ratio > 0 && sell_amount > 0,
        "ERR_INPUT_ZERO"
    );
    if sell_amount == supply {
        return reserve_balance;
    } else if reserve_ratio == MAX_RESERVE_RATIO {
        return reserve_balance * sell_amount / supply;
    }
    (reserve_balance as f64
        * (1f64
            - (1f64 - sell_amount as f64 / supply as f64)
                .powf(MAX_RESERVE_RATIO as f64 / reserve_ratio as f64)))
    .floor() as u128
}
