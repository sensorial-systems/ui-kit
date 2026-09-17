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

The immediate text field keeps a caret and a selection: the pointer places and
drags them, a double click selects all, arrows/Home/End move them (Shift extends),
and typing, Backspace, Delete, paste, copy and cut act at them. Text an input
method is composing (`Input.preedit`) is shown at the caret until it commits.

### Input and output

`Input` also carries the wheel (`scroll`), the secondary button, the keys pressed
(`keys`, with `modifiers`), `time` for double clicks, delays and animation, the
`viewport` popups keep inside, clipboard requests (`copy`, `cut`, `paste`) and
`access_actions` from screen readers. After `Ui::end`, `Ui::output()` says what
the frame asks of the host: the `cursor`, text for the `clipboard`, whether it
`wants_keyboard`, the `ime_area`, and `repaint_after` while something animates.

With winit, `ui_kit_window::InputCollector` (feature `winit` + `immediate`) does
both directions: feed it every `WindowEvent`, `take` the input each frame, and
`apply` the output to the window. Feature `clipboard` connects the system
clipboard; feature `accesskit` adds `AccessibilityBridge`, which publishes
`Ui::accessibility()` to screen readers and turns their requests into input.

### Ids, styles and state

`ui.scope("left", |ui| ..)` prefixes every id used inside, so a component can be
drawn twice. A duplicate id stops debug builds and is listed by
`Ui::duplicate_ids` in release. `ui.with_style`, `ui.scoped` and `ui.with_clip`
restore what they change. `Ui::theme` holds the named styles widgets use --
`Theme::button(Variant)`, rows, popups, tooltips, fields -- built from a
`Palette` with `Theme::from_palette`. `ui.animate(id, target, seconds)` eases a
value; `ui.remember`/`ui.remembered` keep a size across frames, and
`ui.painted_since(ui.mark())` measures what was just drawn.

### Widgets

Beyond buttons, checkboxes, sliders, fields, comboboxes and number pickers:
`Toggle`, `RadioGroup`, `SegmentedControl`, `Tabs`, `TreeView`, `Table` (sortable,
only visible rows drawn), `Menu::button`, `Menu::context` (right click),
`MenuBar`, `ui.popup` (nested, flips and slides to stay on screen), `ui.tooltip`,
`ui.modal`, `ui.toast`/`ui.show_toasts`, `ui.image` (contain, cover or fill; the
renderer draws textures registered with `register_image`), `ScrollArea` (both
axes), `VirtualList`, drag and drop (`drag_source`, `drop_target`) and keyboard
shortcuts (`ui.shortcut`). Built-in widgets describe themselves for
accessibility.

### Testing

`Harness` drives a view by control id: `click("save", view)`,
`type_text`, `key`, `scroll`, `drag`, `right_click`, `double_click`, and reads
back `rect(id)`, `find_text` and the accessibility nodes.

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
handled or `false` to delegate. Override `CommandRenderer::wants` to name the
commands a renderer handles: the rest are then drawn in batches -- one pass for a
run's shapes and one for its text -- instead of a pass apiece.

`Style::overflow` chooses whether text wraps, clips or ends in an ellipsis. Install
`renderer.text_measure()` on a `Ui` with `set_shared_text_measure` so
`Ui::text_width`, `fit_text` and `elide` use the fonts the renderer draws with;
`load_font` adds a font to both. Honor the command clip and record into the provided
encoder. Unknown kinds/materials are errors; the Dioxus host displays an alert for
unsupported custom materials. Native GPU shaders are not translated into CSS.

## Current coverage and layout

The shared model includes Text, Button, Checkbox, Slider, TextInput, Toggle,
Select, Progress, Separator, Panel, Row and Column, rendered by both hosts. Legacy tables, graphs, calendars and other existing components remain
on the original Dioxus API for incremental migration.

The supported layout subset is fixed or fill widths, fixed or intrinsic heights,
rows/columns, padding and gaps. Rows divide remaining width equally between fill
children. Overflow clips. Wrapping, scrolling, grids, responsive breakpoints,
reverse Tab, and overlapping interactive widgets are not implemented yet. The DOM
adapter uses the corresponding CSS rules; glyph metrics can differ by platform.

### Immediate layout

Immediate widgets take a `Rect`, but nothing should compute one by adding offsets.
`ui_kit_immediate::layout` says how big things are and works out where they go;
the shared `layout` above is solved by the same `Flex`.

```rust
use ui_kit_immediate::{Flex, Insets, Size, Strip, Stack, Grid};

// A bar: fixed controls at both ends, the seek bar in whatever is left.
let mut bar = Strip::row(bounds.inset_by(Insets::symmetric(8.0, 0.0)), 4.0);
let play = bar.leading(32.0);
let volume = bar.trailing(84.0);
let open = bar.trailing_if_room(68.0, 120.0); // gives way on a narrow bar
let seek = bar.rest();

// A run solved in one pass: fixed items, gaps, then fills by weight.
let [dec, value, inc] = Flex::row(0.0).split(field, [Size::Fixed(28.0), Size::FILL, Size::Fixed(28.0)]);

// Rows down a scrolling card, and tiles wrapped into as many columns as fit.
let mut rows = Stack::new(content, 6.0);
let heading = rows.row(20.0);
let tiles = Grid::fit(area, 120.0, 8.0);
let third = tiles.cell(2);
```

`Rect` adds `align`, `centered`, `center_vertically`, `inset_by`, `split_top`,
`split_bottom` and `overlaps`. `Ui::text_width` / `Ui::fit_text` size a control
to its label; install a shaping measure with `Ui::set_text_measure`, otherwise
the width is estimated from the character count.

### Components

`ui_kit_immediate::components` has the immediate counterparts of the Dioxus
components, drawn in the colours of `ui.theme.palette` (`components::theme::theme(dark)`
builds the kit's light and dark looks). Stateless ones are builders with a
`show` method; stateful ones take a state value the application owns:

```rust
use ui_kit_immediate::components::{Button, ButtonVariant, Dropdown, Menu, MenuItem, MenuState};
use ui_kit_immediate::icon::Icon;

ui.theme = components::theme::theme(dark);
if Button::new("Save").icon(Icon::Check).show(&mut ui, "save", rect).clicked { /* ... */ }
Dropdown::new(&["Rust", "Go"]).show(&mut ui, "language", field, &mut language);
if let Some(id) = Menu::horizontal(&items).show(&mut ui, "nav", bar, &mut menu_state) { /* ... */ }
```

Inputs: `Button`, `TextInput`, `Select`, `Dropdown`, `Checkbox`, `Switch`,
`Slider`, `OtpInput`, `DateTimePicker`, `ColorPicker`, `ColorField`,
`EditableText`. Feedback: `Label` (with `ellipsis()` for one-line text),
`Badge`, `Notification`, `ProgressBar`, `Spinner`, `StatusDot` (a dot with a
breathing ring while something is alive). Containers: `Card`, `Modal`,
`Browser`, `Table`, `PipelineBoard`, `Timeline`, `Bill`, `MetricCard`,
`FormFlow`, `JobList` (work something is running or has queued, grouped, with
pause, resume and stop on each job and each group; the application maps its
own work onto `Job` and `JobGroup` and acts on the `JobAction` it returns), plus
`card_header`, `stat_row` and `empty_state` for the parts of a card that are
the same everywhere. Navigation: `Menu` (horizontal or
vertical; children cascade into submenus) and `Sidebar` (a column of sections
with icons, badges and a highlight that slides to the chosen one; icons only
when narrow). Charts and graphs: `Sparkline`, `AreaChart` (a series over a
filled area, with guide lines and a pinnable range), `Gauge` (a ring filled
to a fraction, coloured by how full), `BarChart` (bars on a shared scale with
a caption each), `SegmentedBar` (parts of a whole laid end to end, the rest
left as track, with tooltips and a legend), `SlotLane` (capacity as slots in
use and a count waiting), `PieChart`, `StackedBarChart`, `FlowGraph`,
`NetworkGraph`, `HierarchyGraph`, `GraphNode`, `GraphCamera`. Menus, dropdowns, pickers and dialogs draw over the
page on their own layers and take the pointer from what is under them, so they
can be shown from anywhere in a frame; set `Input::viewport` so they stay on screen.

### Vectors, SVG and icons

`ui_kit_immediate::vector` paints antialiased meshes (`fill_triangles`, `line`,
`polyline`, `circle`). `svg::Svg::parse` reads an SVG once and tessellates its
fills and strokes; `paint` and `paint_tinted` then place it at any size.
`icon::Icon` is a set of line icons (chevrons, arrows, close, plus, check,
calendar, loader, play, pause, stop, trash, refresh, folder, file, save,
settings, cpu, download, upload, flame, snowflake, zap, search, copy, external
link, layers, activity, list, terminal, gauge, clock, undo, info, alert,
database, hard drive, power, ...) drawn from SVG, so no control depends on a
font having a glyph. Meshes are drawn by `ui_kit_wgpu::VectorRenderer`:
`WgpuRenderer::with_custom(&gpu, format, VectorRenderer::new(&gpu, format))`.
`EditableText` places its caret with `ui_kit_wgpu::ShapedRichText`.

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
the window host from `ui_kit_immediate::components`; `VectorRenderer` composes
vector drawing with the standard renderer through `CommandRenderer`. Snapshot a
state with `cargo run -p immediate-gallery -- --snapshot out.png`, adding
`--click LABEL`, `--drag FROM TO`, `--section NAME` or `--dark`.

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
