use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct HorizontalMenuItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub children: Vec<HorizontalMenuSubItem>,
}

#[derive(Clone, PartialEq)]
pub struct HorizontalMenuSubItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub sub_children: Vec<HorizontalMenuLeafItem>,
}

#[derive(Clone, PartialEq)]
pub struct HorizontalMenuLeafItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub description: Option<String>,
}

#[component]
pub fn HorizontalMenu(
    #[props(default)] items: Vec<HorizontalMenuItem>,
    #[props(default)] on_select: Option<EventHandler<String>>,
    #[props(default)] class: Option<String>,
) -> Element {
    let mut active_item = use_signal(|| None::<String>);
    let mut active_sub_item = use_signal(|| None::<String>);

    let mut is_hovering_bar = use_signal(|| false);
    let mut is_hovering_dropdown = use_signal(|| false);

    let is_open = active_item.read().is_some();
    let current_active_item_id = active_item.read().clone();
    let current_active_sub_id = active_sub_item.read().clone();

    // Find active item data & compute item X offsets
    let mut active_item_index = None;
    let mut item_left_offsets = Vec::new();
    let mut accum_x = 0.0;

    for (idx, item) in items.iter().enumerate() {
        if let Some(ref active_id) = current_active_item_id {
            if &item.id == active_id {
                active_item_index = Some(idx);
            }
        }
        item_left_offsets.push(accum_x);
        let icon_w = if item.icon.is_some() { 24.0 } else { 0.0 };
        let chevron_w = if !item.children.is_empty() { 20.0 } else { 0.0 };
        let text_w = item.label.len() as f64 * 8.2;
        let btn_w = 32.0 + icon_w + chevron_w + text_w;
        accum_x += btn_w;
    }

    let mut last_left_offset = use_signal(|| 0.0);

    let dropdown_left_offset = if let Some(idx) = active_item_index {
        let offset = item_left_offsets.get(idx).copied().unwrap_or(0.0);
        if *last_left_offset.read() != offset {
            last_left_offset.set(offset);
        }
        offset
    } else {
        *last_left_offset.read()
    };

    let active_item_data = items.iter().find(|item| {
        if let Some(ref id) = current_active_item_id {
            &item.id == id
        } else {
            false
        }
    }).cloned();

    let mut last_active_item_data = use_signal(|| None::<HorizontalMenuItem>);
    let mut last_active_sub_id = use_signal(|| None::<String>);

    if let Some(ref data) = active_item_data {
        if last_active_item_data.read().as_ref() != Some(data) {
            last_active_item_data.set(Some(data.clone()));
        }
    }

    if current_active_sub_id.is_some() {
        if *last_active_sub_id.read() != current_active_sub_id {
            last_active_sub_id.set(current_active_sub_id.clone());
        }
    }

    let render_item_data = if is_open {
        active_item_data.clone()
    } else {
        last_active_item_data.read().clone()
    };

    let render_sub_id = if is_open {
        current_active_sub_id.clone()
    } else {
        last_active_sub_id.read().clone()
    };

    let has_children = active_item_data.as_ref().map_or(false, |item| !item.children.is_empty());

    let mut close_menu = move || {
        active_item.set(None);
        active_sub_item.set(None);
    };

    // Compute height for vertical subitems and horizontal leaf items
    let target_data = if is_open { active_item_data.as_ref() } else { render_item_data.as_ref() };
    let num_subs = target_data.map_or(0, |item| item.children.len());
    let active_sub_data = target_data.and_then(|item| {
        if let Some(ref sub_id) = render_sub_id {
            item.children.iter().find(|s| &s.id == sub_id)
        } else {
            None
        }
    });

    let num_leaves = active_sub_data.map_or(0, |s| s.sub_children.len());
    let has_sub_active = render_sub_id.is_some() && num_leaves > 0;

    // Generous height calculation ensuring leaves + descriptions are fully visible without clipping
    let vert_height = if num_subs > 0 { num_subs * 44 } else { 0 };
    let leaf_height = if num_leaves > 0 { 32 + num_leaves * 76 } else { 0 };
    let dropdown_height = vert_height.max(leaf_height);
    let dropdown_width = if has_sub_active { 480 } else { 220 };

    let class_str = format!(
        "uikit-horizontal-menu-root {}",
        class.unwrap_or_default()
    );

    rsx! {
        div {
            class: "{class_str}",
            onmouseleave: move |_| {
                is_hovering_bar.set(false);
                is_hovering_dropdown.set(false);
                close_menu();
            },
            // Background overlay with darken & blur when expanded
            div {
                key: "horizontal-menu-backdrop",
                class: format!(
                    "uikit-horizontal-menu-backdrop {}",
                    if is_open && has_children { "is-active" } else { "" }
                ),
                onclick: move |_| close_menu(),
            }

            // Top Menu Bar
            div {
                class: "uikit-horizontal-menu-bar",
                onmouseenter: move |_| {
                    is_hovering_bar.set(true);
                },
                onmouseleave: move |evt: MouseEvent| {
                    is_hovering_bar.set(false);
                    let coords = evt.element_coordinates();
                    if coords.y < 34.0 || coords.x <= 2.0 {
                        close_menu();
                    }
                },
                for item in items.clone() {
                    {
                        let item_id = item.id.clone();
                        let has_subs = !item.children.is_empty();
                        let is_item_active = current_active_item_id.as_deref() == Some(&item_id);

                        let on_item_click = {
                            let item_id = item_id.clone();
                            let on_select = on_select.clone();
                            move |_| {
                                if has_subs {
                                    if is_item_active {
                                        close_menu();
                                    } else {
                                        active_item.set(Some(item_id.clone()));
                                        active_sub_item.set(None);
                                    }
                                } else {
                                    close_menu();
                                    if let Some(ref cb) = on_select {
                                        cb.call(item_id.clone());
                                    }
                                }
                            }
                        };

                        let on_item_mouseenter = {
                            let item_id = item_id.clone();
                            let current_active_item_id = current_active_item_id.clone();
                            move |_| {
                                if has_subs {
                                    if current_active_item_id.as_deref() != Some(&item_id) {
                                        active_item.set(Some(item_id.clone()));
                                        active_sub_item.set(None);
                                    }
                                } else {
                                    close_menu();
                                }
                            }
                        };

                        rsx! {
                            button {
                                key: "{item.id}",
                                class: format!(
                                    "uikit-horizontal-menu-item-btn {}",
                                    if is_item_active { "active" } else { "" }
                                ),
                                onclick: on_item_click,
                                onmouseenter: on_item_mouseenter,
                                if let Some(ref icon) = item.icon {
                                    i { class: "uikit-horizontal-menu-icon {icon}" }
                                }
                                span { class: "uikit-horizontal-menu-label", "{item.label}" }
                                if has_subs {
                                    svg {
                                        class: format!(
                                            "uikit-horizontal-menu-chevron {}",
                                            if is_item_active { "active" } else { "" }
                                        ),
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "2",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        polyline { points: "6 9 12 15 18 9" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Shared expandable dropdown container: dynamically follows X position and morphs size
            div {
                class: format!(
                    "uikit-horizontal-menu-dropdown {} {}",
                    if is_open && has_children { "is-expanded" } else { "is-collapsed" },
                    if has_sub_active { "has-sub-active" } else { "" }
                ),
                style: format!("left: {}px;", dropdown_left_offset),
                onmouseenter: move |_| {
                    is_hovering_dropdown.set(true);
                },
                onmouseleave: move |_| {
                    is_hovering_dropdown.set(false);
                    if !*is_hovering_bar.read() {
                        close_menu();
                    }
                },
                div {
                    class: "uikit-horizontal-menu-dropdown-content",
                    style: if is_open && has_children {
                        format!("width: {}px; height: {}px;", dropdown_width, dropdown_height)
                    } else {
                        "width: 220px; height: 0px;".to_string()
                    },
                    if let Some(item_data) = render_item_data {
                        // Left Column: Options listed vertically
                        div { class: "uikit-horizontal-menu-vertical-column",
                            for sub in item_data.children.iter().cloned() {
                                {
                                    let sub_id = sub.id.clone();
                                    let is_sub_active = render_sub_id.as_deref() == Some(&sub_id);
                                    let has_sub_children = !sub.sub_children.is_empty();

                                    let on_sub_click = {
                                        let sub_id = sub_id.clone();
                                        let on_select = on_select.clone();
                                        move |_| {
                                            if has_sub_children {
                                                active_sub_item.set(Some(sub_id.clone()));
                                            } else {
                                                close_menu();
                                                if let Some(ref cb) = on_select {
                                                    cb.call(sub_id.clone());
                                                }
                                            }
                                        }
                                    };

                                    let on_sub_hover = {
                                        let sub_id = sub_id.clone();
                                        move |_| {
                                            if has_sub_children {
                                                active_sub_item.set(Some(sub_id.clone()));
                                            } else {
                                                active_sub_item.set(None);
                                            }
                                        }
                                    };

                                    rsx! {
                                        div {
                                            key: "{sub.id}",
                                            class: format!(
                                                "uikit-horizontal-menu-sub-item {}",
                                                if is_sub_active { "active" } else { "" }
                                            ),
                                            onmouseenter: on_sub_hover,
                                            onclick: on_sub_click,
                                            div { class: "uikit-horizontal-menu-sub-item-header",
                                                if let Some(ref icon) = sub.icon {
                                                    i { class: "uikit-horizontal-menu-icon {icon}" }
                                                }
                                                span { class: "uikit-horizontal-menu-sub-label", "{sub.label}" }
                                                if has_sub_children {
                                                    svg {
                                                        class: format!(
                                                            "uikit-horizontal-menu-sub-chevron {}",
                                                            if is_sub_active { "active" } else { "" }
                                                        ),
                                                        view_box: "0 0 24 24",
                                                        fill: "none",
                                                        stroke: "currentColor",
                                                        stroke_width: "2",
                                                        stroke_linecap: "round",
                                                        stroke_linejoin: "round",
                                                        polyline { points: "9 18 15 12 9 6" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Right panel: Horizontal expansion for selected vertical option
                        if let Some(ref active_sub_id) = render_sub_id {
                            if let Some(sub_data) = item_data.children.iter().find(|s| &s.id == active_sub_id) {
                                if !sub_data.sub_children.is_empty() {
                                    div {
                                        key: "{sub_data.id}",
                                        class: "uikit-horizontal-menu-horizontal-panel",
                                        for leaf in sub_data.sub_children.iter().cloned() {
                                            {
                                                let leaf_id = leaf.id.clone();
                                                let on_leaf_click = {
                                                    let leaf_id = leaf_id.clone();
                                                    let on_select = on_select.clone();
                                                    move |_| {
                                                        close_menu();
                                                        if let Some(ref cb) = on_select {
                                                            cb.call(leaf_id.clone());
                                                        }
                                                    }
                                                };

                                                rsx! {
                                                    div {
                                                        key: "{leaf.id}",
                                                        class: "uikit-horizontal-menu-leaf-item",
                                                        onclick: on_leaf_click,
                                                        if let Some(ref icon) = leaf.icon {
                                                            div { class: "uikit-horizontal-menu-leaf-icon",
                                                                i { class: "{icon}" }
                                                            }
                                                        }
                                                        div { class: "uikit-horizontal-menu-leaf-content",
                                                            div { class: "uikit-horizontal-menu-leaf-title", "{leaf.label}" }
                                                            if let Some(ref desc) = leaf.description {
                                                                div { class: "uikit-horizontal-menu-leaf-desc", "{desc}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
