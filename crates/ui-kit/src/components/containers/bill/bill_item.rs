#[derive(Debug, Clone, PartialEq)]
pub struct BillItem {
    pub id: String,
    pub description: String,
    pub category: Option<String>,
    pub amount: f64,
    pub cost: f64,
    pub discount: f64,
    pub tax_rate: f64,
}

impl BillItem {
    pub fn new(id: impl Into<String>, description: impl Into<String>, amount: f64, cost: f64) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            category: None,
            amount,
            cost,
            discount: 0.0,
            tax_rate: 0.0,
        }
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn with_discount(mut self, discount: f64) -> Self {
        self.discount = discount;
        self
    }

    pub fn with_tax_rate(mut self, tax_rate: f64) -> Self {
        self.tax_rate = tax_rate;
        self
    }

    pub fn subtotal(&self) -> f64 {
        self.amount * self.cost
    }

    pub fn tax_amount(&self) -> f64 {
        let base = (self.subtotal() - self.discount).max(0.0);
        base * (self.tax_rate / 100.0)
    }

    pub fn total(&self) -> f64 {
        let base = (self.subtotal() - self.discount).max(0.0);
        base + self.tax_amount()
    }
}
