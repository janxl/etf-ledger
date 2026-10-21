use std::fmt;

/// An amount of money in minor units (cents), stored as a whole number.
///
/// Integers avoid the rounding errors of floating point. Negative values are
/// allowed: the sign is how a ledger tells debits from credits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money(pub i64);
impl Money {
    /// The amount in cents.
    pub fn cents(&self) -> i64 {
        self.0
    }
}

/// Formats as dollars, for example `$10.50` or `-$0.05`.
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        write!(f, "{}${}.{:02}", sign, abs / 100, abs % 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_money_displays_correctly() {
        assert_eq!(Money(-5).to_string(), "-$0.05");
    }

    #[test]
    fn display_handles_extremes() {
        assert_eq!(Money(0).to_string(), "$0.00");
        assert_eq!(Money(i64::MIN).to_string(), "-$92233720368547758.08");
    }
}
