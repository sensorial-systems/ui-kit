use crate::components::info::BadgeVariant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BillStatus {
    #[default]
    Pending,
    Paid,
    Overdue,
    Draft,
    Cancelled,
    Refunded,
}

impl BillStatus {
    pub fn label(&self) -> &'static str {
        match self {
            BillStatus::Pending => "Pending Payment",
            BillStatus::Paid => "Paid",
            BillStatus::Overdue => "Overdue",
            BillStatus::Draft => "Draft",
            BillStatus::Cancelled => "Cancelled",
            BillStatus::Refunded => "Refunded",
        }
    }

    pub fn badge_variant(&self) -> BadgeVariant {
        match self {
            BillStatus::Pending => BadgeVariant::Warning,
            BillStatus::Paid => BadgeVariant::Success,
            BillStatus::Overdue => BadgeVariant::Error,
            BillStatus::Draft => BadgeVariant::Default,
            BillStatus::Cancelled => BadgeVariant::Default,
            BillStatus::Refunded => BadgeVariant::Info,
        }
    }
}
