//! Native DOM controls for the shared semantic view.
use dioxus::prelude::*;
use ui_kit_core::{Event as UiEvent, Kind, Material, Theme, Value, View};

/// Render the same formatted runs used by the immediate renderer as native DOM text.
#[component]
pub fn RichTextView(
    document: ui_kit_core::RichText,
    #[props(default = 16.0)] font_size: f32,
) -> Element {
    let align = match document.align {
        ui_kit_core::TextAlign::Left => "left",
        ui_kit_core::TextAlign::Center => "center",
        ui_kit_core::TextAlign::Right => "right",
    };
    rsx! {div{style:format!("white-space:pre-wrap;text-align:{align};font-size:{font_size}px;line-height:1.25;"),for(index,run)in document.runs.into_iter().enumerate(){
        span{key:"{index}",style:format!("font-weight:{};font-style:{};text-decoration:{} {};font-size:{}em;",if run.format.bold||run.format.heading>0{700}else{400},if run.format.italic{"italic"}else{"normal"},if run.format.underline{"underline"}else{""},if run.format.strike{"line-through"}else{""},match run.format.heading{1=>2.0,2=>1.5,_=>1.0}),"{run.text}"}
    }}}
}

#[component]
pub fn SharedView<A: Clone + PartialEq + 'static>(
    view: View<A>,
    on_action: EventHandler<A>,
    #[props(default)] theme: Theme,
) -> Element {
    if let Err(error) = view.validate() {
        return rsx! {div {role:"alert","{error}"}};
    }
    render_node(view, theme, on_action, false, false)
}

/// Common dispatch is deliberately accessible to hosts and conformance tests.
pub fn dispatch_event<A>(view: &View<A>, event: &UiEvent) -> Option<A> {
    view.dispatch(event)
}
fn color(c: [f32; 4]) -> String {
    format!(
        "rgba({},{},{},{})",
        c[0] * 255.0,
        c[1] * 255.0,
        c[2] * 255.0,
        c[3]
    )
}
fn render_node<A: Clone + PartialEq + 'static>(
    view: View<A>,
    theme: Theme,
    on_action: EventHandler<A>,
    inherited_disabled: bool,
    row_child: bool,
) -> Element {
    let disabled = inherited_disabled || view.is_disabled();
    let id = view.id.clone().unwrap_or_default();
    let style = view.resolved_style(&theme);
    let mut css=format!("box-sizing:border-box;min-width:0;flex-shrink:0;overflow:hidden;height:{}px;font:{}px system-ui;color:{};",view.intrinsic_height(),style.font_size,color(style.foreground));
    if let Some(width) = view.layout.width {
        css += &format!("width:{width}px;")
    } else if row_child {
        css += "flex:1;"
    } else {
        css += "width:100%;"
    }
    let container = matches!(view.kind, Kind::Column | Kind::Row | Kind::Panel);
    if container {
        css += &format!(
            "display:flex;flex-direction:{};gap:{}px;padding:{}px;",
            if matches!(view.kind, Kind::Row) {
                "row"
            } else {
                "column"
            },
            view.layout.gap,
            view.layout.padding
        );
    }
    if view.is_control() || matches!(view.kind, Kind::Panel) {
        css += &format!(
            "background:{};border:1px solid {};border-radius:{}px;",
            color(style.fill),
            color(style.stroke),
            style.radius
        );
        match &style.material {
            Material::Glass { blur } => {
                css += &format!(
                    "backdrop-filter:blur({blur}px);-webkit-backdrop-filter:blur({blur}px);"
                )
            }
            Material::Custom { name, .. } => {
                return rsx! {div {role:"alert","Unsupported DOM material: {name}"}}
            }
            _ => {}
        }
    }
    css += &format!(
        "accent-color:{};font-weight:{};text-align:{};border-width:{}px;",
        color(style.accent),
        style.font_weight,
        match style.text_align {
            ui_kit_core::TextAlign::Left => "left",
            ui_kit_core::TextAlign::Center => "center",
            ui_kit_core::TextAlign::Right => "right",
        },
        style.stroke_width
    );
    if let Some(end) = style.gradient_end {
        css += &format!(
            "background:linear-gradient(135deg,{},{});",
            color(style.fill),
            color(end)
        );
    }
    if disabled {
        css += "opacity:0.5;"
    }
    let padding = style.text_padding;
    let event_view = view.clone();
    let event_id = id.clone();
    let emit = move |value: Value| {
        if !disabled {
            if let Some(action) = dispatch_event(
                &event_view,
                &UiEvent {
                    id: event_id.clone(),
                    value,
                },
            ) {
                on_action.call(action)
            }
        }
    };
    match view.kind {
        Kind::Column | Kind::Row | Kind::Panel => {
            let row = matches!(view.kind, Kind::Row);
            rsx! {div {style:css,for (i,child) in view.children.into_iter().enumerate() {
                {let key=child.id.clone().unwrap_or_else(||format!("container-{i}"));rsx!{Fragment {key: "{key}", {render_node(child,theme.clone(),on_action,disabled,row)}}}}
            }}}
        }
        Kind::Text(content) => rsx! {div {style:css,"{content}"}},
        Kind::Button { label, loading } => {
            rsx! {button {id,style:css,disabled,r#type:"button","aria-busy":loading.to_string(),onclick:move |_|emit(Value::Activate),if loading {"Working… "} "{label}"}}
        }
        Kind::Checkbox { label, checked } => {
            rsx! {label {style:format!("{css}display:flex;align-items:center;gap:12px;padding:0 {padding}px;"),
                input {id,r#type:"checkbox",checked,disabled,onchange:move |e|emit(Value::Toggle(e.checked()))} span {"{label}"}
            }}
        }
        Kind::Slider {
            label,
            value,
            range,
        } => {
            let value = range.normalize(value).unwrap_or(range.min);
            rsx! {label {style:format!("{css}display:flex;align-items:center;gap:12px;padding:0 {padding}px;"),span {"{label}"}
                input {id,r#type:"range",style:"flex:1;min-width:0;",min:range.min,max:range.max,step:range.step,value,disabled,
                    oninput:move |e| {if let Ok(value)=e.value().parse() {emit(Value::Number(value))}}
                }
            }}
        }
        Kind::TextInput {
            label,
            value,
            placeholder,
        } => {
            rsx! {input {id,style:format!("{css}padding:0 {padding}px;"),r#type:"text","aria-label":label,value,placeholder,disabled,oninput:move |e|emit(Value::Text(e.value()))}}
        }
    }
}
