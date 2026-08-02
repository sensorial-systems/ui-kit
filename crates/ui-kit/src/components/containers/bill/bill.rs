use super::super::card::Card;
use super::bill_data::BillData;
use dioxus::prelude::*;

#[component]
pub fn Bill(
    bill: BillData,
) -> Element {
    let currency = &bill.currency;
    let total_sum = bill.total_sum();

    let header_elem = rsx! {
        div {
            class: "uikit-bill-header",
            span { class: "uikit-bill-id", "#{bill.id}" }
        }
    };

    rsx! {
        div {
            class: "uikit-bill",
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
                                rsx! {
                                    div {
                                        key: "{item.id}",
                                        class: "uikit-bill-item-row",
                                        span { class: "uikit-bill-item-desc", "{item.description}" }
                                        span { class: "uikit-bill-item-qty", "x{item.amount}" }
                                        span { class: "uikit-bill-item-total", "{currency} {item_total:.2}" }
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
                        span { class: "uikit-bill-total-value", "{currency} {total_sum:.2}" }
                    }
                }
            }
        }
    }
}
