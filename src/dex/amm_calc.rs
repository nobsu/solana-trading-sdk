pub fn amm_buy_get_sol_in(sol_reserve: u64, token_reserve: u64, token_out: u64) -> u64 {
    if token_out == 0 || sol_reserve == 0 || token_reserve == 0 || token_out >= token_reserve {
        return 0;
    }

    let sol_reserve = sol_reserve as u128;
    let token_reserve = token_reserve as u128;
    let token_in = token_out as u128;
    let numerator = sol_reserve.checked_mul(token_in).unwrap();
    let denominator = token_reserve.checked_sub(token_in).unwrap();
    let sol_out = numerator.checked_div(denominator).unwrap();

    sol_out as u64
}

pub fn amm_buy_get_token_out(sol_reserve: u64, token_reserve: u64, sol_in: u64) -> u64 {
    if sol_in == 0 || sol_reserve == 0 || token_reserve == 0 {
        return 0;
    }

    let invariant = sol_reserve as u128 * token_reserve as u128;
    let new_sol_reserve = sol_reserve as u128 + sol_in as u128;

    let new_token_reserve = invariant / new_sol_reserve;
    let token_out = token_reserve as u128 - new_token_reserve;

    token_out as u64
}

pub fn amm_sell_get_sol_out(sol_reserve: u64, token_reserve: u64, token_in: u64) -> u64 {
    if token_in == 0 || sol_reserve == 0 || token_reserve == 0 {
        return 0;
    }

    let sol_reserve = sol_reserve as u128;
    let token_reserve = token_reserve as u128;
    let token_in = token_in as u128;
    let numerator = sol_reserve.checked_mul(token_in).unwrap();
    let denominator = token_reserve.checked_add(token_in).unwrap();
    let sol_out = numerator.checked_div(denominator).unwrap();

    sol_out as u64
}
