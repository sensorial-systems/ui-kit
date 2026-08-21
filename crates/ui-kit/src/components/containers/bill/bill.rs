use super::super::card::Card;
use super::bill_data::BillData;
use dioxus::prelude::*;

/// Which of the three ways an amount can go the colour says it went.
fn amount_class(value: f64) -> &'static str {
    if value > 1e-9 {
        "uikit-bill-item-positive"
    } else if value < -1e-9 {
        "uikit-bill-item-negative"
    } else {
        "uikit-bill-item-neutral"
    }
}

#[component]
pub fn Bill(bill: BillData, #[props(into, default)] class: Option<String>) -> Element {
    let total_sum = bill.total_sum();
    let formatted_total = bill.format_amount(total_sum);
    let converted_total = bill.format_conversion(total_sum);
    let total_color_class = amount_class(total_sum);

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
                                let formatted_item_total = bill.format_amount(item_total);
                                // Converted from this line, not carried down from the one above:
                                // a reading of an amount is only ever a reading of that amount.
                                let converted_item_total = bill.format_conversion(item_total);
                                let color_class = amount_class(item_total);
                                rsx! {
                                    div {
                                        key: "{item.id}",
                                        class: "uikit-bill-item-row",
                                        span { class: "uikit-bill-item-desc", "{item.description}" }
                                        span { class: "uikit-bill-item-qty", "x{item.amount}" }
                                        span { class: "uikit-bill-item-total {color_class}",
                                            "{formatted_item_total}"
                                            if let Some(converted) = converted_item_total {
                                                span { class: "uikit-bill-conversion", "({converted})" }
                                            }
                                        }
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
                        span { class: "uikit-bill-total-value {total_color_class}",
                            "{formatted_total}"
                            if let Some(converted) = converted_total {
                                span { class: "uikit-bill-conversion", "({converted})" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{BillConversion, BillItem, CurrencyPosition};
    use super::*;

    fn render(bill: BillData) -> String {
        #[component]
        fn Harness(bill: BillData) -> Element {
            rsx! { Bill { bill } }
        }

        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { bill });
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    fn chain_bill() -> BillData {
        BillData::new("Register a name")
            .with_currency("SOL")
            .with_currency_position(CurrencyPosition::Suffix)
            .with_decimal_places(9)
            .with_conversion(BillConversion::new(76.2, "USDC"))
            .with_items(vec![BillItem::new("fee", "Network fee", 1.0, 0.000005)])
    }

    /// What a browser is handed for a converted bill: the charge, then its reading in its own
    /// span, on the line and on the total alike.
    #[test]
    fn a_converted_bill_draws_both_currencies() {
        let markup = render(chain_bill());
        assert!(
            markup.contains(
                r#"<span class="uikit-bill-item-total uikit-bill-item-positive">0.000005000 SOL<span class="uikit-bill-conversion">(0.00038 USDC)</span></span>"#
            ),
            "{markup}"
        );
        assert!(
            markup.contains(r#"uikit-bill-total-value uikit-bill-item-positive">0.000005000 SOL<span class="uikit-bill-conversion">(0.00038 USDC)</span>"#),
            "{markup}"
        );
    }

    /// Without a rate the bill is the bill it always was, with no empty parentheses left behind.
    #[test]
    fn an_unconverted_bill_draws_one_currency() {
        let mut bill = chain_bill();
        bill.conversion = None;
        let markup = render(bill);
        assert!(markup.contains("0.000005000 SOL"), "{markup}");
        assert!(!markup.contains("uikit-bill-conversion"), "{markup}");
        assert!(!markup.contains("()"), "{markup}");
    }

    /// The default is what every bill written before any of this existed still gets.
    #[test]
    fn a_plain_bill_still_leads_with_its_symbol() {
        let markup = render(
            BillData::new("INV-1").with_items(vec![BillItem::new("i", "Service", 2.0, 50.0)]),
        );
        assert!(markup.contains("$ 100.00"), "{markup}");
    }
}
