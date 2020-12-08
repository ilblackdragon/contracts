use near_sdk::Balance;

use crate::bconst::{Weight, BONE};

/**********************************************************************************************
// calcSpotPrice                                                                             //
// sP = spotPrice                                                                            //
// bI = tokenBalanceIn                ( bI / wI )         1                                  //
// bO = tokenBalanceOut         sP =  -----------  *  ----------                             //
// wI = tokenWeightIn                 ( bO / wO )     ( 1 - sF )                             //
// wO = tokenWeightOut                                                                       //
// sF = swapFee                                                                              //
**********************************************************************************************/
pub fn calc_spot_price(
    balance_in: Balance,
    weight_in: Weight,
    balace_out: Balance,
    weight_out: Weight,
    swap_fee: Balance,
) -> Balance {
    let numer = balance_in / weight_in;
    let denom = balace_out / weight_out;
    let ratio = numer / denom;
    let scale = BONE / (BONE - swap_fee);
    ratio * scale
}
