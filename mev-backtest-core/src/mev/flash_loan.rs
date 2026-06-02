use alloy::primitives::U256;

use crate::types::FlashLoanProvider;

impl FlashLoanProvider {
    /// Flash loan fee in basis points (1 bps = 0.01%).
    pub fn fee_bps(self) -> u64 {
        match self {
            FlashLoanProvider::Auto => 5, // default to Aave fee
            FlashLoanProvider::Balancer => 0, // free
            FlashLoanProvider::Aave => 5,  // 0.05%
            FlashLoanProvider::Uniswap => 3, // 0.03% (flash swap)
        }
    }

    /// Calculate the flash loan fee in wei for a given borrow amount.
    pub fn fee_on(self, amount: U256) -> U256 {
        let fee_bps = self.fee_bps();
        if fee_bps == 0 {
            return U256::ZERO;
        }
        amount
            .checked_mul(U256::from(fee_bps))
            .map(|v| v / U256::from(10000u64))
            .unwrap_or(U256::ZERO)
    }
}

/// Calculate net profit after flash loan fee.
pub fn flash_loan_net_profit(
    gross_profit: U256,
    flash_loan_amount: U256,
    provider: FlashLoanProvider,
) -> U256 {
    let fee = provider.fee_on(flash_loan_amount);
    gross_profit.saturating_sub(fee)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balancer_no_fee() {
        let amount = U256::from(1_000_000_000_000_000_000u128);
        assert_eq!(FlashLoanProvider::Balancer.fee_on(amount), U256::ZERO);
    }

    #[test]
    fn test_aave_fee() {
        let amount = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
        let fee = FlashLoanProvider::Aave.fee_on(amount);
        // 5 bps = 0.05% = 5/10000
        assert_eq!(fee, U256::from(500_000_000_000_000u128)); // 5e14
    }

    #[test]
    fn test_auto_fee_defaults_to_5bps() {
        let amount = U256::from(1_000_000_000_000_000_000u128);
        assert_eq!(FlashLoanProvider::Auto.fee_on(amount), FlashLoanProvider::Aave.fee_on(amount));
    }

    #[test]
    fn test_zero_amount() {
        assert_eq!(FlashLoanProvider::Aave.fee_on(U256::ZERO), U256::ZERO);
    }

    #[test]
    fn test_net_profit() {
        let gross = U256::from(100_000_000_000_000_000u128);
        let amount = U256::from(1_000_000_000_000_000_000u128);
        let net = flash_loan_net_profit(gross, amount, FlashLoanProvider::Aave);
        assert_eq!(net, U256::from(99_500_000_000_000_000u128));
    }

    #[test]
    fn test_net_profit_saturated() {
        let gross = U256::from(50_000_000_000_000u128);
        let amount = U256::from(1_000_000_000_000_000_000u128);
        let net = flash_loan_net_profit(gross, amount, FlashLoanProvider::Aave);
        // gross (5e13) < fee (1e14) → net = 0
        assert_eq!(net, U256::ZERO);
    }

    #[test]
    fn test_fee_bps_values() {
        assert_eq!(FlashLoanProvider::Balancer.fee_bps(), 0);
        assert_eq!(FlashLoanProvider::Aave.fee_bps(), 5);
        assert_eq!(FlashLoanProvider::Uniswap.fee_bps(), 3);
        assert_eq!(FlashLoanProvider::Auto.fee_bps(), 5);
    }
}
