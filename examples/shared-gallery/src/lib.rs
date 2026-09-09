//! The same state, view and actions are used by the browser and GPU gallery.
use ui_kit_core::*;
#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    pub name: String,
    pub volume: f64,
    pub enabled: bool,
    pub saves: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            name: "Spatial workspace".into(),
            volume: 0.55,
            enabled: true,
            saves: 0,
        }
    }
}
#[derive(Clone, PartialEq, Debug)]
pub enum Action {
    Rename(String),
    Volume(f64),
    Enabled(bool),
    Save,
}
impl Settings {
    pub fn update(&mut self, action: Action) {
        match action {
            Action::Rename(v) => self.name = v,
            Action::Volume(v) => self.volume = v,
            Action::Enabled(v) => self.enabled = v,
            Action::Save => self.saves += 1,
        }
    }
}
pub fn settings_view(state: &Settings) -> View<Action> {
    panel([
        text("One description. Two renderers."),
        text("Shared IDs, layout, theme and semantic actions"),
        text_input("name", "Workspace name", &state.name).on_input(Action::Rename),
        slider("volume", "Volume", state.volume)
            .range(0.0, 1.0, 0.05)
            .on_change(Action::Volume),
        row([
            checkbox("enabled", "Enable saving", state.enabled).on_toggle(Action::Enabled),
            button("save", format!("Save · {}", state.saves))
                .disabled(!state.enabled)
                .on_activate(Action::Save),
        ]),
        button("disabled", "Disabled component")
            .disabled(true)
            .on_activate(Action::Save),
        button("loading", "Loading component")
            .loading(true)
            .on_activate(Action::Save),
    ])
    .padding(24.0)
    .gap(16.0)
    .width(720.0)
}
pub fn gallery_theme() -> Theme {
    let mut theme = Theme::default();
    theme.panel.material = Material::Glass { blur: 12.0 };
    theme.panel.fill = [0.06, 0.1, 0.2, 0.65];
    theme
}
