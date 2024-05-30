use std::ops::{Add, Deref, DerefMut, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenAmount(u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StakedTokenAmount(u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LpTokenAmount(u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Price(u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fee(u64);

#[derive(Debug, Clone)]
struct LpPool {
    price: Price,
    token_amount: TokenAmount,
    st_token_amount: StakedTokenAmount,
    lp_token_amount: LpTokenAmount,
    liquidity_target: TokenAmount,
    min_fee: Fee,
    max_fee: Fee,
}

#[derive(Debug)]
enum Errors {
    InsufficientLiquidity,
    InvalidAmount,
}

// Implement Deref for easier access to the inner value
impl Deref for TokenAmount {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TokenAmount {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for StakedTokenAmount {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StakedTokenAmount {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for LpTokenAmount {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LpTokenAmount {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Implement Add and Sub for arithmetic operations
impl Add for TokenAmount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        TokenAmount(self.0 + other.0)
    }
}

impl Sub for TokenAmount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        TokenAmount(self.0 - other.0)
    }
}

impl Add for StakedTokenAmount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        StakedTokenAmount(self.0 + other.0)
    }
}

impl Sub for StakedTokenAmount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        StakedTokenAmount(self.0 - other.0)
    }
}

impl Add for LpTokenAmount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        LpTokenAmount(self.0 + other.0)
    }
}

impl Sub for LpTokenAmount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        LpTokenAmount(self.0 - other.0)
    }
}

// Define LpPool struct and methods
impl LpPool {
    pub fn init(
        price: Price,
        min_fee: Fee,
        max_fee: Fee,
        liquidity_target: TokenAmount,
    ) -> Result<Self, Errors> {
        Ok(LpPool {
            price,
            token_amount: TokenAmount(0),
            st_token_amount: StakedTokenAmount(0),
            lp_token_amount: LpTokenAmount(0),
            liquidity_target,
            min_fee,
            max_fee,
        })
    }

    pub fn add_liquidity(self, token_amount: TokenAmount) -> Result<(Self, LpTokenAmount), Errors> {
        let new_token_amount = self.token_amount + token_amount;
        let lp_tokens_minted = if self.lp_token_amount.0 == 0 {
            token_amount.0
        } else {
            (token_amount.0 as u128 * self.lp_token_amount.0 as u128 / self.token_amount.0 as u128)
                as u64
        };
        let new_lp_token_amount = self.lp_token_amount + LpTokenAmount(lp_tokens_minted);

        let new_pool = LpPool {
            token_amount: new_token_amount,
            lp_token_amount: new_lp_token_amount,
            ..self
        };

        Ok((dbg!(new_pool), LpTokenAmount(lp_tokens_minted)))
    }

    pub fn remove_liquidity(
        self,
        lp_token_amount: LpTokenAmount,
    ) -> Result<(Self, TokenAmount, StakedTokenAmount), Errors> {
        if lp_token_amount.0 > self.lp_token_amount.0 {
            return Err(Errors::InsufficientLiquidity);
        }
        let lp_token_proportion = lp_token_amount.0 as u128 / self.lp_token_amount.0 as u128;

        let token_withdrawn = (self.token_amount.0 as u128 * lp_token_amount.0 as u128) as u64;
        let staked_token_withdrawn = (self.st_token_amount.0 as u128 * lp_token_proportion) as u64;

        let new_token_amount = self.token_amount - TokenAmount(token_withdrawn);
        let new_st_token_amount = self.st_token_amount - StakedTokenAmount(staked_token_withdrawn);
        let new_lp_token_amount = self.lp_token_amount - lp_token_amount;

        let new_pool = LpPool {
            token_amount: new_token_amount,
            st_token_amount: new_st_token_amount,
            lp_token_amount: new_lp_token_amount,
            ..self
        };

        Ok((
            new_pool,
            TokenAmount(token_withdrawn),
            StakedTokenAmount(staked_token_withdrawn),
        ))
    }

    pub fn swap(
        self,
        staked_token_amount: StakedTokenAmount,
    ) -> Result<(Self, TokenAmount), Errors> {
        let token_amount =
            (staked_token_amount.0 as u128 * self.price.0 as u128 / 1_000_000) as u64;

        if token_amount > self.token_amount.0 {
            return Err(Errors::InsufficientLiquidity);
        }

        let new_token_balance = TokenAmount(self.token_amount.0 - token_amount);
        let fee = calculate_fee(
            self.max_fee,
            self.min_fee,
            self.liquidity_target,
            new_token_balance,
        );
        let fee_amount = (token_amount as u128 * fee.0 as u128 / 1_000_000) as u64;

        let token_amount_net = TokenAmount(token_amount - fee_amount);
        let new_st_token_amount = self.st_token_amount + staked_token_amount;

        let new_pool = LpPool {
            token_amount: new_token_balance,
            st_token_amount: new_st_token_amount,
            ..self
        };

        Ok((dbg!(new_pool), token_amount_net))
    }
}

fn calculate_fee(
    max_fee: Fee,
    min_fee: Fee,
    liquidity_target: TokenAmount,
    amount_after: TokenAmount,
) -> Fee {
    let fee = if amount_after.0 >= liquidity_target.0 {
        println!("minimal fee");
        min_fee.0
    } else {
        println!("non minimal fee");
        let fee_diff = max_fee.0 - min_fee.0;
        let fee_adjustment = fee_diff as u128 * amount_after.0 as u128 / liquidity_target.0 as u128;
        max_fee.0 - fee_adjustment as u64
    };

    Fee(fee)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_story_example() {
        let lp_pool = LpPool::init(
            Price(1_500_000),            // 1.5 with 6 decimals precision
            Fee(1_000),                  // 0.1% fee
            Fee(90_000),                 // 9% fee
            TokenAmount(90 * 1_000_000), // 21,000 Tokens with 6 decimals precision
        )
        .unwrap();

        // Step 1: Add liquidity of 100.0 Tokens
        let (lp_pool, lp_tokens) = lp_pool
            .add_liquidity(TokenAmount(100_000 * 1_000_000))
            .unwrap();
        assert_eq!( 100_000 * 1_000_000, lp_tokens.0); // 100.0 LpToken

        // Step 2: Swap 6 StakedToken
        let (lp_pool, received_tokens) = lp_pool.swap(StakedTokenAmount(6 * 1_000_000)).unwrap();
        assert_eq!(8_991_000, received_tokens.0); // 8.991 Tokens received

        // Step 3: Add more liquidity of 10.0 Tokens
        let (lp_pool, more_lp_tokens) = lp_pool
            .add_liquidity(TokenAmount(10_000 * 1_000_000))
            .unwrap();
        assert_eq!(9_999_100 * 1_000_000, more_lp_tokens.0); // 10.0 LpToken

        // Step 4: Swap 30 StakedToken
        let (lp_pool, more_received_tokens) =
            lp_pool.swap(StakedTokenAmount(30 * 1_000_000)).unwrap();
        assert_eq!(43_442_370, more_received_tokens.0); // 43.44237 Tokens received

        // Step 5: Remove liquidity of 109.9991 LpToken
        let (_lp_pool, tokens_withdrawn, staked_tokens_withdrawn) = lp_pool
            .remove_liquidity(LpTokenAmount(109_999_100))
            .unwrap();
        assert_eq!(tokens_withdrawn.0, 57_566_630); // 57.56663 Tokens withdrawn
        assert_eq!(staked_tokens_withdrawn.0, 36 * 1_000_000); // 36 StakedToken withdrawn
    }
}
