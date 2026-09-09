use dioxus::prelude::*;
use shared_gallery::{gallery_theme, settings_view, Settings};
use ui_kit_dioxus::SharedView;
fn main() {
    dioxus::launch(App);
}
#[component]
fn App() -> Element {
    let mut state = use_signal(Settings::default);
    rsx! {
        style {"body {{margin:0;padding:40px;background:radial-gradient(ellipse at top left,#4556a3,#0b1425 70%);color:white;font-family:system-ui;}} button,input {{font:inherit;}}"}
        SharedView {view:settings_view(&state.read()),theme:gallery_theme(),on_action:move |action|state.write().update(action)}
    }
}
