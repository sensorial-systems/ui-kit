use crate::{WindowAction, WindowChrome};
use dioxus::prelude::*;
use ui_kit_core::Theme;
use ui_kit_dioxus::SharedView;
/// The shared self-drawn window frame, as DOM.
///
/// The bar, its button IDs and the actions they emit come from the same
/// [`WindowChrome`] the immediate host paints, so a window described once looks
/// and behaves the same in either. The region a press falls in is the host's:
/// ask [`WindowChrome::hit`] with the pointer position.
#[component]
pub fn WindowChromeView(
    chrome: WindowChrome,
    on_action: EventHandler<WindowAction>,
    #[props(default)] theme: Theme,
    #[props(default)] children: Element,
) -> Element {
    let radius = chrome.effective_radius();
    let bar = chrome.view(|action| action);
    rsx! {div {style:format!("display:flex;flex-direction:column;height:100%;overflow:hidden;border-radius:{radius}px;background:{};",color(theme.panel.fill)),
        div {style:"-webkit-app-region:drag;flex-shrink:0;",
            SharedView {view: bar, theme: theme.clone(), on_action: move |action| on_action.call(action)}
        }
        div {style:"flex:1;min-height:0;overflow:auto;", {children}}
    }}
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
