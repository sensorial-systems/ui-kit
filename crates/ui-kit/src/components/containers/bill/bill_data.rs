use super::bill_item::BillItem;
use super::bill_status::BillStatus;

#[derive(Debug, Clone, PartialEq)]
pub struct BillData {
    pub id: String,
    pub issue_date: String,
    pub due_date: String,
    pub status: BillStatus,
    pub currency: String,
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
