//! Interactive GPU implementation of the existing Dioxus gallery.
//! The Dioxus example remains an independent DOM/CSS renderer.
pub mod editor;
pub mod geometry;
pub mod graphs;
pub mod hierarchy;
pub mod pickers;
pub mod survey;
#[cfg(test)]
mod tests;
use std::collections::HashMap;
use ui_kit_core::{Color, Material, Point, Rect, Style, TextAlign};
use ui_kit_immediate::{DrawCommand, Input, Response, Ui};

pub const SECTIONS: &[&str] = &[
    "Navigation",
    "Browser Containers",
    "1. Buttons",
    "2. Form Controls",
    "3. Feedback & Badges",
    "4. Modals & Dynamic Form Flow Engine",
    "5. Metric & Data Visualization",
    "6. Table Component",
    "7. Planning Components",
    "8. Bill Component",
    "9. Graph Components",
];
fn rgb(v: u32) -> Color {
    [
        (v >> 16) as f32 / 255.0,
        ((v >> 8) & 255) as f32 / 255.0,
        (v & 255) as f32 / 255.0,
        1.0,
    ]
}
const BLUE: Color = [0.231, 0.510, 0.965, 1.0];
const GREEN: Color = [0.063, 0.725, 0.506, 1.0];
const YELLOW: Color = [0.961, 0.620, 0.043, 1.0];
const RED: Color = [0.937, 0.267, 0.267, 1.0];
#[derive(Clone, Copy)]
struct Palette {
    bg: Color,
    fg: Color,
    card: Color,
    border: Color,
    muted: Color,
    secondary: Color,
    dark: bool,
}
impl Palette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                bg: rgb(0x121214),
                fg: rgb(0xf4f4f5),
                card: rgb(0x1a1a1e),
                border: rgb(0x2e2e33),
                muted: rgb(0x8b8b93),
                secondary: rgb(0x27272a),
                dark,
            }
        } else {
            Self {
                bg: rgb(0xffffff),
                fg: rgb(0x09090b),
                card: rgb(0xffffff),
                border: rgb(0xe4e4e7),
                muted: rgb(0x71717a),
                secondary: rgb(0xf4f4f5),
                dark,
            }
        }
    }
}
#[derive(Clone)]
enum Overlay {
    Menu {
        label: String,
        choices: Vec<String>,
        anchor: Rect,
    },
    Modal,
    Survey(usize),
    Calendar,
    Color,
}
pub struct Gallery {
    ui: Ui,
    overlay_ui: Ui,
    pub shift: bool,
    pipeline_selected: Vec<usize>,
    pub dark: bool,
    pub scroll: f32,
    scrollbar_grab: Option<f32>,
    pub content_height: f32,
    viewport: f32,
    pub glass: f32,
    pub username: String,
    pub accepted: bool,
    pub notifications: bool,
    pub language: String,
    pub otp: String,
    pub slider: f32,
    pub date: String,
    pub color: String,
    inline_color: String,
    pub loading: bool,
    timeout: Option<f32>,
    pub selectable: bool,
    aspect: usize,
    pub interactions: u32,
    pub tabs: Vec<String>,
    pub tab: usize,
    pub address: String,
    overlay: Option<Overlay>,
    table: [bool; 5],
    sort: usize,
    descending: bool,
    pipeline: Vec<(String, usize)>,
    drag: Option<usize>,
    timeline_zoom: f32,
    timeline_expanded: bool,
    timeline_milestone: bool,
    task_left: f32,
    task_width: f32,
    task_drag: Option<(bool, f32, f32)>,
    selected: String,
    tree_edit: bool,
    tree: hierarchy::Hierarchy,
    calendar: pickers::Calendar,
    color_editor: pickers::ColorEditor,
    inline_editor: pickers::ColorEditor,
    picker_anchor: Rect,
    pub rich: editor::Editor,
    network_camera: graphs::GraphCamera,
    flow_camera: graphs::GraphCamera,
    network_selected: String,
    flow_selected: String,
    pub survey_result: Option<survey::SurveyResult>,
    survey_role: Option<usize>,
    survey_stack: [bool; 3],
    survey_feedback: String,
    pub section_positions: HashMap<String, f32>,
}
impl Default for Gallery {
    fn default() -> Self {
        Self {
            ui: Ui::default(),
            overlay_ui: Ui::default(),
            shift: false,
            pipeline_selected: Vec::new(),
            dark: false,
            scroll: 0.0,
            scrollbar_grab: None,
            content_height: 10000.0,
            viewport: 850.0,
            glass: 1.0,
            username: String::new(),
            accepted: false,
            notifications: true,
            language: "Rust".into(),
            otp: String::new(),
            slider: 50.0,
            date: "2026-07-16 18:00".into(),
            color: "#3b82f6".into(),
            inline_color: "#10b981".into(),
            loading: false,
            timeout: None,
            selectable: true,
            aspect: 0,
            interactions: 0,
            tabs: vec![
                "overview.local".into(),
                "components.local".into(),
                "api.local".into(),
            ],
            tab: 0,
            address: "overview.local".into(),
            overlay: None,
            table: [true, true, false, false, false],
            sort: 1,
            descending: false,
            pipeline: vec![
                ("Research customer needs".into(), 0),
                ("Prepare implementation brief".into(), 1),
                ("Define semantic tokens".into(), 2),
                ("Review component docs".into(), 3),
                ("Publish alpha release".into(), 4),
            ],
            drag: None,
            timeline_zoom: 1.0,
            timeline_expanded: true,
            timeline_milestone: true,
            task_left: 70.0,
            task_width: 15.0,
            task_drag: None,
            selected: "build".into(),
            tree_edit: false,
            tree: hierarchy::Hierarchy::default(),
            calendar: pickers::Calendar::default(),
            color_editor: pickers::ColorEditor::new("#3b82f6"),
            inline_editor: pickers::ColorEditor::new("#10b981"),
            picker_anchor: r(0.0, 0.0, 0.0, 0.0),
            rich: editor::Editor::default(),
            network_camera: Default::default(),
            flow_camera: Default::default(),
            network_selected: "db".into(),
            flow_selected: "build".into(),
            survey_result: None,
            survey_role: None,
            survey_stack: [false; 3],
            survey_feedback: String::new(),
            section_positions: HashMap::new(),
        }
    }
}
impl Gallery {
    pub fn background(&self) -> wgpu::Color {
        let c = Palette::new(self.dark).bg;
        wgpu::Color {
            r: c[0] as f64,
            g: c[1] as f64,
            b: c[2] as f64,
            a: 1.0,
        }
    }
    pub fn scroll_by(&mut self, delta: f32) {
        if self.overlay.is_none() {
            self.scroll =
                (self.scroll + delta).clamp(0.0, (self.content_height - self.viewport).max(0.0));
        }
    }
    pub fn dismiss(&mut self) {
        if self.overlay.is_none() && self.rich.editing {
            self.rich.cancel();
        }
        self.overlay = None;
        self.cancel_drag();
    }
    pub fn cancel_drag(&mut self) {
        self.drag = None;
        self.task_drag = None;
    }
    pub fn frame(&mut self, input: Input, width: f32, height: f32, time: f32) -> Vec<DrawCommand> {
        self.viewport = height;
        self.rich.time = time;
        if self.timeout.is_some_and(|t| time >= t) {
            self.timeout = None;
        }
        let background_input = if self.overlay.is_some() {
            Input::default()
        } else {
            input.clone()
        };
        let mut ui = std::mem::take(&mut self.ui);
        ui.begin(background_input.clone());
        // Capture before content so a drag cannot activate an underlying control.
        if self.content_height > height && self.overlay.is_none() {
            let thumb = (height * height / self.content_height)
                .max(24.0)
                .min(height);
            let travel = (height - thumb).max(1.0);
            let range = (self.content_height - height).max(0.0);
            let offset = self.scroll / range.max(1.0) * travel;
            let response = ui.interact("gallery-scrollbar", r(width - 14.0, 0.0, 14.0, height));
            if response.active {
                if let Some(pt) = input.pointer {
                    let grab = *self.scrollbar_grab.get_or_insert_with(|| {
                        if pt.y >= offset && pt.y <= offset + thumb {
                            pt.y - offset
                        } else {
                            thumb * 0.5
                        }
                    });
                    self.scroll = ((pt.y - grab) / travel).clamp(0.0, 1.0) * range;
                }
            } else {
                self.scrollbar_grab = None;
            }
        } else {
            self.scrollbar_grab = None;
        }
        let palette = Palette::new(self.dark);
        let x = ((width - 1000.0) * 0.5).max(0.0) + 20.0;
        let w = (width - 40.0).min(960.0).max(280.0);
        let mut p = Painter {
            ui: &mut ui,
            input: &background_input,
            palette,
            scroll: self.scroll,
            viewport: Rect {
                x: 0.0,
                y: 0.0,
                width,
                height,
            },
            glass: self.glass,
        };
        p.ui.clip = Some(p.viewport);
        let mut y = 40.0;
        p.label(
            x,
            y,
            w.min(480.0),
            34.0,
            "Dioxus Component Gallery",
            28.0,
            700,
            palette.fg,
        );
        p.label(
            x,
            y + 42.0,
            w.min(480.0),
            22.0,
            "A premium collection of reusable and highly customizable components.",
            14.0,
            400,
            palette.muted,
        );
        let controls_x = if w >= 920.0 { x + w - 460.0 } else { x };
        let controls_y = if w >= 920.0 { y } else { y + 84.0 };
        p.label(
            controls_x,
            controls_y,
            220.0,
            20.0,
            "Select Theme",
            13.0,
            500,
            palette.fg,
        );
        p.label(
            controls_x + 240.0,
            controls_y + 15.0,
            160.0,
            20.0,
            "Glass Strength",
            13.0,
            500,
            palette.fg,
        );
        p.right(
            controls_x + 404.0,
            controls_y + 15.0,
            56.0,
            20.0,
            &format!("{}%", (self.glass * 100.0) as i32),
            13.0,
            palette.muted,
        );
        if p.button(
            "theme",
            r(controls_x, controls_y + 26.0, 220.0, 38.0),
            if self.dark { "Black" } else { "White" },
            2,
            false,
        ) {
            self.menu(
                "theme",
                vec!["White", "Black"],
                r(controls_x, controls_y + 64.0, 220.0, 80.0),
            );
        }
        p.slider(
            "glass",
            r(controls_x + 240.0, controls_y + 32.0, 220.0, 38.0),
            &mut self.glass,
            0.0,
            2.0,
            0.05,
            false,
        );
        y = controls_y + 84.0;
        p.line(x, y, x + w, y, palette.border, 1.0);
        y += 40.0;
        self.section(&mut p, &mut y, x, w, "Navigation");
        p.card(r(x, y, w, 332.0));
        p.subheading(x + 20.0, y + 20.0, w - 40.0, "Horizontal Menu");
        p.box_(
            r(x + 20.0, y + 52.0, 428.0, 38.0),
            palette.card,
            palette.border,
            8.0,
        );
        p.box_(
            r(x + 20.0, y + 140.0, 240.0, 170.0),
            palette.card,
            palette.border,
            8.0,
        );
        let mut bx = x + 20.0;
        for (id, label, bw) in [
            ("products", "Products  ˅", 106.0),
            ("solutions", "Solutions  ˅", 108.0),
            ("pricing", "Pricing", 76.0),
            ("docs", "Documentation", 120.0),
        ] {
            if p.button(
                &format!("nav-{id}"),
                r(bx, y + 52.0, bw, 38.0),
                label,
                3,
                false,
            ) {
                self.nav_menu(id, r(bx, y + 90.0, 260.0, 190.0));
            }
            bx += bw + 6.0;
        }
        p.subheading(x + 20.0, y + 110.0, w - 40.0, "Vertical Menu");
        for (i, (id, label)) in [
            ("products", "Products                 ›"),
            ("solutions", "Solutions                ›"),
            ("pricing", "Pricing"),
            ("docs", "Documentation"),
        ]
        .iter()
        .enumerate()
        {
            let rect = r(x + 20.0, y + 146.0 + i as f32 * 42.0, 240.0, 38.0);
            if p.button(&format!("vertical-{id}"), rect, label, 3, false) {
                self.nav_menu(id, r(rect.x + 244.0, rect.y, 260.0, 200.0));
            }
        }
        y += 372.0;
        self.section(&mut p, &mut y, x, w, "Browser Containers");
        p.label(x,y,w,22.0,"Presentation-only browser chrome and controlled tabs. The caller supplies addresses, panels, state, and behavior.",14.0,400,palette.muted);
        y += 38.0;
        p.card(r(x, y, w, 430.0));
        p.label(
            x + 20.0,
            y + 16.0,
            400.0,
            30.0,
            "Browser + TabbedContainer",
            18.0,
            600,
            palette.fg,
        );
        p.badge(
            r(x + w - 155.0, y + 20.0, 135.0, 26.0),
            &format!("{} interactions", self.interactions),
            BLUE,
            false,
        );
        let browser = r(x + 20.0, y + 60.0, w - 40.0, 344.0);
        p.box_(browser, palette.card, palette.border, 12.0);
        if p.button(
            "browser-back",
            r(browser.x + 8.0, browser.y + 8.0, 34.0, 34.0),
            "‹",
            3,
            false,
        ) {
            self.interactions += 1;
        }
        if p.button(
            "browser-forward",
            r(browser.x + 44.0, browser.y + 8.0, 34.0, 34.0),
            "›",
            3,
            false,
        ) {
            self.interactions += 1;
        }
        if p.field(
            "address",
            r(
                browser.x + 90.0,
                browser.y + 8.0,
                browser.width - 142.0,
                34.0,
            ),
            &mut self.address,
            "Address",
            false,
        ) {
            self.interactions += 1;
        }
        if p.button(
            "browser-go",
            r(
                browser.x + browser.width - 44.0,
                browser.y + 8.0,
                34.0,
                34.0,
            ),
            "↵",
            3,
            false,
        ) {
            if let Some(t) = self.tabs.get_mut(self.tab) {
                *t = self.address.clone();
            }
            self.interactions += 1;
        }
        let mut close = None;
        for (i, tab) in self.tabs.clone().iter().enumerate() {
            let tx = browser.x + 8.0 + i as f32 * 180.0;
            if tx + 180.0 > browser.x + browser.width - 40.0 {
                break;
            }
            if p.button(
                &format!("tab-{i}"),
                r(tx, browser.y + 52.0, 144.0, 34.0),
                if tab.is_empty() { "New tab" } else { tab },
                if self.tab == i { 1 } else { 3 },
                false,
            ) {
                self.tab = i;
                self.address = tab.clone();
                self.interactions += 1;
            }
            if p.button(
                &format!("close-{i}"),
                r(tx + 144.0, browser.y + 52.0, 28.0, 34.0),
                "×",
                3,
                false,
            ) {
                close = Some(i);
            }
        }
        if let Some(i) = close {
            self.tabs.remove(i);
            if self.tabs.is_empty() {
                self.tabs.push(String::new());
            }
            self.tab = self.tab.min(self.tabs.len().saturating_sub(1));
            self.address = self.tabs.get(self.tab).cloned().unwrap_or_default();
            self.interactions += 1;
        }
        if p.button(
            "add-tab",
            r(
                browser.x + browser.width - 40.0,
                browser.y + 52.0,
                32.0,
                34.0,
            ),
            "+",
            3,
            false,
        ) {
            self.tabs.push(String::new());
            self.tab = self.tabs.len() - 1;
            self.address = String::new();
            self.interactions += 1;
        }
        p.label(
            browser.x + 24.0,
            browser.y + 170.0,
            browser.width - 48.0,
            44.0,
            self.tabs.get(self.tab).map_or("Open a page", |s| {
                if s.is_empty() {
                    "Open a page"
                } else {
                    s
                }
            }),
            32.0,
            700,
            palette.fg,
        );
        p.ui.commands.last_mut().unwrap().style.text_align = TextAlign::Center;
        p.label(
            browser.x + 24.0,
            browser.y + 224.0,
            browser.width - 48.0,
            48.0,
            if self.address.is_empty() {
                "Type an address above and press Enter."
            } else {
                "The application can load this address into any renderer it owns."
            },
            14.0,
            400,
            palette.muted,
        );
        p.ui.commands.last_mut().unwrap().style.text_align = TextAlign::Center;
        y += 470.0;
        self.buttons(&mut p, &mut y, x, w, time);
        self.forms(&mut p, &mut y, x, w);
        self.feedback(&mut p, &mut y, x, w, time);
        self.metrics(&mut p, &mut y, x, w, time);
        self.table(&mut p, &mut y, x, w);
        self.planning(&mut p, &mut y, x, w);
        self.bills(&mut p, &mut y, x, w);
        self.graphs(&mut p, &mut y, x, w, time);
        self.content_height = y + 40.0;
        if self.content_height > height {
            let thumb = (height * height / self.content_height).max(24.0);
            let offset = self.scroll / (self.content_height - height).max(1.0) * (height - thumb);
            p.scroll = 0.0;
            p.box_(
                r(width - 9.0, offset, 7.0, thumb),
                palette.border,
                [0.0; 4],
                4.0,
            );
        }
        let mut commands = ui.end().to_vec();
        self.ui = ui;
        if let Some(overlay) = self.overlay.clone() {
            let mut oui = std::mem::take(&mut self.overlay_ui);
            oui.begin(input.clone());
            let mut op = Painter {
                ui: &mut oui,
                input: &input,
                palette,
                scroll: 0.0,
                viewport: r(0.0, 0.0, width, height),
                glass: self.glass,
            };
            op.ui.clip = Some(op.viewport);
            self.overlay(&mut op, overlay, width, height);
            commands.extend_from_slice(oui.end());
            self.overlay_ui = oui;
        }
        // Custom geometry uses command-local coordinates so window composition
        // can translate it using the same rectangle as standard UI commands.
        for command in &mut commands {
            if let Material::Custom { name, parameters } = &mut command.style.material {
                let stride = match name.as_str() {
                    "gallery.triangles" => 2,
                    "gallery.colored-triangles" => 6,
                    _ => continue,
                };
                for vertex in parameters.chunks_exact_mut(stride) {
                    vertex[0] -= command.rect.x;
                    vertex[1] -= command.rect.y;
                }
            }
        }
        commands
    }
    fn section(&mut self, p: &mut Painter, y: &mut f32, x: f32, w: f32, title: &str) {
        self.section_positions.insert(title.into(), *y);
        p.box_(r(x, *y, 4.0, 24.0), BLUE, [0.0; 4], 0.0);
        p.label(x + 16.0, *y, w - 16.0, 24.0, title, 20.0, 600, p.palette.fg);
        *y += 40.0;
    }
    fn menu(&mut self, label: &str, choices: Vec<&str>, anchor: Rect) {
        self.overlay = Some(Overlay::Menu {
            label: label.into(),
            choices: choices.into_iter().map(String::from).collect(),
            anchor: Rect {
                y: anchor.y - self.scroll,
                ..anchor
            },
        });
    }
    fn nav_menu(&mut self, id: &str, anchor: Rect) {
        match id {
            "products" => self.menu("Products", vec!["Platform  ›", "Integrations  ›"], anchor),
            "solutions" => self.menu("Solutions", vec!["Enterprise  ›"], anchor),
            _ => {
                self.selected = id.into();
            }
        }
    }
    fn buttons(&mut self, p: &mut Painter, y: &mut f32, x: f32, w: f32, time: f32) {
        self.section(p, y, x, w, "1. Buttons");
        p.card(r(x, *y, w, 572.0));
        let x = x + 20.0;
        let mut cy = *y + 20.0;
        p.subheading(x, cy, w - 40.0, "Variants");
        cy += 30.0;
        let mut bx = x;
        for (i, (label, width)) in [
            ("Primary", 98.0),
            ("Secondary", 116.0),
            ("Outline", 94.0),
            ("Text Button", 126.0),
        ]
        .iter()
        .enumerate()
        {
            p.button(
                &format!("variant-{i}"),
                r(bx, cy, *width, 38.0),
                label,
                i as u8,
                false,
            );
            bx += width + 12.0;
        }
        cy += 62.0;
        p.subheading(x, cy, w - 40.0, "Sizes");
        cy += 30.0;
        for (i, (label, h, bw)) in [
            ("Small", 30.0, 76.0),
            ("Medium", 38.0, 102.0),
            ("Large", 46.0, 110.0),
        ]
        .iter()
        .enumerate()
        {
            p.button(
                &format!("size-{i}"),
                r(x + i as f32 * 122.0, cy + (46.0 - h) * 0.5, *bw, *h),
                label,
                0,
                false,
            );
        }
        cy += 70.0;
        p.subheading(x, cy, w - 40.0, "States");
        cy += 30.0;
        p.button("disabled", r(x, cy, 106.0, 38.0), "Disabled", 0, true);
        p.button(
            "loading",
            r(x + 118.0, cy, 108.0, 38.0),
            "◌ Loading",
            0,
            true,
        );
        if p.button(
            "start-loading",
            r(x + 238.0, cy, 144.0, 38.0),
            if self.loading {
                "◌ Loading"
            } else {
                "Click to Load"
            },
            0,
            self.loading,
        ) {
            self.loading = true;
        }
        if self.loading
            && p.button(
                "reset-loading",
                r(x + 394.0, cy, 78.0, 38.0),
                "Reset",
                3,
                false,
            )
        {
            self.loading = false;
        }
        if p.button(
            "timeout",
            r(x + 490.0, cy, 186.0, 38.0),
            if self.timeout.is_some() {
                "◌ Loading"
            } else {
                "Load (3s Timeout)"
            },
            0,
            self.timeout.is_some(),
        ) {
            self.timeout = Some(time + 3.0);
        }
        cy += 62.0;
        p.subheading(x, cy, w - 40.0, "Circular Buttons");
        cy += 30.0;
        for (i, (label, size, variant, disabled)) in [
            ("S", 30.0, 0, false),
            ("M", 38.0, 0, false),
            ("L", 46.0, 0, false),
            ("2", 38.0, 1, false),
            ("O", 38.0, 2, false),
            ("T", 38.0, 3, false),
            ("D", 38.0, 0, true),
            ("◌", 38.0, 0, true),
            ("R", 38.0, 4, false),
        ]
        .iter()
        .enumerate()
        {
            p.circular(
                &format!("circle-{i}"),
                r(x + i as f32 * 58.0, cy + (46.0 - size) * 0.5, *size, *size),
                label,
                *variant,
                *disabled,
            );
        }
        cy += 70.0;
        p.subheading(x, cy, w - 40.0, "Selectable Buttons (Tags / Filters)");
        cy += 30.0;
        if p.button(
            "selectable",
            r(x, cy, 130.0, 38.0),
            if self.selectable {
                "✓ Selectable"
            } else {
                "Selectable"
            },
            if self.selectable { 5 } else { 1 },
            false,
        ) {
            self.selectable = !self.selectable;
        }
        p.button(
            "not-selected",
            r(x + 142.0, cy, 136.0, 38.0),
            "Not selected",
            1,
            false,
        );
        p.button(
            "selected",
            r(x + 290.0, cy, 130.0, 38.0),
            "✓ Selected",
            5,
            false,
        );
        p.button(
            "tag-disabled",
            r(x + 432.0, cy, 100.0, 38.0),
            "Disabled",
            1,
            true,
        );
        cy += 62.0;
        p.subheading(x, cy, w - 40.0, "Aspect Ratio Selector");
        cy += 30.0;
        let ratios = ["1:1", "16:9", "9:16", "4:3", "3:4"];
        if p.button(
            "aspect",
            r(x, cy, 150.0, 38.0),
            ratios[self.aspect],
            2,
            false,
        ) {
            self.menu("aspect", ratios.to_vec(), r(x, cy + 40.0, 150.0, 200.0));
        }
        p.label(
            x + 166.0,
            cy,
            250.0,
            38.0,
            &format!("{} · aspect ratio", ratios[self.aspect]),
            13.0,
            400,
            p.palette.muted,
        );
        p.button(
            "aspect-disabled",
            r(x + 420.0, cy, 150.0, 38.0),
            "9:16",
            2,
            true,
        );
        *y += 612.0;
    }
    fn forms(&mut self, p: &mut Painter, y: &mut f32, x: f32, w: f32) {
        self.section(p, y, x, w, "2. Form Controls");
        p.card(r(
            x,
            *y,
            w,
            1410.0 + if self.rich.editing { 220.0 } else { 0.0 },
        ));
        let x = x + 20.0;
        let mut cy = *y + 20.0;
        for (i, label) in ["Username", "Email", "Preferred Language"]
            .iter()
            .enumerate()
        {
            p.label(x, cy, 140.0, 38.0, label, 13.0, 500, p.palette.fg);
            match i {
                0 => {
                    p.field(
                        "username",
                        r(x + 140.0, cy, 360.0, 38.0),
                        &mut self.username,
                        "Enter username...",
                        false,
                    );
                }
                1 => {
                    let mut email = "user@example.com".to_string();
                    p.field(
                        "email",
                        r(x + 140.0, cy, 360.0, 38.0),
                        &mut email,
                        "",
                        false,
                    );
                }
                _ => {
                    if p.dropdown("language", r(x + 140.0, cy, 360.0, 38.0), &self.language) {
                        self.menu(
                            "language",
                            vec!["Rust", "TypeScript", "Python", "Go"],
                            r(x + 140.0, cy + 40.0, 360.0, 160.0),
                        );
                    }
                }
            }
            cy += if i == 0 { 70.0 } else { 62.0 };
        }
        p.label(
            x + 140.0,
            *y + 60.0,
            360.0,
            20.0,
            if !self.username.is_empty() && self.username.chars().count() < 3 {
                "Must be at least 3 characters long."
            } else {
                "Must be at least 3 characters long."
            },
            12.0,
            400,
            if !self.username.is_empty() && self.username.chars().count() < 3 {
                RED
            } else {
                p.palette.muted
            },
        );
        p.check(
            "terms",
            r(x, cy, 300.0, 32.0),
            "Accept terms and conditions",
            &mut self.accepted,
            false,
            false,
        );
        let mut yes = true;
        p.check(
            "terms-disabled",
            r(x + 324.0, cy, 320.0, 32.0),
            "Disabled Checkbox (Checked)",
            &mut yes,
            true,
            false,
        );
        cy += 54.0;
        p.check(
            "notifications",
            r(x, cy, 280.0, 32.0),
            "Enable notifications",
            &mut self.notifications,
            false,
            true,
        );
        let mut no = false;
        p.check(
            "switch-disabled",
            r(x + 304.0, cy, 260.0, 32.0),
            "Disabled Switch",
            &mut no,
            true,
            true,
        );
        cy += 58.0;
        p.line(x, cy, x + 500.0, cy, p.palette.border, 1.0);
        cy += 16.0;
        p.subheading(x, cy, 500.0, "One-Time Password (OTP)");
        cy += 28.0;
        p.otp(r(x, cy, 300.0, 44.0), &mut self.otp);
        self.otp = self
            .otp
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(6)
            .collect();
        cy += 48.0;
        p.label(
            x,
            cy,
            500.0,
            22.0,
            &format!("Current value in parent state: '{}'", self.otp),
            12.0,
            400,
            p.palette.muted,
        );
        cy += 42.0;
        p.subheading(x, cy, 500.0, &format!("Slider (Value: {:.1})", self.slider));
        cy += 28.0;
        p.slider(
            "form-slider",
            r(x, cy, 500.0, 24.0),
            &mut self.slider,
            0.0,
            100.0,
            0.5,
            false,
        );
        cy += 46.0;
        p.subheading(x, cy, 500.0, "Disabled Slider");
        cy += 28.0;
        let mut value = 30.0;
        p.slider(
            "slider-disabled",
            r(x, cy, 500.0, 24.0),
            &mut value,
            0.0,
            100.0,
            1.0,
            true,
        );
        cy += 56.0;
        p.subheading(x, cy, 500.0, "Appointment Date & Time");
        cy += 28.0;
        if p.button(
            "date",
            r(x, cy, 500.0, 38.0),
            &format!("{}                   ▦", self.date),
            2,
            false,
        ) {
            self.calendar = pickers::Calendar::from_value(&self.date);
            self.picker_anchor = p.screen(r(x, cy + 42.0, 380.0, 418.0));
            self.overlay = Some(Overlay::Calendar);
        }
        cy += 58.0;
        p.subheading(x, cy, 500.0, "Disabled Date & Time Picker");
        cy += 28.0;
        p.button(
            "date-disabled",
            r(x, cy, 500.0, 38.0),
            "2026-07-16 14:00",
            2,
            true,
        );
        cy += 62.0;
        p.subheading(x, cy, 500.0, "Accent Color Picker (Popover)");
        cy += 28.0;
        if p.button("color", r(x, cy, 260.0, 38.0), &self.color, 2, false) {
            self.color_editor = pickers::ColorEditor::new(&self.color);
            self.picker_anchor = p.screen(r(x, cy + 42.0, 380.0, 408.0));
            self.overlay = Some(Overlay::Color);
        }
        cy += 60.0;
        p.subheading(x, cy, 500.0, "Inline Color Picker");
        cy += 28.0;
        if p.color_editor("inline", r(x, cy, 320.0, 344.0), &mut self.inline_editor) {
            self.inline_color = self.inline_editor.hex.clone();
        }
        cy += 364.0;
        p.subheading(x, cy, 500.0, "Editable Text / WYSIWYG");
        cy += 28.0;
        self.rich.draw(
            p,
            r(
                x,
                cy,
                w - 80.0,
                if self.rich.editing { 300.0 } else { 80.0 },
            ),
        );
        *y += 1450.0 + if self.rich.editing { 220.0 } else { 0.0 };
    }
    fn feedback(&mut self, p: &mut Painter, y: &mut f32, x: f32, w: f32, time: f32) {
        self.section(p, y, x, w, "3. Feedback & Badges");
        p.card(r(x, *y, w, 620.0));
        let x = x + 20.0;
        let mut cy = *y + 20.0;
        p.subheading(x, cy, w - 40.0, "Badges (Variants, Sizes & Styles)");
        cy += 32.0;
        for row in 0..2 {
            p.label(
                x,
                cy,
                if row == 0 { 70.0 } else { 255.0 },
                30.0,
                if row == 0 {
                    "Normal:"
                } else {
                    "Large & Borderless (Metrics Style):"
                },
                13.0,
                400,
                p.palette.muted,
            );
            let start = x + if row == 0 { 80.0 } else { 265.0 };
            for (i, (label, c)) in [
                ("Default", p.palette.muted),
                ("Success", GREEN),
                ("Warning", YELLOW),
                ("Error", RED),
                ("Info", BLUE),
            ]
            .iter()
            .enumerate()
            {
                p.badge(
                    r(
                        start + i as f32 * 98.0,
                        cy,
                        88.0,
                        if row == 0 { 26.0 } else { 32.0 },
                    ),
                    label,
                    *c,
                    row == 1,
                );
            }
            cy += 48.0;
        }
        p.subheading(x, cy, w - 40.0, "Notifications");
        cy += 32.0;
        for (title, body, c) in [
            (
                "System Update",
                "A new software update is available. Please upgrade.",
                BLUE,
            ),
            (
                "Operation Successful",
                "Your settings have been saved correctly.",
                GREEN,
            ),
            (
                "Low Disk Space",
                "Your storage is almost full. Clean some space.",
                YELLOW,
            ),
            (
                "Connection Failed",
                "Unable to connect to the database. Please try again.",
                RED,
            ),
        ] {
            p.notification(r(x, cy, w - 40.0, 66.0), title, body, c);
            cy += 78.0;
        }
        p.subheading(x, cy, w - 40.0, "Spinners (Sizes & Variants)");
        cy += 34.0;
        for (i, (size, label)) in [(16.0, "Small"), (24.0, "Medium"), (32.0, "Large")]
            .iter()
            .enumerate()
        {
            p.spinner(x + 16.0 + i as f32 * 155.0, cy + 16.0, *size, time, BLUE);
            p.label(
                x + 44.0 + i as f32 * 155.0,
                cy,
                100.0,
                32.0,
                label,
                13.0,
                400,
                p.palette.muted,
            );
        }
        *y += 660.0;
        self.section(p, y, x - 20.0, w, "4. Modals & Dynamic Form Flow Engine");
        p.card(r(x - 20.0, *y, w, 128.0));
        p.label(x,*y+16.0,w-40.0,44.0,"Explore modal dialogs as well as dynamic, conditional step-by-step survey flows with real-time branching logic and validation.",14.0,400,p.palette.muted);
        if p.button(
            "modal",
            r(x, *y + 70.0, 210.0, 38.0),
            "Open Standard Modal",
            0,
            false,
        ) {
            self.overlay = Some(Overlay::Modal);
        }
        if p.button(
            "survey",
            r(x + 222.0, *y + 70.0, 250.0, 38.0),
            "Launch Dynamic Form Demo",
            2,
            false,
        ) {
            self.overlay = Some(Overlay::Survey(0));
        }
        *y += 168.0;
    }
    fn metrics(&mut self, p: &mut Painter, y: &mut f32, x: f32, w: f32, time: f32) {
        self.section(p, y, x, w, "5. Metric & Data Visualization");
        p.card(r(x, *y, w, 680.0));
        let x = x + 20.0;
        let cy = *y + 20.0;
        p.subheading(x, cy, w - 40.0, "Metric Cards (Directly using Card)");
        let columns = if w >= 750.0 { 3 } else { 2 };
        let cw = (w - 40.0 - 20.0 * (columns - 1) as f32) / columns as f32;
        for (i, label) in [
            "Requests Count",
            "CPU Usage",
            "System Status",
            "Memory Load",
        ]
        .iter()
        .enumerate()
        {
            let rect = r(
                x + (i % columns) as f32 * (cw + 20.0),
                cy + 40.0 + (i / columns) as f32 * 144.0,
                cw,
                124.0,
            );
            p.card(rect);
            p.label(
                rect.x + 16.0,
                rect.y + 14.0,
                cw - 32.0,
                20.0,
                label,
                13.0,
                500,
                p.palette.muted,
            );
            p.label(
                rect.x + 16.0,
                rect.y + 42.0,
                cw - 32.0,
                36.0,
                &match i {
                    0 => format!("{} reqs", 1234 + (time as u32) * 10),
                    1 => format!("{:.1} %", 75.4 + time.sin() * 4.0),
                    2 => "Healthy".into(),
                    _ => format!("{:.1} GB", 4.2 + time.sin() * 0.2),
                },
                28.0,
                600,
                if i == 2 { GREEN } else { p.palette.fg },
            );
            if i == 1 {
                p.progress(r(rect.x + 16.0, rect.y + 92.0, cw - 32.0, 8.0), 0.754, BLUE);
            }
            if i == 3 {
                p.sparkline(
                    r(rect.x + 16.0, rect.y + 84.0, cw - 32.0, 26.0),
                    &[1.2, 1.5, 2.0, 1.8, 2.4, 3.1, 2.8, 3.5, 4.2],
                    BLUE,
                );
            }
        }
        let mut cy = cy + 350.0;
        p.subheading(x, cy, 400.0, "Progress Bars");
        cy += 32.0;
        p.progress(r(x, cy, 400.0, 8.0), 0.3, BLUE);
        p.progress(r(x, cy + 38.0, 400.0, 8.0), 0.65, BLUE);
        p.label(
            x,
            cy + 52.0,
            400.0,
            20.0,
            "Start                                                        Goal",
            12.0,
            400,
            p.palette.muted,
        );
        cy += 94.0;
        p.subheading(x, cy, 400.0, "Sparklines (Trend Lines)");
        cy += 30.0;
        p.sparkline(
            r(x, cy, (w - 64.0) * 0.5, 55.0),
            &[10.0, 15.0, 8.0, 25.0, 18.0, 30.0],
            BLUE,
        );
        p.sparkline(
            r(x + (w - 40.0) * 0.5, cy, (w - 64.0) * 0.5, 55.0),
            &[30.0, 25.0, 40.0, 35.0, 50.0, 45.0, 60.0],
            GREEN,
        );
        cy += 74.0;
        p.subheading(x, cy, w - 40.0, "Units (Reusable Values)");
        cy += 26.0;
        for (i, (v, size)) in [("1,245 reqs", 14.0), ("75.4 %", 20.0), ("4.2 GB", 28.0)]
            .iter()
            .enumerate()
        {
            p.label(
                x + i as f32 * 210.0,
                cy,
                200.0,
                34.0,
                v,
                *size,
                500,
                p.palette.fg,
            );
        }
        *y += 720.0;
    }
    fn table(&mut self, p: &mut Painter, y: &mut f32, x: f32, w: f32) {
        self.section(p, y, x, w, "6. Table Component");
        p.card(r(x, *y, w, 394.0));
        let x = x + 20.0;
        let cy = *y + 20.0;
        for (i, label) in ["Striped", "Hoverable", "Compact", "Borderless", "Loading"]
            .iter()
            .enumerate()
        {
            p.check(
                &format!("table-option-{i}"),
                r(x + i as f32 * 160.0, cy, 156.0, 28.0),
                label,
                &mut self.table[i],
                false,
                false,
            );
        }
        let widths = [0.08, 0.24, 0.32, 0.18, 0.18];
        let columns = ["ID", "Name", "Email", "Role", "Status"];
        let mut tx = x;
        for (i, label) in columns.iter().enumerate() {
            let cw = (w - 40.0) * widths[i];
            if p.button(
                &format!("sort-{i}"),
                r(tx, cy + 48.0, cw, 42.0),
                &format!(
                    "{label} {}",
                    if self.sort == i {
                        if self.descending {
                            "↓"
                        } else {
                            "↑"
                        }
                    } else {
                        ""
                    }
                ),
                1,
                false,
            ) {
                if self.sort == i {
                    self.descending = !self.descending
                } else {
                    self.sort = i;
                    self.descending = false;
                }
            }
            tx += cw;
        }
        let mut rows = vec![
            [
                "1",
                "Danilo Guanabara",
                "danilo@sensorial.systems",
                "Admin",
                "Active",
            ],
            ["2", "Alice Smith", "alice@example.com", "User", "Inactive"],
            ["3", "Bob Jones", "bob@example.com", "Editor", "Active"],
            [
                "4",
                "Charlie Brown",
                "charlie@example.com",
                "User",
                "Inactive",
            ],
        ];
        rows.sort_by(|a, b| a[self.sort].cmp(b[self.sort]));
        if self.descending {
            rows.reverse();
        }
        for (i, row) in rows.iter().enumerate() {
            let yy = cy + 90.0 + i as f32 * if self.table[2] { 32.0 } else { 44.0 };
            let rh = if self.table[2] { 32.0 } else { 44.0 };
            if (self.table[0] && i % 2 == 1)
                || (self.table[1]
                    && p.input
                        .pointer
                        .is_some_and(|pt| p.screen(r(x, yy, w - 40.0, rh)).contains(pt)))
            {
                p.box_(r(x, yy, w - 40.0, rh), p.palette.secondary, [0.0; 4], 0.0);
            }
            let mut tx = x;
            for (j, cell) in row.iter().enumerate() {
                let cw = (w - 40.0) * widths[j];
                if j >= 3 {
                    p.badge(
                        r(tx + 8.0, yy + (rh - 24.0) * 0.5, cw - 16.0, 24.0),
                        cell,
                        if j == 3 {
                            BLUE
                        } else if *cell == "Active" {
                            GREEN
                        } else {
                            p.palette.muted
                        },
                        j == 3,
                    );
                } else {
                    p.label(
                        tx + 12.0,
                        yy,
                        cw - 20.0,
                        rh,
                        cell,
                        14.0,
                        if j == 0 { 600 } else { 400 },
                        if j == 2 {
                            p.palette.muted
                        } else {
                            p.palette.fg
                        },
                    );
                }
                tx += cw;
            }
            if !self.table[3] {
                p.line(x, yy + rh, x + w - 40.0, yy + rh, p.palette.border, 1.0);
            }
        }
        if self.table[4] {
            p.box_(
                r(x, cy + 90.0, w - 40.0, 220.0),
                [p.palette.bg[0], p.palette.bg[1], p.palette.bg[2], 0.85],
                [0.0; 4],
                0.0,
            );
            p.label(
                x,
                cy + 160.0,
                w - 40.0,
                40.0,
                "Loading…",
                18.0,
                500,
                p.palette.fg,
            );
        }
        *y += 434.0;
    }
    fn planning(&mut self, p: &mut Painter, y: &mut f32, x: f32, w: f32) {
        self.section(p, y, x, w, "7. Planning Components");
        p.label(x,*y,w,34.0,"Reusable workflow and scheduling views. Drag pipeline cards between columns; use Shift-click for multi-selection.",14.0,400,p.palette.muted);
        *y += 50.0;
        let board_height = 190.0
            + (0..5)
                .map(|c| self.pipeline.iter().filter(|(_, col)| *col == c).count())
                .max()
                .unwrap_or(1) as f32
                * 74.0;
        p.card(r(x, *y, w, board_height));
        p.label(
            x + 20.0,
            *y + 20.0,
            w - 40.0,
            26.0,
            "PipelineBoard",
            18.0,
            600,
            p.palette.fg,
        );
        let cw = (w - 56.0) / 5.0;
        for (i, label) in ["Backlog", "Planned", "In progress", "Review", "Done"]
            .iter()
            .enumerate()
        {
            let rect = r(
                x + 20.0 + i as f32 * (cw + 4.0),
                *y + 66.0,
                cw,
                board_height - 94.0,
            );
            p.box_(rect, p.palette.secondary, p.palette.border, 8.0);
            p.label(
                rect.x + 10.0,
                rect.y + 8.0,
                cw - 20.0,
                22.0,
                label,
                13.0,
                600,
                p.palette.fg,
            );
            let mut row = 0;
            for (idx, (title, column)) in self.pipeline.clone().iter().enumerate() {
                if *column == i {
                    let cr = r(
                        rect.x + 8.0,
                        rect.y + 44.0 + row as f32 * 74.0,
                        cw - 16.0,
                        66.0,
                    );
                    row += 1;
                    let response = p.interact(&format!("pipeline-{idx}"), cr);
                    p.box_(
                        cr,
                        p.palette.card,
                        if self.pipeline_selected.contains(&idx) {
                            BLUE
                        } else {
                            p.palette.border
                        },
                        8.0,
                    );
                    p.label(
                        cr.x + 8.0,
                        cr.y + 8.0,
                        cr.width - 16.0,
                        40.0,
                        title,
                        13.0,
                        500,
                        p.palette.fg,
                    );
                    if response.active && self.drag.is_none() {
                        self.drag = Some(idx);
                        if self.shift {
                            if self.pipeline_selected.contains(&idx) {
                                self.pipeline_selected.retain(|v| *v != idx);
                            } else {
                                self.pipeline_selected.push(idx);
                            }
                        } else if !self.pipeline_selected.contains(&idx) {
                            self.pipeline_selected = vec![idx];
                        }
                        self.selected = title.clone();
                    }
                }
            }
            if !p.input.down
                && p.input
                    .pointer
                    .is_some_and(|pt| p.screen(rect).contains(pt))
            {
                if let Some(idx) = self.drag.take() {
                    if self.pipeline_selected.contains(&idx) {
                        for selected in &self.pipeline_selected {
                            self.pipeline[*selected].1 = i;
                        }
                    } else {
                        self.pipeline[idx].1 = i;
                    }
                }
            }
        }
        if !p.input.down {
            self.drag = None;
        }
        *y += board_height + 24.0;
        p.card(r(x, *y, w, 340.0));
        p.label(
            x + 20.0,
            *y + 20.0,
            w - 300.0,
            26.0,
            "Timeline",
            18.0,
            600,
            p.palette.fg,
        );
        if p.button(
            "zoom-out",
            r(x + w - 226.0, *y + 16.0, 96.0, 32.0),
            "− Zoom",
            1,
            false,
        ) {
            self.timeline_zoom = (self.timeline_zoom / 1.16).max(0.6);
        }
        if p.button(
            "zoom-in",
            r(x + w - 118.0, *y + 16.0, 98.0, 32.0),
            "+ Zoom",
            1,
            false,
        ) {
            self.timeline_zoom = (self.timeline_zoom * 1.16).min(1.6);
        }
        let start = x + 190.0;
        let tw = (w - 220.0) * self.timeline_zoom;
        for i in 0..6 {
            let xx = start + i as f32 * tw / 6.0;
            if xx < x + w - 20.0 {
                p.label(
                    xx,
                    *y + 66.0,
                    tw / 6.0,
                    24.0,
                    &format!("Aug {}", 11 + i),
                    12.0,
                    500,
                    p.palette.muted,
                );
                p.line(xx, *y + 92.0, xx, *y + 306.0, p.palette.border, 1.0);
            }
        }
        if p.button(
            "expand-project",
            r(x + 20.0, *y + 100.0, 160.0, 32.0),
            if self.timeline_expanded {
                "˅ Design system"
            } else {
                "› Design system"
            },
            3,
            false,
        ) {
            self.timeline_expanded = !self.timeline_expanded;
        }
        if self.timeline_expanded {
            if p.button(
                "expand-milestone",
                r(x + 32.0, *y + 146.0, 148.0, 32.0),
                if self.timeline_milestone {
                    "˅ Foundation"
                } else {
                    "› Foundation"
                },
                3,
                false,
            ) {
                self.timeline_milestone = !self.timeline_milestone;
            }
            p.progress(r(start + tw * 0.07, *y + 146.0, tw * 0.73, 24.0), 0.6, BLUE);
            if self.timeline_milestone {
                p.label(
                    x + 42.0,
                    *y + 190.0,
                    132.0,
                    30.0,
                    "Semantic tokens",
                    12.0,
                    400,
                    p.palette.fg,
                );
                p.box_(
                    r(start + tw * 0.07, *y + 194.0, tw * 0.58, 22.0),
                    BLUE,
                    [0.0; 4],
                    5.0,
                );
                p.label(
                    x + 42.0,
                    *y + 230.0,
                    132.0,
                    30.0,
                    "Publish alpha",
                    12.0,
                    400,
                    p.palette.fg,
                );
                let rect = r(
                    start + tw * self.task_left / 100.0,
                    *y + 234.0,
                    tw * self.task_width / 100.0,
                    22.0,
                );
                let response = p.interact("timeline-task", rect);
                p.box_(rect, GREEN, [0.0; 4], 5.0);
                if response.active && self.task_drag.is_none() {
                    if let Some(pt) = p.input.pointer {
                        self.task_drag =
                            Some((pt.x > rect.x + rect.width - 10.0, pt.x, self.task_left));
                    }
                }
                if p.input.down {
                    if let (Some((resize, begin, original)), Some(pt)) =
                        (self.task_drag, p.input.pointer)
                    {
                        if resize {
                            self.task_width =
                                ((pt.x - rect.x) / tw * 100.0).clamp(5.0, 100.0 - self.task_left);
                        } else {
                            self.task_left = (original + (pt.x - begin) / tw * 100.0)
                                .clamp(0.0, 100.0 - self.task_width);
                        }
                    }
                } else {
                    self.task_drag = None;
                }
            }
            p.label(
                x + 32.0,
                *y + 274.0,
                145.0,
                28.0,
                "Component docs",
                12.0,
                400,
                p.palette.fg,
            );
            p.box_(
                r(start + tw * 0.42, *y + 276.0, tw * 0.38, 22.0),
                YELLOW,
                [0.0; 4],
                5.0,
            );
        }
        *y += 380.0;
    }
    fn bills(&mut self, p: &mut Painter, y: &mut f32, x: f32, w: f32) {
        self.section(p, y, x, w, "8. Bill Component");
        p.label(x,*y,w,40.0,"Streamlined, display-only bill component displaying item breakdown, quantity, unit cost, and aligned total sum.",14.0,400,p.palette.muted);
        *y += 52.0;
        p.card(r(x, *y, w, 550.0));
        for i in 0..2 {
            let bx = x + 20.0 + i as f32 * (w - 40.0) * 0.5;
            let bw = ((w - 64.0) * 0.5).min(380.0);
            p.label(
                bx,
                *y + 20.0,
                bw,
                22.0,
                if i == 0 {
                    "ACTIVE BILL WIDGET"
                } else {
                    "SECONDARY INVOICE SUMMARY"
                },
                11.0,
                700,
                p.palette.muted,
            );
            p.label(
                bx,
                *y + 58.0,
                bw,
                30.0,
                if i == 0 {
                    "INV-2026-8801"
                } else {
                    "INV-2026-9042"
                },
                20.0,
                600,
                p.palette.fg,
            );
            p.badge(r(bx, *y + 100.0, 92.0, 24.0), "Pending", YELLOW, false);
            let items = if i == 0 {
                vec![
                    (
                        "Cloud Server Cluster (Monthly - 8 Nodes)",
                        "8 × $125.00",
                        "$1,000.00",
                    ),
                    (
                        "UI Kit Enterprise License & Support",
                        "1 × $500.00  ·  −$50.00",
                        "$450.00",
                    ),
                    (
                        "Promotional Refund / Service Credit",
                        "1 × −$150.00",
                        "−$150.00",
                    ),
                    ("Included Tier Support", "1 × $0.00", "$0.00"),
                ]
            } else {
                vec![
                    (
                        "Design System Audit & Review",
                        "1 × 0.004251 SOL",
                        "0.004251 SOL",
                    ),
                    (
                        "Custom Theme Token Preset",
                        "2 × 0.000125 SOL",
                        "0.000250 SOL",
                    ),
                ]
            };
            for (j, (label, quantity, total)) in items.iter().enumerate() {
                let cy = *y + 146.0 + j as f32 * 64.0;
                p.label(bx, cy, bw, 24.0, label, 13.0, 500, p.palette.fg);
                p.label(
                    bx,
                    cy + 26.0,
                    bw * 0.6,
                    20.0,
                    quantity,
                    11.0,
                    400,
                    p.palette.muted,
                );
                p.right(
                    bx + bw * 0.55,
                    cy + 26.0,
                    bw * 0.45,
                    20.0,
                    total,
                    13.0,
                    p.palette.fg,
                );
                p.line(bx, cy + 56.0, bx + bw, cy + 56.0, p.palette.border, 1.0);
            }
            p.label(
                bx,
                *y + 432.0,
                bw * 0.5,
                30.0,
                "Total",
                18.0,
                600,
                p.palette.fg,
            );
            p.right(
                bx + bw * 0.45,
                *y + 432.0,
                bw * 0.55,
                30.0,
                if i == 0 { "$1,420.00" } else { "0.004501 SOL" },
                20.0,
                p.palette.fg,
            );
            p.label(
                bx,
                *y + 476.0,
                bw,
                44.0,
                if i == 0 {
                    "Payment is due within 15 days of invoice date."
                } else {
                    "≈ 0.342976 USDC"
                },
                12.0,
                400,
                p.palette.muted,
            );
        }
        *y += 590.0;
    }
    fn graphs(&mut self, p: &mut Painter, y: &mut f32, x: f32, w: f32, time: f32) {
        self.section(p, y, x, w, "9. Graph Components");
        p.label(x,*y,w,42.0,"Interactive graph visualization components utilizing custom CSS grids, animated SVGs, and level-based auto-layout. Click nodes to select/focus.",14.0,400,p.palette.muted);
        *y += 60.0;
        for (graph, title) in [
            "Flow Graph (CI/CD Pipeline)",
            "Hierarchy Graph (Mind Map / Org Structure)",
            "Network Graph (Mesh Topology)",
            "Node & Edge Shapes Playground",
        ]
        .iter()
        .enumerate()
        {
            if graph == 0 {
                self.flow_graph(p, r(x, *y, w, 410.0), time);
                *y += 442.0;
                continue;
            }
            if graph == 2 {
                self.network_graph(p, r(x, *y, w, 528.0), time);
                *y += 560.0;
                continue;
            }
            if graph == 1 {
                self.hierarchy(p, r(x, *y, w, 460.0));
                *y += 492.0;
                continue;
            }
            p.card(r(x, *y, w, 370.0));
            p.label(
                x + 20.0,
                *y + 18.0,
                w - 40.0,
                30.0,
                title,
                18.0,
                600,
                p.palette.fg,
            );
            let positions: Vec<(f32, f32, &str)> = match graph {
                0 => vec![
                    (0.04, 0.5, "Checkout"),
                    (0.28, 0.25, "Lint & Format"),
                    (0.28, 0.75, "Unit Tests"),
                    (0.56, 0.5, "Build Artifact"),
                    (0.8, 0.5, "Deploy"),
                ],
                1 => vec![
                    (0.03, 0.5, "Core App"),
                    (0.3, 0.2, "UI Layer"),
                    (0.3, 0.5, "Database"),
                    (0.3, 0.8, "GraphQL API"),
                    (0.65, 0.2, "Views & Pages"),
                    (0.65, 0.65, "Shared Parts"),
                ],
                2 => vec![
                    (0.08, 0.2, "Web App"),
                    (0.08, 0.75, "Mobile App"),
                    (0.4, 0.45, "Gateway"),
                    (0.72, 0.15, "Auth"),
                    (0.72, 0.5, "Database"),
                    (0.72, 0.85, "Cache"),
                ],
                _ => vec![
                    (0.04, 0.35, "Box"),
                    (0.28, 0.65, "Circle"),
                    (0.53, 0.35, "Pill"),
                    (0.78, 0.65, "Diamond"),
                ],
            };
            let points: Vec<Point> = positions
                .iter()
                .map(|(px, py, _)| Point::new(x + 28.0 + px * (w - 170.0), *y + 70.0 + py * 220.0))
                .collect();
            let edges = if graph == 0 {
                vec![(0, 1), (0, 2), (1, 3), (2, 3), (3, 4)]
            } else if graph == 1 {
                vec![(0, 1), (0, 2), (0, 3), (1, 4), (1, 5)]
            } else if graph == 2 {
                vec![(0, 2), (1, 2), (2, 3), (2, 4), (2, 5), (3, 4), (4, 5)]
            } else {
                vec![(0, 1), (1, 2), (2, 3)]
            };
            for (a, b) in edges {
                let from = Point::new(points[a].x + 120.0, points[a].y + 22.0);
                let to = Point::new(points[b].x, points[b].y + 22.0);
                p.line(from.x, from.y, (from.x + to.x) * 0.5, from.y, BLUE, 2.0);
                p.line(
                    (from.x + to.x) * 0.5,
                    from.y,
                    (from.x + to.x) * 0.5,
                    to.y,
                    BLUE,
                    2.0,
                );
                p.line((from.x + to.x) * 0.5, to.y, to.x, to.y, BLUE, 2.0);
            }
            for (i, (_, _, label)) in positions.iter().enumerate() {
                let pt = points[i];
                if p.node(
                    &format!("graph-{graph}-{i}"),
                    r(pt.x, pt.y, 124.0, 44.0),
                    label,
                    self.selected == *label,
                    if graph == 3 { i } else { 0 },
                ) {
                    self.selected = label.to_string();
                }
                if self.tree_edit && graph == 1 {
                    p.label(
                        pt.x,
                        pt.y + 48.0,
                        124.0,
                        20.0,
                        "Selected node",
                        11.0,
                        400,
                        p.palette.muted,
                    );
                }
            }
            *y += 402.0;
        }
        p.card(r(x, *y, w, 400.0));
        for (i, title) in ["Pie Chart", "Donut Chart (Center Info)"]
            .iter()
            .enumerate()
        {
            let cx = x + 20.0 + i as f32 * (w - 40.0) * 0.5;
            p.label(cx, *y + 20.0, 400.0, 28.0, title, 18.0, 600, p.palette.fg);
            p.pie(Point::new(cx + 160.0, *y + 196.0), 108.0, i == 1, time);
            p.label(
                cx,
                *y + 328.0,
                400.0,
                30.0,
                "● Direct     ● Referral     ● Social",
                13.0,
                400,
                p.palette.muted,
            );
        }
        *y += 432.0;
        p.card(r(x, *y, w, 390.0));
        p.label(
            x + 20.0,
            *y + 20.0,
            w - 40.0,
            28.0,
            "Stacked Bar Charts",
            18.0,
            600,
            p.palette.fg,
        );
        p.subheading(
            x + 20.0,
            *y + 66.0,
            w - 40.0,
            "Horizontal Stacked Bar Chart (Normalized 100%)",
        );
        for (i, parts) in [[400.0, 300.0, 200.0], [550.0, 250.0, 350.0]]
            .iter()
            .enumerate()
        {
            let cy = *y + 104.0 + i as f32 * 44.0;
            p.label(
                x + 20.0,
                cy,
                100.0,
                28.0,
                &format!("Q{} 2026", i + 1),
                13.0,
                500,
                p.palette.fg,
            );
            let mut left = x + 128.0;
            let total = parts.iter().sum::<f32>();
            for (j, value) in parts.iter().enumerate() {
                let bw = (w - 164.0) * value / total;
                p.box_(
                    r(left, cy, bw, 28.0),
                    [BLUE, GREEN, YELLOW][j],
                    [0.0; 4],
                    0.0,
                );
                left += bw;
            }
        }
        p.subheading(
            x + 20.0,
            *y + 214.0,
            w - 40.0,
            "Vertical Stacked Bar Chart (Absolute Values)",
        );
        for i in 0..3 {
            let left = x + 64.0 + i as f32 * 180.0;
            for j in 0..3 {
                p.box_(
                    r(
                        left,
                        *y + 322.0 - j as f32 * (18.0 + i as f32 * 10.0),
                        44.0,
                        18.0 + i as f32 * 10.0,
                    ),
                    [BLUE, rgb(0x06b6d4), YELLOW][j],
                    [0.0; 4],
                    0.0,
                );
            }
            p.label(
                left - 10.0,
                *y + 346.0,
                80.0,
                24.0,
                &format!("{}", 2024 + i),
                12.0,
                400,
                p.palette.muted,
            );
        }
        *y += 430.0;
    }
    fn overlay(&mut self, p: &mut Painter, overlay: Overlay, width: f32, height: f32) {
        match overlay {
            Overlay::Menu {
                label,
                choices,
                anchor,
            } => {
                let rect = r(
                    anchor.x.min(width - anchor.width - 12.0).max(12.0),
                    anchor
                        .y
                        .min(height - choices.len() as f32 * 38.0 - 20.0)
                        .max(12.0),
                    anchor.width,
                    choices.len() as f32 * 38.0 + 12.0,
                );
                p.shadow(
                    Rect {
                        y: rect.y + 6.0,
                        ..rect
                    },
                    10.0,
                    20.0,
                    if p.palette.dark {
                        [0.0, 0.0, 0.0, 0.50]
                    } else {
                        [0.06, 0.09, 0.18, 0.12]
                    },
                );
                p.box_(rect, p.palette.card, p.palette.border, 10.0);
                if p.input.down && p.input.pointer.is_some_and(|pt| !rect.contains(pt)) {
                    self.overlay = None;
                }
                for (i, choice) in choices.iter().enumerate() {
                    if p.button(
                        &format!("menu-choice-{i}"),
                        r(
                            rect.x + 6.0,
                            rect.y + 6.0 + i as f32 * 38.0,
                            rect.width - 12.0,
                            34.0,
                        ),
                        choice,
                        3,
                        false,
                    ) {
                        self.overlay = None;
                        match label.as_str() {
                            "theme" => self.dark = i == 1,
                            "language" => self.language = choice.clone(),
                            "aspect" => self.aspect = i,
                            "Products" => {
                                self.overlay = Some(Overlay::Menu {
                                    label: choice.clone(),
                                    choices: if i == 0 {
                                        vec!["Analytics".into(), "Automation".into()]
                                    } else {
                                        vec!["API Keys".into(), "Webhooks".into()]
                                    },
                                    anchor: rect,
                                })
                            }
                            "Solutions" => {
                                self.overlay = Some(Overlay::Menu {
                                    label: "Enterprise".into(),
                                    choices: vec!["Security & Compliance".into()],
                                    anchor: rect,
                                })
                            }
                            _ => self.selected = choice.clone(),
                        }
                    }
                }
            }
            Overlay::Calendar | Overlay::Color => {
                let anchor = self.picker_anchor;
                let rect = r(
                    anchor.x.min(width - anchor.width - 12.0).max(12.0),
                    anchor.y.min(height - anchor.height - 12.0).max(12.0),
                    anchor.width,
                    anchor.height,
                );
                p.shadow(
                    Rect {
                        y: rect.y + 6.0,
                        ..rect
                    },
                    12.0,
                    24.0,
                    if p.palette.dark {
                        [0.0, 0.0, 0.0, 0.50]
                    } else {
                        [0.06, 0.09, 0.18, 0.12]
                    },
                );
                p.box_(rect, p.palette.card, p.palette.border, 12.0);
                if p.input.down && p.input.pointer.is_some_and(|pt| !rect.contains(pt)) {
                    self.overlay = None;
                    return;
                }
                if matches!(overlay, Overlay::Calendar) {
                    self.calendar_picker(p, rect);
                } else {
                    if p.color_editor(
                        "popover-color",
                        r(rect.x + 20.0, rect.y + 18.0, rect.width - 40.0, 344.0),
                        &mut self.color_editor,
                    ) {
                        self.color = self.color_editor.hex.clone();
                    }
                    if p.button(
                        "color-close",
                        r(
                            rect.x + rect.width - 110.0,
                            rect.y + rect.height - 46.0,
                            90.0,
                            32.0,
                        ),
                        "Done",
                        0,
                        false,
                    ) {
                        self.overlay = None;
                    }
                }
            }
            other => {
                p.box_(
                    r(0.0, 0.0, width, height),
                    [0.0, 0.0, 0.0, 0.45],
                    [0.0; 4],
                    0.0,
                );
                let x = (width - 540.0) * 0.5;
                let y = (height - 380.0) * 0.5;
                p.card(r(x, y, 540.0, 380.0));
                if p.button(
                    "overlay-close",
                    r(x + 490.0, y + 12.0, 32.0, 32.0),
                    "×",
                    3,
                    false,
                ) {
                    self.overlay = None;
                    return;
                }
                match other {
                    Overlay::Modal => {
                        p.label(
                            x + 24.0,
                            y + 20.0,
                            440.0,
                            32.0,
                            "Confirm Action",
                            20.0,
                            600,
                            p.palette.fg,
                        );
                        p.label(x+24.0,y+82.0,492.0,68.0,"Are you sure you want to perform this action? This operation will affect the active configuration.",16.0,400,p.palette.fg);
                        p.notification(
                            r(x + 24.0, y + 176.0, 492.0, 68.0),
                            "Warning",
                            "This operation cannot be undone.",
                            YELLOW,
                        );
                        if p.button(
                            "cancel",
                            r(x + 276.0, y + 314.0, 110.0, 38.0),
                            "Cancel",
                            2,
                            false,
                        ) || p.button(
                            "confirm",
                            r(x + 398.0, y + 314.0, 118.0, 38.0),
                            "Confirm",
                            0,
                            false,
                        ) {
                            self.overlay = None;
                        }
                    }
                    Overlay::Survey(step) => self.survey(p, step, r(x, y, 540.0, 380.0)),
                    _ => {}
                }
            }
        }
    }
}
fn r(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}
struct Painter<'a> {
    ui: &'a mut Ui,
    input: &'a Input,
    palette: Palette,
    scroll: f32,
    viewport: Rect,
    glass: f32,
}
impl Painter<'_> {
    fn screen(&self, r: Rect) -> Rect {
        Rect {
            y: r.y - self.scroll,
            ..r
        }
    }
    fn base(&self) -> Style {
        Style {
            fill: self.palette.card,
            stroke: self.palette.border,
            foreground: self.palette.fg,
            accent: BLUE,
            radius: 8.0,
            font_size: 14.0,
            font_weight: 400,
            text_padding: 12.0,
            ..Default::default()
        }
    }
    fn label(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: &str,
        size: f32,
        weight: u16,
        color: Color,
    ) {
        self.ui.style = Style {
            foreground: color,
            font_size: size,
            font_weight: weight,
            text_padding: 0.0,
            ..self.base()
        };
        self.ui.label(self.screen(r(x, y, w, h)), text);
    }
    fn right(&mut self, x: f32, y: f32, w: f32, h: f32, text: &str, size: f32, color: Color) {
        self.label(x, y, w, h, text, size, 600, color);
        self.ui.commands.last_mut().unwrap().style.text_align = TextAlign::Right;
    }
    fn subheading(&mut self, x: f32, y: f32, w: f32, text: &str) {
        self.label(x, y, w, 20.0, text, 14.0, 500, self.palette.muted);
    }
    fn shadow(&mut self, rect: Rect, radius: f32, blur: f32, color: Color) {
        self.ui.shadow_box(self.screen(rect), radius, blur, color);
    }
    fn box_(&mut self, rect: Rect, fill: Color, stroke: Color, radius: f32) {
        self.ui.style = Style {
            fill,
            stroke,
            radius,
            stroke_width: if stroke[3] == 0.0 { 0.0 } else { 1.0 },
            ..self.base()
        };
        self.ui.panel(self.screen(rect));
    }
    fn card(&mut self, rect: Rect) {
        let shadow_color = if self.palette.dark {
            [0.0, 0.0, 0.0, 0.40]
        } else {
            [0.06, 0.09, 0.18, 0.08]
        };
        self.shadow(
            Rect {
                y: rect.y + 8.0,
                ..rect
            },
            20.0,
            24.0,
            shadow_color,
        );
        self.ui.style = Style {
            fill: if self.palette.dark {
                [0.10, 0.10, 0.12, 0.75]
            } else {
                [1.0, 1.0, 1.0, 0.40]
            },
            gradient_end: Some(if self.palette.dark {
                [0.08, 0.12, 0.20, 0.8]
            } else {
                [0.82, 0.88, 1.0, (self.glass * 0.42).min(0.65)]
            }),
            stroke: self.palette.border,
            radius: 20.0,
            material: Material::Glass {
                blur: self.glass * 18.0,
            },
            ..self.base()
        };
        self.ui.panel(self.screen(rect));
    }
    fn interact(&mut self, id: &str, rect: Rect) -> Response {
        self.ui.interact(id, self.screen(rect))
    }
    fn button(&mut self, id: &str, rect: Rect, label: &str, variant: u8, disabled: bool) -> bool {
        let mut style = self.base();
        style.font_weight = 500;
        style.text_align = TextAlign::Center;
        match variant {
            0 => {
                style.fill = BLUE;
                style.stroke = BLUE;
                style.foreground = [1.0; 4];
            }
            1 => {
                style.fill = self.palette.secondary;
                style.stroke = [0.0; 4];
            }
            2 => {
                style.fill = self.palette.card;
            }
            3 => {
                style.fill = [0.0; 4];
                style.stroke = [0.0; 4];
            }
            4 => {
                style.fill = RED;
                style.foreground = [1.0; 4];
                style.stroke = RED;
            }
            5 => {
                style.fill = [BLUE[0], BLUE[1], BLUE[2], 0.15];
                style.stroke = BLUE;
                style.foreground = BLUE;
            }
            _ => {}
        }
        if disabled {
            style.fill[3] *= 0.5;
            style.foreground[3] *= 0.5;
            style.stroke[3] *= 0.5;
        }
        let response = if disabled {
            Response::default()
        } else {
            self.interact(id, rect)
        };
        self.ui.style = style;
        self.ui.paint(
            "button",
            label,
            self.screen(rect),
            0.0,
            response,
            self.ui.focused(id) && !disabled,
        );
        response.clicked
    }
    fn dropdown(&mut self, id: &str, rect: Rect, label: &str) -> bool {
        let clicked = self.button(id, rect, label, 2, false);
        // Position the indicator from the control bounds, independently of text width.
        let right = rect.x + rect.width - 14.0;
        let cy = rect.y + rect.height * 0.5;
        self.line(
            right - 8.0,
            cy - 2.0,
            right - 4.0,
            cy + 2.0,
            self.palette.fg,
            1.3,
        );
        self.line(right - 4.0, cy + 2.0, right, cy - 2.0, self.palette.fg, 1.3);
        clicked
    }
    fn circular(&mut self, id: &str, rect: Rect, label: &str, variant: u8, disabled: bool) {
        self.button(id, rect, label, variant, disabled);
        self.ui.commands.last_mut().unwrap().style.radius = rect.height * 0.5;
    }
    fn node(&mut self, id: &str, rect: Rect, label: &str, selected: bool, shape: usize) -> bool {
        if shape == 3 {
            let response = self.interact(id, rect);
            let cx = rect.x + rect.width * 0.5;
            let cy = rect.y + rect.height * 0.5;
            let top = Point::new(cx, rect.y - 16.0);
            let left = Point::new(rect.x, cy);
            let bottom = Point::new(cx, rect.y + rect.height + 16.0);
            let right = Point::new(rect.x + rect.width, cy);
            self.triangles(
                &[top, left, bottom, top, bottom, right],
                if selected {
                    BLUE
                } else {
                    self.palette.secondary
                },
            );
            self.label(
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                label,
                14.0,
                500,
                self.palette.fg,
            );
            self.ui.commands.last_mut().unwrap().style.text_align = TextAlign::Center;
            return response.clicked;
        }
        let rect = if shape == 1 {
            r(rect.x + 28.0, rect.y - 12.0, 68.0, 68.0)
        } else {
            rect
        };
        let clicked = self.button(id, rect, label, if selected { 5 } else { 2 }, false);
        if shape == 1 || shape == 2 {
            self.ui.commands.last_mut().unwrap().style.radius = rect.height * 0.5;
        }
        clicked
    }
    fn field(
        &mut self,
        id: &str,
        rect: Rect,
        value: &mut String,
        placeholder: &str,
        disabled: bool,
    ) -> bool {
        self.ui.style = self.base();
        if disabled {
            self.ui.paint(
                "text_input",
                value,
                self.screen(rect),
                0.0,
                Response::default(),
                false,
            );
            return false;
        }
        let response = self.ui.text_input(id, self.screen(rect), value);
        if self.ui.commands.last().is_some_and(|c| c.value > 0.5) {
            self.ui.commands.last_mut().unwrap().style.fill = [BLUE[0], BLUE[1], BLUE[2], 0.2];
        }
        if value.is_empty() {
            let cmd = self.ui.commands.last_mut().unwrap();
            cmd.text = placeholder.into();
            cmd.style.foreground = self.palette.muted;
        }
        response.changed
    }
    fn check(
        &mut self,
        id: &str,
        rect: Rect,
        label: &str,
        value: &mut bool,
        disabled: bool,
        switch: bool,
    ) {
        let response = if disabled {
            Response::default()
        } else {
            self.interact(id, rect)
        };
        if response.clicked {
            *value = !*value;
        }
        let mut color = if *value { BLUE } else { self.palette.border };
        if disabled {
            color[3] *= 0.5;
        }
        let h = if switch { 22.0 } else { 18.0 };
        let width = if switch { 40.0 } else { 18.0 };
        let rr = r(rect.x, rect.y + (rect.height - h) * 0.5, width, h);
        self.box_(
            rr,
            if switch || *value {
                color
            } else {
                self.palette.card
            },
            color,
            if switch { 11.0 } else { 4.0 },
        );
        if switch {
            self.box_(
                r(
                    rr.x + if *value { 21.0 } else { 3.0 },
                    rr.y + 3.0,
                    16.0,
                    16.0,
                ),
                [1.0; 4],
                [0.0; 4],
                8.0,
            );
        } else if *value {
            // Geometry avoids font-dependent bearings and baseline offsets.
            self.line(
                rr.x + 4.0,
                rr.y + 9.0,
                rr.x + 7.5,
                rr.y + 12.5,
                [1.0; 4],
                1.8,
            );
            self.line(
                rr.x + 7.5,
                rr.y + 12.5,
                rr.x + 14.0,
                rr.y + 5.5,
                [1.0; 4],
                1.8,
            );
        }
        let mut fg = self.palette.fg;
        if disabled {
            fg[3] *= 0.5;
        }
        self.label(
            rect.x + width + 10.0,
            rect.y,
            rect.width - width - 10.0,
            rect.height,
            label,
            14.0,
            400,
            fg,
        );
    }
    fn slider(
        &mut self,
        id: &str,
        rect: Rect,
        value: &mut f32,
        min: f32,
        max: f32,
        step: f32,
        disabled: bool,
    ) {
        let response = if disabled {
            Response::default()
        } else {
            self.interact(id, rect)
        };
        if response.active {
            if let Some(pt) = self.input.pointer {
                *value =
                    (min + ((pt.x - rect.x) / rect.width).clamp(0.0, 1.0) * (max - min)) / step;
                *value = (*value).round() * step;
            }
        }
        if self.ui.focused(id) && !disabled {
            *value = (*value + self.input.step as f32 * step).clamp(min, max);
        }
        let fraction = (*value - min) / (max - min);
        let cy = rect.y + rect.height * 0.5;
        let mut blue = BLUE;
        if disabled {
            blue[3] = 0.5;
        }
        self.box_(
            r(rect.x, cy - 3.0, rect.width, 6.0),
            self.palette.secondary,
            self.palette.border,
            3.0,
        );
        self.box_(
            r(rect.x, cy - 3.0, rect.width * fraction, 6.0),
            blue,
            [0.0; 4],
            3.0,
        );
        self.box_(
            r(rect.x + rect.width * fraction - 7.0, cy - 7.0, 14.0, 14.0),
            blue,
            [0.0; 4],
            7.0,
        );
    }
    fn swatch(&mut self, id: &str, rect: Rect, color: Color) -> bool {
        let response = self.interact(id, rect);
        self.box_(
            rect,
            color,
            if response.hovered {
                self.palette.fg
            } else {
                self.palette.border
            },
            6.0,
        );
        response.clicked
    }
    fn otp(&mut self, rect: Rect, value: &mut String) {
        self.interact("otp", rect);
        let focused = self.ui.focused("otp");
        if focused {
            if self.input.backspace {
                value.pop();
            }
            value.extend(
                self.input
                    .text
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .take(6 - value.len().min(6)),
            );
        }
        for i in 0..6 {
            let cell = r(rect.x + i as f32 * 50.0, rect.y, 42.0, rect.height);
            self.box_(
                cell,
                self.palette.card,
                if focused && i == value.len().min(5) {
                    BLUE
                } else {
                    self.palette.border
                },
                8.0,
            );
            self.label(
                cell.x,
                cell.y,
                cell.width,
                cell.height,
                &value
                    .chars()
                    .nth(i)
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
                20.0,
                500,
                self.palette.fg,
            );
            self.ui.commands.last_mut().unwrap().style.text_align = TextAlign::Center;
        }
    }
    fn badge(&mut self, rect: Rect, text: &str, color: Color, borderless: bool) {
        self.ui.style = Style {
            fill: [
                color[0],
                color[1],
                color[2],
                if borderless { 0.0 } else { 0.1 },
            ],
            foreground: color,
            stroke: if borderless {
                [0.0; 4]
            } else {
                [color[0], color[1], color[2], 0.2]
            },
            text_align: TextAlign::Center,
            text_padding: 6.0,
            font_size: if borderless { 15.0 } else { 12.0 },
            font_weight: 600,
            radius: 6.0,
            ..self.base()
        };
        self.ui.paint(
            "button",
            text,
            self.screen(rect),
            0.0,
            Response::default(),
            false,
        );
    }
    fn notification(&mut self, rect: Rect, title: &str, body: &str, color: Color) {
        self.box_(
            rect,
            [color[0], color[1], color[2], 0.08],
            [color[0], color[1], color[2], 0.2],
            10.0,
        );
        self.label(
            rect.x + 16.0,
            rect.y + 8.0,
            rect.width - 32.0,
            24.0,
            title,
            14.0,
            600,
            color,
        );
        self.label(
            rect.x + 16.0,
            rect.y + 34.0,
            rect.width - 32.0,
            22.0,
            body,
            13.0,
            400,
            self.palette.fg,
        );
    }
    fn progress(&mut self, rect: Rect, value: f32, color: Color) {
        self.box_(rect, self.palette.secondary, [0.0; 4], 4.0);
        self.box_(
            Rect {
                width: rect.width * value.clamp(0.0, 1.0),
                ..rect
            },
            color,
            [0.0; 4],
            4.0,
        );
    }
    fn triangles(&mut self, points: &[Point], color: Color) {
        if points.is_empty() {
            return;
        }
        let minx = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let miny = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let maxx = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let maxy = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        self.ui.style = Style {
            fill: color,
            material: Material::Custom {
                name: "gallery.triangles".into(),
                parameters: points
                    .iter()
                    .flat_map(|p| [p.x, p.y - self.scroll])
                    .collect(),
            },
            ..self.base()
        };
        self.ui.paint(
            "triangles",
            "",
            self.screen(r(minx, miny, maxx - minx, maxy - miny)),
            0.0,
            Response::default(),
            false,
        );
    }
    fn line(&mut self, x: f32, y: f32, x2: f32, y2: f32, color: Color, width: f32) {
        let len = (x2 - x).hypot(y2 - y);
        if len < 0.001 {
            return;
        }
        let dx = -(y2 - y) / len * width * 0.5;
        let dy = (x2 - x) / len * width * 0.5;
        let a = Point::new(x + dx, y + dy);
        let b = Point::new(x - dx, y - dy);
        let c = Point::new(x2 + dx, y2 + dy);
        let d = Point::new(x2 - dx, y2 - dy);
        self.triangles(&[a, b, c, b, d, c], color);
    }
    fn sparkline(&mut self, rect: Rect, data: &[f32], color: Color) {
        let min = data.iter().copied().fold(f32::INFINITY, f32::min);
        let max = data.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        for i in 1..data.len() {
            let x = rect.x + (i - 1) as f32 / (data.len() - 1) as f32 * rect.width;
            let x2 = rect.x + i as f32 / (data.len() - 1) as f32 * rect.width;
            let y =
                rect.y + rect.height - (data[i - 1] - min) / (max - min).max(0.01) * rect.height;
            let y2 = rect.y + rect.height - (data[i] - min) / (max - min).max(0.01) * rect.height;
            self.line(x, y, x2, y2, color, 1.0);
        }
    }
    fn spinner(&mut self, x: f32, y: f32, size: f32, time: f32, color: Color) {
        for i in 0..16 {
            let a = time * 4.0 + i as f32 / 16.0 * std::f32::consts::TAU;
            let mut c = color;
            c[3] = i as f32 / 16.0;
            self.box_(
                r(
                    x + a.cos() * size * 0.5 - 2.0,
                    y + a.sin() * size * 0.5 - 2.0,
                    4.0,
                    4.0,
                ),
                c,
                [0.0; 4],
                2.0,
            );
        }
    }
    fn pie(&mut self, center: Point, radius: f32, donut: bool, _time: f32) {
        for (start, end, color) in [(0.0, 0.45, BLUE), (0.45, 0.78, GREEN), (0.78, 1.0, YELLOW)] {
            let mut points = Vec::new();
            let n = ((end - start) * 180.0_f32).ceil() as usize;
            for i in 0..n {
                let a = (start + (end - start) * i as f32 / n as f32) * std::f32::consts::TAU;
                let b = (start + (end - start) * (i + 1) as f32 / n as f32) * std::f32::consts::TAU;
                let at = |angle: f32, rad: f32| {
                    Point::new(center.x + angle.cos() * rad, center.y + angle.sin() * rad)
                };
                let outer_a = at(a, radius);
                let outer_b = at(b, radius);
                if donut {
                    let ia = at(a, radius * 0.65);
                    let ib = at(b, radius * 0.65);
                    points.extend_from_slice(&[ia, outer_a, outer_b, ia, outer_b, ib]);
                } else {
                    points.extend_from_slice(&[center, outer_a, outer_b]);
                }
            }
            self.triangles(&points, color);
        }
        if donut {
            self.label(
                center.x - radius * 0.5,
                center.y - 20.0,
                radius,
                40.0,
                "75%",
                28.0,
                600,
                self.palette.fg,
            );
        }
    }
}
fn hsv(h: f32, s: f32, v: f32) -> Color {
    let f = h * 6.0;
    let i = f.floor() as i32;
    let a = v * (1.0 - s);
    let b = v * (1.0 - (f - i as f32) * s);
    let c = v * (1.0 - (1.0 - (f - i as f32)) * s);
    let (r, g, b) = match i % 6 {
        0 => (v, c, a),
        1 => (b, v, a),
        2 => (a, v, c),
        3 => (a, b, v),
        4 => (c, a, v),
        _ => (v, a, b),
    };
    [r, g, b, 1.0]
}
pub fn scale_commands(commands: &mut [DrawCommand], scale: f32) {
    for cmd in commands {
        cmd.rect = Rect {
            x: cmd.rect.x * scale,
            y: cmd.rect.y * scale,
            width: cmd.rect.width * scale,
            height: cmd.rect.height * scale,
        };
        if let Some(clip) = &mut cmd.clip {
            clip.x *= scale;
            clip.y *= scale;
            clip.width *= scale;
            clip.height *= scale;
        }
        cmd.style.radius *= scale;
        cmd.style.font_size *= scale;
        cmd.style.text_padding *= scale;
        cmd.style.stroke_width *= scale;
        if let Material::Custom { name, parameters } = &mut cmd.style.material {
            if name == "gallery.colored-triangles" {
                for vertex in parameters.chunks_exact_mut(6) {
                    vertex[0] *= scale;
                    vertex[1] *= scale;
                }
            } else if name == "gallery.triangles" {
                for v in parameters {
                    *v *= scale;
                }
            }
        }
        if let Material::Glass { blur } = &mut cmd.style.material {
            *blur *= scale;
        }
    }
}
