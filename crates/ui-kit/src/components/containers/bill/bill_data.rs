use super::bill_conversion::BillConversion;
use super::bill_item::BillItem;
use super::bill_status::BillStatus;
use super::currency::{format_currency, CurrencyPosition};

#[derive(Debug, Clone, PartialEq)]
pub struct BillData {
    pub id: String,
    pub issue_date: String,
    pub due_date: String,
    pub status: BillStatus,
    pub currency: String,
    /// Which side of every amount [`Self::currency`] is written on.
    pub currency_position: CurrencyPosition,
    /// A second currency the bill can also be read in. See [`BillConversion`].
    pub conversion: Option<BillConversion>,
    pub items: Vec<BillItem>,
    pub tax_rate: f64,
    pub discount: f64,
    pub shipping_fee: f64,
    pub notes: Option<String>,
    pub decimal_places: usize,
}

impl BillData {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            issue_date: "".to_string(),
            due_date: "".to_string(),
            status: BillStatus::Pending,
            currency: "$".to_string(),
            currency_position: CurrencyPosition::Prefix,
            conversion: None,
            items: Vec::new(),
            tax_rate: 0.0,
            discount: 0.0,
            shipping_fee: 0.0,
            notes: None,
            decimal_places: 2,
        }
    }

    pub fn with_decimal_places(mut self, decimal_places: usize) -> Self {
        self.decimal_places = decimal_places;
        self
    }

    /// Writes the currency before or after every amount. Defaults to before it.
    pub fn with_currency_position(mut self, position: CurrencyPosition) -> Self {
        self.currency_position = position;
        self
    }

    /// Gives every amount a reading in a second currency, in parentheses beside it.
    pub fn with_conversion(mut self, conversion: BillConversion) -> Self {
        self.conversion = Some(conversion);
        self
    }

    /// An amount of this bill's own currency, written the way this bill writes them.
    pub fn format_amount(&self, value: f64) -> String {
        format_currency(
            value,
            self.decimal_places,
            &self.currency,
            self.currency_position,
        )
    }

    /// The same amount read in the second currency, when the bill has one.
    pub fn format_conversion(&self, value: f64) -> Option<String> {
        self.conversion
            .as_ref()
            .map(|conversion| conversion.format(value))
    }

    pub fn with_dates(mut self, issue_date: impl Into<String>, due_date: impl Into<String>) -> Self {
        self.issue_date = issue_date.into();
        self.due_date = due_date.into();
        self
    }

    pub fn with_status(mut self, status: BillStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn with_items(mut self, items: Vec<BillItem>) -> Self {
        self.items = items;
        self
    }

    pub fn with_tax_rate(mut self, tax_rate: f64) -> Self {
        self.tax_rate = tax_rate;
        self
    }

    pub fn with_discount(mut self, discount: f64) -> Self {
        self.discount = discount;
        self
    }

    pub fn with_shipping_fee(mut self, shipping_fee: f64) -> Self {
        self.shipping_fee = shipping_fee;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    pub fn subtotal(&self) -> f64 {
        self.items.iter().map(|item| item.subtotal()).sum()
    }

    pub fn item_discounts(&self) -> f64 {
        self.items.iter().map(|item| item.discount).sum()
    }

    pub fn total_item_taxes(&self) -> f64 {
        self.items.iter().map(|item| item.tax_amount()).sum()
    }

    pub fn global_tax(&self) -> f64 {
        let base = (self.subtotal() - self.item_discounts() - self.discount).max(0.0);
        base * (self.tax_rate / 100.0)
    }

    pub fn total_tax(&self) -> f64 {
        self.total_item_taxes() + self.global_tax()
    }

    pub fn total_sum(&self) -> f64 {
        let sub = self.subtotal();
        let disc = self.item_discounts() + self.discount;
        let tax = self.total_tax();
        sub - disc + tax + self.shipping_fee
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BillConversion;

    /// The pair the bill draws for every amount: what is charged, and what that reads as.
    fn chain_bill() -> BillData {
        BillData::new("Register a name")
            .with_currency("SOL")
            .with_currency_position(CurrencyPosition::Suffix)
            .with_decimal_places(9)
            .with_conversion(BillConversion::new(76.2, "USDC"))
            .with_items(vec![
                BillItem::new("rent", "Rent-exempt deposit", 1.0, 0.001357),
                BillItem::new("fee", "Network fee", 1.0, 0.000005),
            ])
    }

    #[test]
    fn a_bill_writes_its_own_currency_where_it_was_told_to() {
        let bill = chain_bill();
        assert_eq!(bill.format_amount(0.001357), "0.001357000 SOL");
        assert_eq!(
            BillData::new("i").format_amount(12.0),
            "$ 12.00",
            "a bill that says nothing keeps the leading symbol it always had"
        );
    }

    /// The reading of each line comes from that line, and the reading of the total from the total.
    #[test]
    fn every_amount_is_read_in_the_second_currency() {
        let bill = chain_bill();
        assert_eq!(
            bill.format_conversion(0.001357).as_deref(),
            Some("0.10 USDC")
        );
        assert_eq!(bill.format_amount(bill.total_sum()), "0.001362000 SOL");
        assert_eq!(
            bill.format_conversion(bill.total_sum()).as_deref(),
            Some("0.10 USDC")
        );
    }

    /// The network fee is a real charge, and two places would have called it nothing.
    #[test]
    fn a_line_too_small_for_two_places_still_reads_as_a_charge() {
        assert_eq!(
            chain_bill().format_conversion(0.000005).as_deref(),
            Some("0.00038 USDC")
        );
    }

    /// A bill with no rate to convert at is a bill in one currency, not a bill full of blanks.
    #[test]
    fn a_bill_without_a_conversion_reads_in_one_currency() {
        let bill = BillData::new("i").with_currency("SOL");
        assert_eq!(bill.format_conversion(1.0), None);
    }
}
