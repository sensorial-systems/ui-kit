# ui-kit-app

Out-of-the-box native application runner for `ui-kit`.

`ui-kit-app` completely encapsulates the desktop lifecycle:
- Window creation with custom chrome, rounded borders, and transparent desktop backing.
- `wgpu` initialization (`Instance`, `Surface`, `Device`, `Queue`) with preferred transparent alpha compositor modes.
- Full DPI scaling from logical units to physical pixels.
- Event loop management (`winit`), mouse, scroll, keyboard shortcuts, and IME input routing.
- 60 FPS redraw loop and presentation.

## Quick Start

Add `ui-kit-app` to your `Cargo.toml`:

```toml
[dependencies]
ui-kit-app = { path = "../../crates/ui-kit-app" }
ui-kit-immediate = { path = "../../crates/ui-kit-immediate" }
```

### Closure-based Application

```rust,no_run
use ui_kit_app::FrameContext;
use ui_kit_immediate::{DrawCommand, Ui};

fn main() -> anyhow::Result<()> {
    ui_kit_app::run("My Application", |ctx: &FrameContext| {
        let mut ui = Ui::default();
        ui.begin(ctx.input.clone());
        ui.button("btn", ui_kit_core::Rect { x: 50.0, y: 50.0, width: 140.0, height: 40.0 }, "Click Me");
        ui.end()
    })
}
```

### Trait-based Application with Custom Config

```rust,no_run
use ui_kit_app::{App, AppConfig, FrameContext};
use ui_kit_immediate::DrawCommand;

struct MyApp {
    counter: u32,
}

impl App for MyApp {
    fn frame(&mut self, ctx: &FrameContext) -> Vec<DrawCommand> {
        // Compose your widgets here
        vec![]
    }

    fn on_scroll(&mut self, delta: f32) {
        // Handle scroll wheel
    }
}

fn main() -> anyhow::Result<()> {
    let config = AppConfig::new("My Application")
        .with_size(1200.0, 850.0)
        .with_dark(true);

    ui_kit_app::run_app(MyApp { counter: 0 }, config)
}
```

### Applications that live in the tray

The runner keeps calling `App::update` at the frame cadence whether or not
the window is on screen, so an application hidden to the system tray still
polls and still answers its tray menu. It draws only while visible.

- `AppConfig::with_visible(false)` starts with the window hidden; the window
  is created hidden either way, so nothing flashes before it is meant to show.
- `App::window_commands` is drained after every update: return
  `WindowCommand::Show`, `Hide`, `Focus` or `Exit` to have the runner do it.
- `App::on_close_requested` is asked when the close button or the system
  wants the window gone. Return `false` to keep running — having hidden the
  window, or asked about unsaved work — or `true` to exit.
- `App::text_measure_ready` hands the application what measures text the way
  the renderer draws it, once the renderer exists. Install it on the
  application's `Ui` with `set_shared_text_measure`, and controls sized to
  their labels fit them rather than an estimate.

```rust,no_run
use ui_kit_app::{App, FrameContext, WindowCommand};
use ui_kit_immediate::{DrawCommand, TextMeasure, Ui};

struct TrayApp {
    ui: Ui,
    show_requested: bool,
}

impl App for TrayApp {
    fn frame(&mut self, _ctx: &FrameContext) -> Vec<DrawCommand> {
        vec![]
    }

    fn update(&mut self) {
        // Poll the tray here; it runs even while the window is hidden.
    }

    fn text_measure_ready(&mut self, measure: TextMeasure) {
        self.ui.set_shared_text_measure(measure);
    }

    fn window_commands(&mut self) -> Vec<WindowCommand> {
        if std::mem::take(&mut self.show_requested) {
            vec![WindowCommand::Show]
        } else {
            vec![]
        }
    }
}
```
