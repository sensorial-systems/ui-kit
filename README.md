# UI Kit

Shared Rust UI descriptions for Dioxus and immediate GPU rendering, alongside the
existing Dioxus component library.

| Crate | Responsibility |
| --- | --- |
| `ui-kit-core` | Typed component tree, IDs, theme, layout and action dispatch |
| `ui-kit-dioxus` | Native HTML controls and Dioxus event adapter |
| `ui-kit-immediate` | Pointer/focus state, semantic actions, clipped drawing commands |
| `ui-kit-wgpu` | Shared device/queue, texture targets, materials and rendering extensions |
| `ui-kit-window` | Custom window chrome, optional immediate/Dioxus rendering, and reusable winit integration |
| `ui-kit` | Existing Dioxus components, plus `shared` and `SharedView` exports |

The new core and hosts have no dependency on Prism. The Prism implementation is
still present in its repository; adoption there can happen separately. Existing
`ui-kit::Button`, `Slider`, `TextInput`, etc. signatures remain unchanged.

## Declare once

The application owns its state and actions. The view contains descriptions and
action mappings, not Dioxus signals or GPU handles.

```rust
use ui_kit_core::*; // also available as ui_kit::shared::*

#[derive(Clone, PartialEq)]
enum Action { Rename(String), Volume(f64), Enabled(bool), Save }

fn settings(name: &str, volume: f64, enabled: bool) -> View<Action> {
    panel([
        text("Settings"),
        text_input("name", "Workspace name", name).on_input(Action::Rename),
        slider("volume", "Volume", volume)
            .range(0.0, 1.0, 0.05)
            .on_change(Action::Volume),
        row([
            checkbox("enabled", "Enable saving", enabled).on_toggle(Action::Enabled),
            button("save", "Save").disabled(!enabled).on_activate(Action::Save),
        ]),
    ]).padding(24.0).gap(16.0)
}
```

IDs must be unique across the tree and stable across updates. Compose views with
functions, `column`, `row`, `panel`, and dynamically generated child iterators.
Handlers accept action constructors or `Send + Sync + 'static` closures. This
in-process view is not a serialized wire protocol; action handlers are closures.

## Dioxus host

```rust
use dioxus::prelude::*;
use ui_kit::SharedView;

#[component]
fn SettingsScreen() -> Element {
    let mut state = use_signal(Settings::default);
    rsx! {
        SharedView {
            view: settings(&state.read().name, state.read().volume, state.read().enabled),
            on_action: move |action| state.write().update(action),
        }
    }
}
```

`Settings` and its reducer belong to your application; the runnable gallery has
a complete implementation. `SharedView` uses native buttons and inputs, preserving
browser keyboard behavior, text editing, selection, clipboard, IME and semantics.
It maps events through `View::dispatch`, including disabled/loading guards and
slider range/step normalization. DOM text input has an accessible label.

## Immediate host and wgpu

```rust
use ui_kit_core::{Rect, Theme};
use ui_kit_immediate::{ImmediateHost, composition::UiRenderer};
use ui_kit_wgpu::{GpuContext, WgpuRenderer, WgpuFrame, TextureTarget};

// Initialize once using your application's Arc<Device> and Arc<Queue>.
let gpu = GpuContext::new(device.clone(), queue.clone());
let mut renderer = WgpuRenderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm)?;
let target = renderer.create_texture(800, 600)?;
let mut host = ImmediateHost::default();

// Each frame, from the same application view:
let view = settings(&state.name, state.volume, state.enabled);
let frame = host.frame(
    &view,
    &Theme::default(),
    input,
    Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 },
)?;
for action in frame.actions { state.update(action); }

let mut encoder = device.create_command_encoder(&Default::default());
renderer.render(&mut WgpuFrame {
    encoder: &mut encoder,
    target: TextureTarget::from_texture2d(&target)?,
    clear: Some(wgpu::Color::TRANSPARENT),
}, &frame.commands)?;
// Add other passes, including sampling target.view on a world-space panel.
queue.submit([encoder.finish()]);
```

`ImmediateHost` persists focus and capture. `Input` supplies an optional panel-pixel
pointer, button-down state, committed text, backspace, forward Tab, activation, and
signed keyboard `step` for sliders. Submit press/release frames in order. The host
emits actions; it never owns or mutates application values. Drawing can be repeated
for other targets without advancing interaction. Changed application values are
reflected by rebuilding the next view.

`spatial::WorldPanel` maps a controller ray to panel pixels. Use the resulting
point as `Input.pointer`; the rendering layer does not own a headset session.
The immediate text field currently supports committed insertion at the end and
backspace. Selection, clipboard, IME integration and accessibility bridges require
host integration; it is not yet a full native text editor.

## Appearance and custom rendering

```rust
let mut theme = Theme::default();
theme.panel.material = Material::Glass { blur: 12.0 };
theme.control.accent = [0.4, 0.9, 0.8, 1.0];
```

Both adapters use typed colors, corner radius, foreground, font size and materials.
Nodes can override a style with `.styled(style)`. Browser glass uses CSS backdrop
blur; wgpu uses a nine-tap snapshot blur. These approximate the same appearance,
not identical pixels. wgpu renders system fonts by default; call `load_font(bytes)`
on platforms without system fonts or to supply application fonts.

`WgpuRenderer` accepts a caller-owned encoder and a full base-mip 2D texture view.
Supported formats are RGBA8/BGRA8 unorm, linear/sRGB, and RGBA16 float, with one
sample and one layer. UI records commands without submitting, polling, acquiring
a surface, or creating a device. `clear: None` overlays previous scene rendering.
Glass requires `COPY_SRC`. Transparent textures contain premultiplied RGB; use
premultiplied blending when compositing them. All resources must share the device.

Implement `UiRenderer<Target>` to replace the command backend, or compose partial
wgpu overrides with `WgpuRenderer::with_custom(&gpu, format, (effect_a, effect_b))`.
Each `CommandRenderer` receives context, frame and command; return `true` when
handled or `false` to delegate. Honor the command clip and record into the provided
encoder. Unknown kinds/materials are errors; the Dioxus host displays an alert for
unsupported custom materials. Native GPU shaders are not translated into CSS.

## Current coverage and layout

The shared model includes Text, Button, Checkbox, Slider, TextInput, Panel, Row
and Column. Legacy tables, graphs, calendars and other existing components remain
on the original Dioxus API for incremental migration.

The supported layout subset is fixed or fill widths, fixed or intrinsic heights,
rows/columns, padding and gaps. Rows divide remaining width equally between fill
children. Overflow clips. Wrapping, scrolling, grids, responsive breakpoints,
reverse Tab, and overlapping interactive widgets are not implemented yet. The DOM
adapter uses the corresponding CSS rules; glyph metrics can differ by platform.

## Run and verify

Standalone immediate-mode gallery (native winit window, wgpu rendering):

It uses `ui-kit-window` for the custom title bar and native window interactions.
See the [winit composition guide](crates/ui-kit-window/README.md) to reuse the
same setup in another application.

```powershell
cd D:\dev\sensorial\systems\ui-kit
cargo run -p immediate-gallery
```

The window has light/dark themes, glass strength, scrolling, menus, browser tabs,
forms, loading states, dialogs, metrics, tables, planning views, invoices and
graphs. Mouse wheel and Page Up/Down scroll; Ctrl+Home/End jump to the ends;
Escape closes overlays. Shift-click selects pipeline cards for group dragging.
The application supplies the wgpu device/queue and renders into a reusable
texture before presenting. `Gallery::frame` builds commands separately from
the window host; `GeometryRenderer` composes vector drawing with the standard
renderer through `CommandRenderer`.

The existing Dioxus gallery is still its own DOM/CSS renderer:

```powershell
cd examples/gallery
dx serve --web --port 8096
```

The native gallery follows that gallery's sections and theme, but is not yet
pixel-identical or feature-equivalent for every advanced component.
Date/time editing now supports month navigation, leap years, validated time,
and Apply/Cancel. Color editing supports Canvas, Presets, Hex, RGB, HSL, and
alpha, both inline and in a popover. Ctrl/Cmd+A replaces a whole text field.
Hierarchy editing adds, renames, and deletes subtrees with stable IDs. The
survey follows the reference role-based branch and exposes its submitted
answers through `Gallery::survey_result`.

The native page is currently built from immediate
primitives; it does not imply that all legacy components have been added to the
shared semantic tree. The smaller shared gallery below demonstrates the
single-definition API supported by both adapters.

Capture the native renderer without opening a window:

```powershell
cargo run -p immediate-gallery -- --snapshot target/immediate-gallery.png
cargo run -p immediate-gallery -- --snapshot target/immediate-dark.png --dark --section "7. Planning Components"
cargo run -p immediate-gallery -- --snapshot target/calendar.png --click "2026-07-16 18:00"
cargo run -p immediate-gallery -- --snapshot target/color.png --click "#3b82f6" --click HSL
```

Snapshots use the same frame builder and renderer as the live window.
Repeat `--click` to drive a sequence of controls (exact labels take precedence
over prefixes). The picker, hierarchy, and survey implementations are separated
into `pickers.rs`, `hierarchy.rs`, and `survey.rs` in the native example.

Live shared gallery:

```sh
cd examples/shared-gallery
dx serve --web --features web --port 8098
```

From the repository root, generate a GPU PNG and static side-by-side HTML from the
same `settings_view`:

```sh
cargo run -p shared-gallery --example texture --features gpu
```

Outputs: `target/shared-gallery.png` and `target/shared-gallery.html`. The HTML
comparison is static; use the live gallery for application interactions. GPU
readback and waiting occur only in the example/tests.

```sh
cargo test -p ui-kit-core -p ui-kit-immediate -p ui-kit-dioxus -p ui-kit-wgpu -p shared-gallery --features shared-gallery/gpu
cargo test -p ui-kit
```

Tests cover action parity, range rules, disabled/loading states, native DOM
semantics, layout, input capture, Unicode editing, VR ray mapping, shared GPU
handles, texture pixels, clipping and glass transparency. GPU tests need an adapter.

The original comprehensive Dioxus gallery remains in `examples/gallery`.

Rich text uses `ui_kit_core::RichText` (formatted runs) and `RichTextEditor`
(selection, grapheme-aware editing, formatting, undo/redo). The native gallery
adds a toolbar, hit testing, keyboard navigation, and Save/Cancel. Lists currently
insert text markers; the editor does not yet implement clipboard integration or
scrolling inside long documents. Flow and network graphs support selection,
zoom buttons, Fit, and background dragging. Both network renderers use the same
deterministic `ui_kit_core::graph::network_layout` function.

A formatted document can be displayed by either renderer:

```rust,ignore
let document = ui_kit_core::RichText::plain("Hello");
// Native GPU command (application controls placement and frame submission):
ui.rich_text(bounds, ui_kit_immediate::RichTextPaint {
    document: document.clone(), cursor: None, anchor: 0,
});
// Dioxus DOM/CSS component:
rsx! { ui_kit::RichTextView { document } }
```

`RichTextView` displays shared formatted runs; the existing Dioxus editable-text
component remains its own DOM editor. Sharing document/layout logic does not
replace either renderer.
Native interaction/rendering details: drag the right-hand scrollbar thumb (or
click its track), and drag graph backgrounds to pan. Pointer movement requests
an immediate redraw; animations use a 60 Hz timer. Graph and chart meshes have
one-physical-pixel antialiasing at their outer edges. Control labels are centered
using glyph ink bounds, and RGB channel edits preserve untouched channels.

Run the opt-in rendering diagnostic with:

```powershell
cargo test -p immediate-gallery --test render profile_flow_frames -- --ignored --nocapture
```

It reports CPU and GPU time for warmed flow frames, including a synchronous GPU
wait; it is a diagnostic, not a timing assertion or a live-window latency measure.
