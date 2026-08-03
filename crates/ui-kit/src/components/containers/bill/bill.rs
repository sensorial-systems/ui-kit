use super::super::card::Card;
use super::bill_data::BillData;
use dioxus::prelude::*;

#[component]
pub fn Bill(
    bill: BillData,
    #[props(into, default)] class: Option<String>,
) -> Element {
    let currency = &bill.currency;
    let decimals = bill.decimal_places;
    let total_sum = bill.total_sum();
    let formatted_total = format!("{:.*}", decimals, total_sum);
    let total_color_class = if total_sum > 1e-9 {
        "uikit-bill-item-positive"
    } else if total_sum < -1e-9 {
        "uikit-bill-item-negative"
    } else {
        "uikit-bill-item-neutral"
    };

    let header_elem = rsx! {
        div {
            class: "uikit-bill-header",
            span { class: "uikit-bill-id", "{bill.id}" }
        }
    };

    let extra_class = class.as_deref().unwrap_or_default();

    rsx! {
        div {
            class: "uikit-bill {extra_class}",
            Card {
                header: header_elem,
                div {
                    class: "uikit-bill-items",
                    if bill.items.is_empty() {
                        div { class: "uikit-bill-empty", "No items" }
                    } else {
                        {
                            bill.items.iter().map(|item| {
                                let item_total = item.total();
                                let formatted_item_total = format!("{:.*}", decimals, item_total);
                                let color_class = if item_total > 1e-9 {
                                    "uikit-bill-item-positive"
                                } else if item_total < -1e-9 {
                                    "uikit-bill-item-negative"
                                } else {
                                    "uikit-bill-item-neutral"
                                };
                                rsx! {
                                    div {
                                        key: "{item.id}",
                                        class: "uikit-bill-item-row",
                                        span { class: "uikit-bill-item-desc", "{item.description}" }
                                        span { class: "uikit-bill-item-qty", "x{item.amount}" }
                                        span { class: "uikit-bill-item-total {color_class}", "{currency} {formatted_item_total}" }
                                    }
                                }
                            })
                        }
                    }
                    // Total Row inside items container for 100% aligned columns
                    div {
                        class: "uikit-bill-item-row uikit-bill-total-row",
                        span { class: "uikit-bill-total-label", "Total" }
                        span { class: "uikit-bill-total-qty", "" }
                        span { class: "uikit-bill-total-value {total_color_class}", "{currency} {formatted_total}" }
                    }
                }
            }
        }
    }
}
