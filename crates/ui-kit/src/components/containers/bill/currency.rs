/// Which side of an amount its currency is written on.
///
/// Symbols lead and tickers follow — `$ 12.00`, but `12.00 USDC` — and which of the two a bill
/// wants is not something the bill can work out from the string it was handed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurrencyPosition {
    /// `$ 12.00`.
    #[default]
    Prefix,
    /// `12.00 USDC`.
    Suffix,
}

/// An amount written with its currency, on the side asked for.
pub fn format_currency(
    value: f64,
    decimal_places: usize,
    currency: &str,
    position: CurrencyPosition,
) -> String {
    let amount = format_amount(value, decimal_places);
    match position {
        CurrencyPosition::Prefix => format!("{currency} {amount}"),
        CurrencyPosition::Suffix => format!("{amount} {currency}"),
    }
}

/// An amount to the places asked for, and to more of them when that many would round a real
/// charge away to nothing.
///
/// Every line of a bill is something somebody pays. Two places is the right precision for almost
/// any currency and the wrong one for a fee of a fraction of a cent: `0.00` does not read as
/// "small", it reads as "free", and that is a different claim about somebody's money. So the
/// places asked for are a floor rather than a ceiling, and an amount too small to survive them is
/// written out until it is a number again — as far as a `f64` can still tell it from zero, and no
/// further.
///
/// One place further than that, in fact. The first width that shows anything shows a single
/// digit, and a single digit is reached by rounding: at two places `0.000005` becomes `0.00001`,
/// which is twice what was charged. A second digit costs one character and stops the smallest
/// amounts on a bill from being the least accurate ones.
pub fn format_amount(value: f64, decimal_places: usize) -> String {
    /// Past this a `f64` has nothing left to say, and the search below has to stop somewhere.
    const MAX_PLACES: usize = 18;

    let rounds_to_nothing = |places: usize| {
        format!("{value:.places$}")
            .parse::<f64>()
            .is_ok_and(|rounded| rounded == 0.0)
    };

    if value == 0.0 || !rounds_to_nothing(decimal_places) {
        return format!("{value:.decimal_places$}");
    }
    let mut places = decimal_places;
    while places < MAX_PLACES && rounds_to_nothing(places) {
        places += 1;
    }
    let places = (places + 1).min(MAX_PLACES);
    format!("{value:.places$}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_symbol_leads_and_a_ticker_follows() {
        assert_eq!(
            format_currency(12.0, 2, "$", CurrencyPosition::Prefix),
            "$ 12.00"
        );
        assert_eq!(
            format_currency(12.0, 2, "USDC", CurrencyPosition::Suffix),
            "12.00 USDC"
        );
    }

    /// The ordinary case: the places asked for are the places written.
    #[test]
    fn an_amount_is_written_to_the_places_asked_for() {
        assert_eq!(format_amount(0.11, 2), "0.11");
        assert_eq!(format_amount(1.5, 2), "1.50");
        assert_eq!(format_amount(0.004251, 9), "0.004251000");
    }

    /// A fee of a fraction of a cent is charged, and `0.00` would say it was not.
    #[test]
    fn an_amount_too_small_for_those_places_is_written_out_further() {
        assert_eq!(format_amount(0.00038, 2), "0.00038");
        assert_eq!(format_amount(0.000005, 2), "0.000005");
        assert_eq!(format_amount(-0.00038, 2), "-0.00038");
    }

    /// Widening stops at two digits, not at the one that rounding would leave: `0.00001` is twice
    /// the fee that was actually charged.
    #[test]
    fn a_widened_amount_keeps_a_second_significant_digit() {
        assert_eq!(format_amount(0.000005, 2), "0.000005");
        assert_eq!(format_amount(1.234e-7, 2), "0.00000012");
    }

    /// An amount that survives the places asked for is written in them, rounding and all — this
    /// widens what would vanish, it does not chase precision for its own sake.
    #[test]
    fn an_amount_that_survives_is_left_alone() {
        assert_eq!(format_amount(0.009, 2), "0.01");
        assert_eq!(format_amount(1.239, 2), "1.24");
    }

    /// Nothing is nothing, and it is written the way the bill writes everything else.
    #[test]
    fn nothing_keeps_the_places_asked_for() {
        assert_eq!(format_amount(0.0, 2), "0.00");
        assert_eq!(format_amount(0.0, 9), "0.000000000");
    }

    /// An amount smaller than a `f64` can hold apart from zero still has to come back as a
    /// string, rather than spinning looking for a place that will show it.
    #[test]
    fn an_amount_beyond_precision_still_terminates() {
        assert_eq!(format_amount(1e-30, 2).len(), "0.".len() + 18);
    }
}
