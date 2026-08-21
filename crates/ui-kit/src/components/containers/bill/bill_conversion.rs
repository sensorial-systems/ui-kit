use super::currency::{format_currency, CurrencyPosition};

/// A second currency the same bill can be read in.
///
/// A bill stays written in what is actually charged: a chain takes its rent in SOL, and a bill
/// that quoted dollars would be quoting a number nobody is paying. But few people have a feel for
/// what a fraction of a SOL is worth, so every amount can also carry what it comes to, at
/// whatever rate the caller last knew.
///
/// It is a reading of the bill and never part of its arithmetic — no total here is reached by
/// adding these up, and each is converted from the amount beside it rather than from the one
/// above. That is also why a rate nobody knows yet is simply no conversion at all: the bill is
/// complete without one.
#[derive(Debug, Clone, PartialEq)]
pub struct BillConversion {
    /// What one unit of the bill's own currency is worth in this one.
    pub rate: f64,
    pub currency: String,
    pub decimal_places: usize,
    pub position: CurrencyPosition,
}

impl BillConversion {
    /// A reading in `currency`, at `rate` of it to one of the bill's own.
    ///
    /// Two places and a trailing ticker, which is how the currencies anyone converts *into* are
    /// written; both are still the caller's to change.
    pub fn new(rate: f64, currency: impl Into<String>) -> Self {
        Self {
            rate,
            currency: currency.into(),
            decimal_places: 2,
            position: CurrencyPosition::Suffix,
        }
    }

    pub fn with_decimal_places(mut self, decimal_places: usize) -> Self {
        self.decimal_places = decimal_places;
        self
    }

    pub fn with_position(mut self, position: CurrencyPosition) -> Self {
        self.position = position;
        self
    }

    /// What an amount of the bill's own currency comes to in this one.
    pub fn convert(&self, value: f64) -> f64 {
        value * self.rate
    }

    /// That amount, written the way this conversion writes them.
    pub fn format(&self, value: f64) -> String {
        format_currency(
            self.convert(value),
            self.decimal_places,
            &self.currency,
            self.position,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_amount_reads_as_what_it_is_worth() {
        let conversion = BillConversion::new(76.2, "USDC");
        assert_eq!(conversion.format(0.001), "0.08 USDC");
    }

    /// The conversion of a charge too small for two places is still a charge.
    #[test]
    fn a_charge_under_a_cent_survives_the_conversion() {
        let conversion = BillConversion::new(76.2, "USDC");
        assert_eq!(conversion.format(0.000005), "0.00038 USDC");
    }

    #[test]
    fn a_refund_converts_to_a_refund() {
        let conversion = BillConversion::new(76.2, "USDC");
        assert_eq!(conversion.format(-0.01), "-0.76 USDC");
    }
}
