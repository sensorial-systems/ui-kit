//! Interactive GPU implementation of the existing Dioxus gallery, built from
//! `ui_kit_immediate::components`. The Dioxus example remains an independent
//! DOM/CSS renderer.
#[cfg(test)]
mod tests;
use std::collections::HashMap;
use ui_kit_core::{Point, Rect, RichText, Style, TextAlign, TextFormat, TextRun};
use ui_kit_immediate::components::theme::{rgb, DANGER, INFO, SUCCESS, WARNING};
use ui_kit_immediate::components::{
    self as kit, section_heading, Answer, Badge, BarGroup, Bill, BillItem, Browser, BrowserState,
    Button, ButtonVariant, Card, Checkbox, ColorField, ColorFieldState, ColorPicker,
    DateTimePicker, DateTimePickerState, Dropdown, EditableText, FlowGraph, FlowNode, FormFlow,
    FormFlowEvent, FormFlowState, GraphCamera, GraphEdge, GraphNode, HierarchyGraph,
    HierarchyGraphModel, Label, Menu, MenuItem, MenuState, MetricCard, MetricChart, Modal,
    NetworkGraph, NetworkGraphNode, NodeShape, Notification, OtpInput, PieChart, PieSlice,
    PipelineBoard, PipelineCard, PipelineColumn, PipelineState, ProgressBar, Question,
    QuestionKind, Slider, Sparkline, Spinner, StackedBarChart, StatusVariant, Switch, Table,
    TableCell, TableColumn, TableOptions, TableSort, TextInput, Timeline, TimelineMilestone,
    TimelineProject, TimelineRange, TimelineState, TimelineTask,
};
use ui_kit_immediate::icon::Icon;
use ui_kit_immediate::vector::line;
use ui_kit_immediate::{
    DrawCommand, Flex, Grid, HorizontalAlignment, Input, Insets, Size, Stack, Strip, Ui,
    VerticalAlignment,
};

/// Space before each section's title, on top of the page's gap.
const SECTION_SPACE: f32 = 24.0;
/// Space between a group of controls and the next group's subheading.
const GROUP_SPACE: f32 = 14.0;
/// Space between the graph cards, on top of the page's gap.
const GRAPH_SPACE: f32 = 16.0;
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
pub const LANGUAGES: [&str; 4] = ["Rust", "TypeScript", "Python", "Go"];
const RATIOS: [&str; 5] = ["1:1", "16:9", "9:16", "4:3", "3:4"];
const ROLES: [&str; 3] = ["developer", "designer", "pm"];
const STACKS: [&str; 3] = ["rust", "ts", "python"];

/// A dialog over the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog {
    Modal,
    Survey,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurveyResult {
    pub role: String,
    pub tech_stack: Vec<String>,
    pub feedback: String,
}

pub struct Gallery {
    ui: Ui,
    pub shift: bool,
    pub dark: bool,
    pub scroll: f32,
    scrollbar_grab: Option<f32>,
    pub content_height: f32,
    viewport: f32,
    pub glass: f32,
    pub username: String,
    pub accepted: bool,
    pub notifications: bool,
    pub language: usize,
    pub otp: String,
    pub slider: f32,
    pub date: String,
    pub date_picker: DateTimePickerState,
    pub color: String,
    pub color_field: ColorFieldState,
    pub inline_color: ColorPicker,
    pub loading: bool,
    pub timeout: Option<f32>,
    pub selectable: bool,
    pub aspect: usize,
    pub interactions: u32,
    pub browser: BrowserState,
    pub dialog: Option<Dialog>,
    pub horizontal_menu: MenuState,
    pub vertical_menu: MenuState,
    pub selected: String,
    pub table: TableOptions,
    pub sort: Option<TableSort>,
    pub pipeline: Vec<PipelineCard>,
    pub pipeline_state: PipelineState,
    timeline_zoom: f32,
    pub timeline: Vec<TimelineProject>,
    timeline_state: TimelineState,
    pub tree_edit: bool,
    pub tree: HierarchyGraphModel,
    pub rich: EditableText,
    rich_metrics: ui_kit_wgpu::ShapedRichText,
    network_camera: GraphCamera,
    pub flow_camera: GraphCamera,
    network_selected: String,
    flow_selected: String,
    pub survey: FormFlowState,
    survey_questions: Vec<Question>,
    pub survey_result: Option<SurveyResult>,
    pub section_positions: HashMap<String, f32>,
    time: f32,
}

fn rich_text_sample() -> RichText {
    let mut text = RichText::default();
    for (run, bold, italic) in [
        ("Hello ", false, false),
        ("World", true, false),
        ("! This is a ", false, false),
        ("WYSIWYG", false, true),
        (
            " editor.\nDouble-click this text block to edit formatting.",
            false,
            false,
        ),
    ] {
        text.runs.push(TextRun {
            text: run.into(),
            format: TextFormat {
                bold,
                italic,
                ..Default::default()
            },
        });
    }
    text
}

fn survey_questions() -> Vec<Question> {
    let options = |items: &[&str]| items.iter().map(|s| s.to_string()).collect();
    vec![
        Question::new(
            "Welcome to the Dynamic Form Demo",
            "Experience intuitive step-by-step interactive forms with logical conditional branching.",
            QuestionKind::Info,
        ),
        Question::new(
            "What is your primary role?",
            "We customize your onboarding flow according to your background.",
            QuestionKind::Single(options(&[
                "Software Engineer / Developer",
                "UI/UX Designer",
                "Product Manager",
            ])),
        )
        .required(),
        Question::new(
            "Which languages or frameworks do you use daily?",
            "Select all that apply.",
            QuestionKind::Multiple(options(&[
                "Rust & Dioxus",
                "TypeScript & React",
                "Python & AI Stack",
            ])),
        )
        .when(1, 0),
        Question::new(
            "How can we make Sensorial UI Kit better for you?",
            "Tell us about features, components, or improvements you'd like to see.",
            QuestionKind::Text {
                placeholder: "Type your thoughts and suggestions here...".into(),
            },
        ),
    ]
}

fn timeline_projects() -> Vec<TimelineProject> {
    vec![TimelineProject {
        id: "design-system".into(),
        title: "Design system".into(),
        expanded: true,
        milestones: vec![TimelineMilestone {
            id: "foundation".into(),
            title: "Foundation".into(),
            range: TimelineRange::new(0.07, 0.73),
            progress: 0.6,
            expanded: true,
            tasks: vec![
                TimelineTask::new(
                    "tokens",
                    "Semantic tokens",
                    TimelineRange::new(0.07, 0.58),
                    INFO,
                ),
                TimelineTask::new(
                    "alpha",
                    "Publish alpha",
                    TimelineRange::new(0.70, 0.15),
                    SUCCESS,
                ),
            ],
        }],
        tasks: vec![TimelineTask::new(
            "docs",
            "Component docs",
            TimelineRange::new(0.42, 0.38),
            WARNING,
        )],
    }]
}

fn nav_items() -> Vec<MenuItem> {
    vec![
        MenuItem::new("products", "Products").with_children(vec![
            MenuItem::new("platform", "Platform").with_children(vec![
                MenuItem::new("analytics", "Analytics"),
                MenuItem::new("automation", "Automation"),
            ]),
            MenuItem::new("integrations", "Integrations").with_children(vec![
                MenuItem::new("api-keys", "API Keys"),
                MenuItem::new("webhooks", "Webhooks"),
            ]),
        ]),
        MenuItem::new("solutions", "Solutions").with_children(vec![MenuItem::new(
            "enterprise",
            "Enterprise",
        )
        .with_children(vec![MenuItem::new("security", "Security & Compliance")])]),
        MenuItem::new("pricing", "Pricing"),
        MenuItem::new("docs", "Documentation"),
    ]
}

impl Default for Gallery {
    fn default() -> Self {
        let pipeline = [
            "Research customer needs",
            "Prepare implementation brief",
            "Define semantic tokens",
            "Review component docs",
            "Publish alpha release",
        ]
        .iter()
        .enumerate()
        .map(|(i, title)| PipelineCard::new(format!("card-{i}"), *title, i))
        .collect();
        Self {
            ui: Ui::default(),
            shift: false,
            dark: false,
            scroll: 0.0,
            scrollbar_grab: None,
            content_height: 10000.0,
            viewport: 850.0,
            glass: 1.0,
            username: String::new(),
            accepted: false,
            notifications: true,
            language: 0,
            otp: String::new(),
            slider: 50.0,
            date: "2026-07-16 18:00".into(),
            date_picker: DateTimePickerState::default(),
            color: "#3b82f6".into(),
            color_field: ColorFieldState::default(),
            inline_color: ColorPicker::new("#10b981"),
            loading: false,
            timeout: None,
            selectable: true,
            aspect: 0,
            interactions: 0,
            browser: BrowserState::new(vec![
                "overview.local".into(),
                "components.local".into(),
                "api.local".into(),
            ]),
            dialog: None,
            horizontal_menu: MenuState::default(),
            vertical_menu: MenuState::default(),
            selected: "build".into(),
            table: TableOptions {
                striped: true,
                hoverable: true,
                ..Default::default()
            },
            sort: Some(TableSort {
                column: 1,
                ..Default::default()
            }),
            pipeline,
            pipeline_state: PipelineState::default(),
            timeline_zoom: 1.0,
            timeline: timeline_projects(),
            timeline_state: TimelineState::default(),
            tree_edit: false,
            tree: HierarchyGraphModel::new(&[
                ("Core App", None),
                ("UI Layer", Some(0)),
                ("Database", Some(0)),
                ("GraphQL API", Some(0)),
                ("Views & Pages", Some(1)),
                ("Shared Parts", Some(1)),
                ("Web Views", Some(4)),
                ("Mobile Views", Some(4)),
                ("Form Controls", Some(5)),
                ("Navigation", Some(5)),
            ]),
            rich: EditableText::new(rich_text_sample()),
            rich_metrics: Default::default(),
            network_camera: Default::default(),
            flow_camera: Default::default(),
            network_selected: "db".into(),
            flow_selected: "build".into(),
            survey: FormFlowState::default(),
            survey_questions: survey_questions(),
            survey_result: None,
            section_positions: HashMap::new(),
            time: 0.0,
        }
    }
}

/// A subheading and the row of content under it, `height` tall.
fn group(ui: &mut Ui, rows: &mut Stack, title: &str, height: f32) -> Rect {
    Label::subheading().show(ui, rows.row(20.0), title);
    rows.row(height)
}

impl Gallery {
    pub fn background(&self) -> wgpu::Color {
        let c = kit::theme::palette(self.dark).background;
        wgpu::Color {
            r: c[0] as f64,
            g: c[1] as f64,
            b: c[2] as f64,
            a: 1.0,
        }
    }

    /// Measure text with the renderer's fonts, so content-sized controls fit.
    pub fn set_text_measure(
        &mut self,
        measure: impl Fn(&str, &Style) -> f32 + Send + Sync + 'static,
    ) {
        self.ui.set_text_measure(measure);
    }

    pub fn scroll_by(&mut self, delta: f32) {
        if self.dialog.is_none() {
            self.scroll =
                (self.scroll + delta).clamp(0.0, (self.content_height - self.viewport).max(0.0));
        }
    }

    pub fn dismiss(&mut self) {
        if self.dialog.is_none() && self.rich.editing {
            self.rich.cancel();
        }
        self.dialog = None;
        self.horizontal_menu.close();
        self.vertical_menu.close();
        self.cancel_drag();
    }

    pub fn cancel_drag(&mut self) {
        self.pipeline_state.cancel_drag();
        self.timeline_state.cancel_drag();
    }

    pub fn frame(
        &mut self,
        mut input: Input,
        width: f32,
        height: f32,
        time: f32,
    ) -> Vec<DrawCommand> {
        self.viewport = height;
        self.time = time;
        if self.timeout.is_some_and(|t| time >= t) {
            self.timeout = None;
        }
        let window = Rect::new(0.0, 0.0, width, height);
        input.viewport = Some(input.viewport.unwrap_or(window));
        let scrollbar = window.split_right(14.0, 0.0).1;
        let mut ui = std::mem::take(&mut self.ui);
        ui.theme = kit::theme::theme(self.dark);
        ui.begin(input.clone());
        // Capture before content so a drag cannot activate an underlying control.
        if self.content_height > height && self.dialog.is_none() {
            let thumb = (height * height / self.content_height)
                .max(24.0)
                .min(height);
            let travel = (height - thumb).max(1.0);
            let range = (self.content_height - height).max(0.0);
            let offset = self.scroll / range.max(1.0) * travel;
            let response = ui.interact("gallery-scrollbar", scrollbar);
            match (response.active, input.pointer) {
                (true, Some(pt)) => {
                    let grab = *self.scrollbar_grab.get_or_insert_with(|| {
                        if pt.y >= offset && pt.y <= offset + thumb {
                            pt.y - offset
                        } else {
                            thumb * 0.5
                        }
                    });
                    self.scroll = ((pt.y - grab) / travel).clamp(0.0, 1.0) * range;
                }
                _ => self.scrollbar_grab = None,
            }
        } else {
            self.scrollbar_grab = None;
        }
        ui.clip = Some(window);
        // The page is a column at most 960 wide, kept 20 from the window's
        // edges, laid out where it is on screen.
        let column = Rect::new(0.0, 40.0 - self.scroll, width.max(320.0), 0.0)
            .inset_by(Insets::symmetric(20.0, 0.0))
            .center_horizontally(960.0);
        let mut page = Stack::new(column, 16.0);
        self.header(&mut ui, &mut page);
        self.navigation(&mut ui, &mut page);
        self.browser(&mut ui, &mut page);
        self.buttons(&mut ui, &mut page);
        self.forms(&mut ui, &mut page);
        self.feedback(&mut ui, &mut page);
        self.metrics(&mut ui, &mut page);
        self.table(&mut ui, &mut page);
        self.planning(&mut ui, &mut page);
        self.bills(&mut ui, &mut page);
        self.graphs(&mut ui, &mut page);
        self.content_height = 40.0 + page.height() + 80.0;
        if self.content_height > height {
            let thumb = (height * height / self.content_height).max(24.0);
            let offset = self.scroll / (self.content_height - height).max(1.0) * (height - thumb);
            let track = scrollbar.inset_by(Insets::new(5.0, 0.0, 2.0, 0.0));
            let border = ui.theme.palette.border;
            kit::panel(
                &mut ui,
                Rect {
                    y: offset,
                    height: thumb,
                    ..track
                },
                border,
                [0.0; 4],
                4.0,
            );
        }
        self.dialog(&mut ui, window);
        let commands = ui.end().to_vec();
        self.ui = ui;
        commands
    }

    fn section(&mut self, ui: &mut Ui, page: &mut Stack, title: &str) {
        page.space(SECTION_SPACE);
        let row = page.row(24.0);
        self.section_positions
            .insert(title.into(), row.y + self.scroll);
        section_heading(ui, row, title);
    }

    fn card(&self, ui: &mut Ui, rect: Rect) {
        Card::new().glass(self.glass).show(ui, rect);
    }

    fn header(&mut self, ui: &mut Ui, page: &mut Stack) {
        // A wide page puts the theme controls beside the title, a narrow one under it.
        let wide = page.width() >= 920.0;
        let header = page.row(if wide { 84.0 } else { 168.0 });
        let (intro, controls) = if wide {
            header.split_right(460.0, 0.0)
        } else {
            header.split_top(84.0, 0.0)
        };
        let mut intro = Strip::column(intro.split_left(480.0, 0.0).0, 8.0);
        Label::heading(28.0)
            .weight(700)
            .show(ui, intro.leading(34.0), "Dioxus Component Gallery");
        Label::muted().show(
            ui,
            intro.leading(22.0),
            "A premium collection of reusable and highly customizable components.",
        );
        let [theme, glass] = Flex::row(20.0).split(controls, [Size::Fixed(220.0); 2]);
        let mut theme = Strip::column(theme, 6.0);
        Label::new()
            .size(13.0)
            .weight(500)
            .show(ui, theme.leading(20.0), "Select Theme");
        let mut dark = usize::from(self.dark);
        if Dropdown::new(&["White", "Black"])
            .show(ui, "theme", theme.leading(38.0), &mut dark)
            .changed
        {
            self.dark = dark == 1;
        }
        let (caption, slider) = glass.split_top(32.0, 0.0);
        let [caption, value] = Flex::row(4.0).split(
            caption.split_bottom(20.0, 0.0).1,
            [Size::FILL, Size::Fixed(56.0)],
        );
        Label::new()
            .size(13.0)
            .weight(500)
            .show(ui, caption, "Glass Strength");
        Label::muted()
            .size(13.0)
            .weight(600)
            .align(TextAlign::Right)
            .show(ui, value, &format!("{}%", (self.glass * 100.0) as i32));
        Slider::new(0.0, 2.0).step(0.05).show(
            ui,
            "glass",
            slider.split_top(38.0, 0.0).0,
            &mut self.glass,
        );
        let border = ui.theme.palette.border;
        line(
            ui,
            Point::new(header.x, header.bottom()),
            Point::new(header.right(), header.bottom()),
            1.0,
            border,
        );
    }

    fn navigation(&mut self, ui: &mut Ui, page: &mut Stack) {
        self.section(ui, page, "Navigation");
        let card = page.row(332.0);
        self.card(ui, card);
        let items = nav_items();
        let mut rows = Stack::new(card.inset(20.0), 12.0);
        Label::subheading().show(ui, rows.row(20.0), "Horizontal Menu");
        let widths = [112.0, 116.0, 76.0, 120.0];
        let menu = Menu::horizontal(&items).widths(&widths);
        let bar = rows.row(38.0).split_left(menu.extent(ui), 0.0).0;
        if let Some(id) = menu.show(ui, "nav", bar, &mut self.horizontal_menu) {
            self.selected = id;
        }
        rows.space(6.0);
        Label::subheading().show(ui, rows.row(20.0), "Vertical Menu");
        let menu = Menu::vertical(&items);
        let list = rows.row(menu.extent(ui)).split_left(240.0, 0.0).0;
        if let Some(id) = menu.show(ui, "vertical", list, &mut self.vertical_menu) {
            self.selected = id;
        }
    }

    fn browser(&mut self, ui: &mut Ui, page: &mut Stack) {
        self.section(ui, page, "Browser Containers");
        Label::muted().show(ui, page.row(22.0), "Presentation-only browser chrome and controlled tabs. The caller supplies addresses, panels, state, and behavior.");
        let card = page.row(430.0);
        self.card(ui, card);
        let (heading, browser) = card
            .inset_by(Insets::new(20.0, 16.0, 20.0, 26.0))
            .split_top(30.0, 14.0);
        Label::heading(18.0).show(
            ui,
            heading.split_left(400.0, 0.0).0,
            "Browser + TabbedContainer",
        );
        Badge::new(StatusVariant::Info).show(
            ui,
            heading.align(
                135.0,
                26.0,
                HorizontalAlignment::Right,
                VerticalAlignment::Bottom,
            ),
            &format!("{} interactions", self.interactions),
        );
        let (events, content) = Browser.show(ui, "browser", browser, &mut self.browser);
        self.interactions += events.len() as u32;
        let mut text = Strip::column(content.inset_by(Insets::new(16.0, 74.0, 16.0, 0.0)), 10.0);
        let current = match self.browser.current() {
            "" => "Open a page",
            page => page,
        };
        Label::heading(32.0)
            .weight(700)
            .align(TextAlign::Center)
            .show(ui, text.leading(44.0), current);
        Label::muted().align(TextAlign::Center).show(
            ui,
            text.leading(48.0),
            if self.browser.address.is_empty() {
                "Type an address above and press Enter."
            } else {
                "The application can load this address into any renderer it owns."
            },
        );
    }

    fn buttons(&mut self, ui: &mut Ui, page: &mut Stack) {
        self.section(ui, page, "1. Buttons");
        let card = page.row(572.0);
        self.card(ui, card);
        let mut rows = Stack::new(card.inset(20.0), 10.0);
        let mut row = Strip::row(group(ui, &mut rows, "Variants", 38.0), 12.0);
        for (i, (label, width, variant)) in [
            ("Primary", 98.0, ButtonVariant::Primary),
            ("Secondary", 116.0, ButtonVariant::Secondary),
            ("Outline", 94.0, ButtonVariant::Outline),
            ("Text Button", 126.0, ButtonVariant::Text),
        ]
        .into_iter()
        .enumerate()
        {
            Button::new(label).variant(variant).show(
                ui,
                &format!("variant-{i}"),
                row.leading(width),
            );
        }
        rows.space(GROUP_SPACE);
        let mut row = Strip::row(group(ui, &mut rows, "Sizes", 46.0), 12.0);
        for (i, (label, h, w, size)) in [
            ("Small", 30.0, 76.0, 13.0),
            ("Medium", 38.0, 102.0, 14.0),
            ("Large", 46.0, 110.0, 16.0),
        ]
        .into_iter()
        .enumerate()
        {
            let rect = row.leading(110.0).align(
                w,
                h,
                HorizontalAlignment::Left,
                VerticalAlignment::Center,
            );
            Button::new(label)
                .font_size(size)
                .show(ui, &format!("size-{i}"), rect);
        }
        rows.space(GROUP_SPACE);
        let mut row = Strip::row(group(ui, &mut rows, "States", 38.0), 12.0);
        Button::new("Disabled")
            .disabled(true)
            .show(ui, "disabled", row.leading(106.0));
        Button::new("Loading")
            .icon(Icon::Loader)
            .disabled(true)
            .show(ui, "loading", row.leading(108.0));
        let loading = if self.loading {
            Button::new("Loading").icon(Icon::Loader)
        } else {
            Button::new("Click to Load")
        };
        if loading
            .disabled(self.loading)
            .show(ui, "start-loading", row.leading(144.0))
            .clicked
        {
            self.loading = true;
        }
        // Reset only shows while loading, but keeps its place either way.
        let reset = row.leading(78.0);
        if self.loading
            && Button::new("Reset")
                .variant(ButtonVariant::Text)
                .show(ui, "reset-loading", reset)
                .clicked
        {
            self.loading = false;
        }
        row.skip_leading(6.0);
        let timeout = if self.timeout.is_some() {
            Button::new("Loading").icon(Icon::Loader)
        } else {
            Button::new("Load (3s Timeout)")
        };
        if timeout
            .disabled(self.timeout.is_some())
            .show(ui, "timeout", row.leading(186.0))
            .clicked
        {
            self.timeout = Some(self.time + 3.0);
        }
        rows.space(GROUP_SPACE);
        let mut row = Strip::row(group(ui, &mut rows, "Circular Buttons", 46.0), 12.0);
        for (i, (content, size, variant, disabled)) in [
            (Err("S"), 30.0, ButtonVariant::Primary, false),
            (Err("M"), 38.0, ButtonVariant::Primary, false),
            (Err("L"), 46.0, ButtonVariant::Primary, false),
            (Ok(Icon::Plus), 38.0, ButtonVariant::Secondary, false),
            (Ok(Icon::Check), 38.0, ButtonVariant::Outline, false),
            (Ok(Icon::ChevronRight), 38.0, ButtonVariant::Text, false),
            (Err("D"), 38.0, ButtonVariant::Primary, true),
            (Ok(Icon::Loader), 38.0, ButtonVariant::Primary, true),
            (Ok(Icon::Close), 38.0, ButtonVariant::Danger, false),
        ]
        .into_iter()
        .enumerate()
        {
            let rect = row.leading(46.0).align(
                size,
                size,
                HorizontalAlignment::Left,
                VerticalAlignment::Center,
            );
            let button = match content {
                Ok(icon) => Button::icon_only(icon),
                Err(label) => Button::new(label),
            };
            button
                .variant(variant)
                .circular()
                .padding(0.0)
                .disabled(disabled)
                .show(ui, &format!("circle-{i}"), rect);
        }
        rows.space(GROUP_SPACE);
        let mut row = Strip::row(
            group(ui, &mut rows, "Selectable Buttons (Tags / Filters)", 38.0),
            12.0,
        );
        let tag = |selected: bool, label: &'static str| {
            let button = Button::new(label);
            if selected {
                button.icon(Icon::Check).variant(ButtonVariant::Selected)
            } else {
                button.variant(ButtonVariant::Secondary)
            }
        };
        if tag(self.selectable, "Selectable")
            .show(ui, "selectable", row.leading(130.0))
            .clicked
        {
            self.selectable = !self.selectable;
        }
        tag(false, "Not selected").show(ui, "not-selected", row.leading(136.0));
        tag(true, "Selected").show(ui, "selected", row.leading(130.0));
        tag(false, "Disabled")
            .disabled(true)
            .show(ui, "tag-disabled", row.leading(100.0));
        rows.space(GROUP_SPACE);
        let mut row = Strip::row(group(ui, &mut rows, "Aspect Ratio Selector", 38.0), 16.0);
        Dropdown::new(&RATIOS).show(ui, "aspect", row.leading(150.0), &mut self.aspect);
        Label::muted().size(13.0).show(
            ui,
            row.leading(238.0),
            &format!("{} aspect ratio", RATIOS[self.aspect]),
        );
        let mut disabled = 2;
        Dropdown::new(&RATIOS).disabled(true).show(
            ui,
            "aspect-disabled",
            row.leading(150.0),
            &mut disabled,
        );
    }

    fn forms(&mut self, ui: &mut Ui, page: &mut Stack) {
        self.section(ui, page, "2. Form Controls");
        let card = page.row(1330.0 + self.rich.height());
        self.card(ui, card);
        let mut rows = Stack::new(card.inset(20.0), 8.0);
        let field = Flex::row(0.0);
        let columns = [Size::Fixed(140.0), Size::Fixed(360.0)];
        let caption = Label::new().size(13.0).weight(500);
        // The username's hint sits right under its field.
        let (username, hint) = rows.row(62.0).split_top(38.0, 2.0);
        let [name, control] = field.split(username, columns);
        caption.show(ui, name, "Username");
        TextInput::new("Enter username...").show(ui, "username", control, &mut self.username);
        let [_, hint] = field.split(hint, columns);
        let short = !self.username.is_empty() && self.username.chars().count() < 3;
        let hint_label = if short {
            Label::new().color(DANGER)
        } else {
            Label::muted()
        };
        hint_label
            .size(12.0)
            .show(ui, hint, "Must be at least 3 characters long.");
        let [name, control] = field.split(rows.row(38.0), columns);
        caption.show(ui, name, "Email");
        let mut email = "user@example.com".to_string();
        TextInput::new("").show(ui, "email", control, &mut email);
        rows.space(16.0);
        let [name, control] = field.split(rows.row(38.0), columns);
        caption.show(ui, name, "Preferred Language");
        Dropdown::new(&LANGUAGES).show(ui, "language", control, &mut self.language);
        rows.space(16.0);
        let mut row = Strip::row(rows.row(32.0), 24.0);
        Checkbox::new().show(
            ui,
            "terms",
            row.leading(300.0),
            "Accept terms and conditions",
            &mut self.accepted,
        );
        let mut yes = true;
        Checkbox::new().disabled(true).show(
            ui,
            "terms-disabled",
            row.leading(320.0),
            "Disabled Checkbox (Checked)",
            &mut yes,
        );
        rows.space(GROUP_SPACE);
        let mut row = Strip::row(rows.row(32.0), 24.0);
        Switch::new().show(
            ui,
            "notifications",
            row.leading(280.0),
            "Enable notifications",
            &mut self.notifications,
        );
        let mut no = false;
        Switch::new().disabled(true).show(
            ui,
            "switch-disabled",
            row.leading(260.0),
            "Disabled Switch",
            &mut no,
        );
        rows.space(GROUP_SPACE);
        // The rest of the form is 500 wide.
        let divider = rows.row(0.0).split_left(500.0, 0.0).0;
        let border = ui.theme.palette.border;
        line(
            ui,
            Point::new(divider.x, divider.y),
            Point::new(divider.right(), divider.y),
            1.0,
            border,
        );
        rows.space(8.0);
        let otp = OtpInput::new(6);
        let (code, value) = group(ui, &mut rows, "One-Time Password (OTP)", 70.0)
            .split_left(500.0, 0.0)
            .0
            .split_top(44.0, 4.0);
        otp.show(
            ui,
            "otp",
            code.split_left(otp.width(), 0.0).0,
            &mut self.otp,
        );
        Label::muted().size(12.0).show(
            ui,
            value,
            &format!("Current value in parent state: '{}'", self.otp),
        );
        rows.space(GROUP_SPACE);
        let title = format!("Slider (Value: {:.1})", self.slider);
        let slider = group(ui, &mut rows, &title, 24.0).split_left(500.0, 0.0).0;
        Slider::new(0.0, 100.0)
            .step(0.5)
            .show(ui, "form-slider", slider, &mut self.slider);
        rows.space(GROUP_SPACE);
        let slider = group(ui, &mut rows, "Disabled Slider", 24.0)
            .split_left(500.0, 0.0)
            .0;
        let mut value = 30.0;
        Slider::new(0.0, 100.0).step(1.0).disabled(true).show(
            ui,
            "slider-disabled",
            slider,
            &mut value,
        );
        rows.space(24.0);
        let date = group(ui, &mut rows, "Appointment Date & Time", 38.0)
            .split_left(500.0, 0.0)
            .0;
        DateTimePicker::new().show(ui, "date", date, &mut self.date, &mut self.date_picker);
        rows.space(GROUP_SPACE);
        let date = group(ui, &mut rows, "Disabled Date & Time Picker", 38.0)
            .split_left(500.0, 0.0)
            .0;
        let mut fixed = "2026-07-16 14:00".to_string();
        DateTimePicker::new().disabled(true).show(
            ui,
            "date-disabled",
            date,
            &mut fixed,
            &mut DateTimePickerState::default(),
        );
        rows.space(GROUP_SPACE);
        let color = group(ui, &mut rows, "Accent Color Picker (Popover)", 38.0)
            .split_left(260.0, 0.0)
            .0;
        ColorField.show(ui, "color", color, &mut self.color, &mut self.color_field);
        rows.space(GROUP_SPACE);
        let picker = group(ui, &mut rows, "Inline Color Picker", ColorPicker::HEIGHT)
            .split_left(320.0, 0.0)
            .0;
        self.inline_color.show(ui, "inline", picker);
        rows.space(GROUP_SPACE);
        let editor = group(ui, &mut rows, "Editable Text / WYSIWYG", self.rich.height())
            .inset_by(Insets::new(0.0, 0.0, 40.0, 0.0));
        self.rich
            .show(ui, "rich", editor, &mut self.rich_metrics, self.time);
    }

    fn feedback(&mut self, ui: &mut Ui, page: &mut Stack) {
        self.section(ui, page, "3. Feedback & Badges");
        let card = page.row(620.0);
        self.card(ui, card);
        let mut rows = Stack::new(card.inset(20.0), 12.0);
        Label::subheading().show(ui, rows.row(20.0), "Badges (Variants, Sizes & Styles)");
        for (caption, caption_width, height, large) in [
            ("Normal:", 70.0, 26.0, false),
            ("Large & Borderless (Metrics Style):", 255.0, 32.0, true),
        ] {
            let mut row = Strip::row(rows.row(36.0), 10.0);
            Label::muted().size(13.0).show(
                ui,
                row.leading(caption_width).split_top(30.0, 0.0).0,
                caption,
            );
            for (label, variant) in [
                ("Default", StatusVariant::Default),
                ("Success", StatusVariant::Success),
                ("Warning", StatusVariant::Warning),
                ("Error", StatusVariant::Error),
                ("Info", StatusVariant::Info),
            ] {
                let badge = Badge::new(variant);
                let badge = if large { badge.large() } else { badge };
                badge.show(ui, row.leading(88.0).split_top(height, 0.0).0, label);
            }
        }
        Label::subheading().show(ui, rows.row(20.0), "Notifications");
        for (title, body, variant) in [
            (
                "System Update",
                "A new software update is available. Please upgrade.",
                StatusVariant::Info,
            ),
            (
                "Operation Successful",
                "Your settings have been saved correctly.",
                StatusVariant::Success,
            ),
            (
                "Low Disk Space",
                "Your storage is almost full. Clean some space.",
                StatusVariant::Warning,
            ),
            (
                "Connection Failed",
                "Unable to connect to the database. Please try again.",
                StatusVariant::Error,
            ),
        ] {
            Notification::new(variant).show(ui, rows.row(Notification::HEIGHT), title, body);
        }
        Label::subheading().show(ui, rows.row(20.0), "Spinners (Sizes & Variants)");
        let mut row = Strip::row(rows.row(32.0), 11.0);
        for (size, label) in [(16.0, "Small"), (24.0, "Medium"), (32.0, "Large")] {
            let (spinner, caption) = row.leading(144.0).split_left(32.0, 12.0);
            Spinner::new(size).show(ui, spinner, self.time);
            Label::muted().size(13.0).show(ui, caption, label);
        }

        self.section(ui, page, "4. Modals & Dynamic Form Flow Engine");
        let card = page.row(128.0);
        self.card(ui, card);
        let mut rows = Stack::new(card.inset_by(Insets::new(20.0, 16.0, 20.0, 20.0)), 10.0);
        Label::muted().show(ui, rows.row(44.0), "Explore modal dialogs as well as dynamic, conditional step-by-step survey flows with real-time branching logic and validation.");
        let mut row = Strip::row(rows.row(38.0), 12.0);
        if Button::new("Open Standard Modal")
            .show(ui, "modal", row.leading(210.0))
            .clicked
        {
            self.dialog = Some(Dialog::Modal);
        }
        if Button::new("Launch Dynamic Form Demo")
            .variant(ButtonVariant::Outline)
            .show(ui, "survey", row.leading(250.0))
            .clicked
        {
            self.survey.reset();
            self.dialog = Some(Dialog::Survey);
        }
    }

    fn metrics(&mut self, ui: &mut Ui, page: &mut Stack) {
        self.section(ui, page, "5. Metric & Data Visualization");
        let card = page.row(680.0);
        self.card(ui, card);
        let inner = card.inset(20.0);
        let mut rows = Stack::new(inner, 12.0);
        Label::subheading().show(ui, rows.row(20.0), "Metric Cards (Directly using Card)");
        rows.space(8.0);
        let time = self.time;
        let values = [
            format!("{} reqs", 1234 + (time as u32) * 10),
            format!("{:.1} %", 75.4 + time.sin() * 4.0),
            "Healthy".into(),
            format!("{:.1} GB", 4.2 + time.sin() * 0.2),
        ];
        let memory = [1.2, 1.5, 2.0, 1.8, 2.4, 3.1, 2.8, 3.5, 4.2];
        let cards = [
            MetricCard {
                label: "Requests Count",
                value: &values[0],
                ..Default::default()
            },
            MetricCard {
                label: "CPU Usage",
                value: &values[1],
                chart: MetricChart::Progress(0.754),
                ..Default::default()
            },
            MetricCard {
                label: "System Status",
                value: &values[2],
                color: Some(SUCCESS),
                ..Default::default()
            },
            MetricCard {
                label: "Memory Load",
                value: &values[3],
                chart: MetricChart::Sparkline(&memory),
                ..Default::default()
            },
        ];
        let columns = if page.width() >= 750.0 { 3 } else { 2 };
        let grid =
            |area: Rect| Grid::columns(area, columns, 20.0).with_cell_height(MetricCard::HEIGHT);
        let tiles = grid(rows.row(grid(inner).height(cards.len())));
        for (i, metric) in cards.iter().enumerate() {
            metric.show(ui, tiles.cell(i), self.glass);
        }
        rows.space(30.0);
        let progress = group(ui, &mut rows, "Progress Bars", 72.0)
            .split_left(400.0, 0.0)
            .0;
        let mut bars = Stack::new(progress, 30.0);
        ProgressBar::new().show(ui, bars.row(8.0), 0.3);
        let (bar, captions) = bars.row(34.0).split_top(8.0, 6.0);
        ProgressBar::new().show(ui, bar, 0.65);
        let [start, goal] = Flex::row(0.0).split(captions, [Size::FILL; 2]);
        Label::muted().size(12.0).show(ui, start, "Start");
        Label::muted()
            .size(12.0)
            .align(TextAlign::Right)
            .show(ui, goal, "Goal");
        rows.space(10.0);
        let [first, second] = Flex::row(24.0).split(
            group(ui, &mut rows, "Sparklines (Trend Lines)", 55.0),
            [Size::FILL; 2],
        );
        Sparkline::new().show(ui, first, &[10.0, 15.0, 8.0, 25.0, 18.0, 30.0]);
        Sparkline::new().color(SUCCESS).show(
            ui,
            second,
            &[30.0, 25.0, 40.0, 35.0, 50.0, 45.0, 60.0],
        );
        rows.space(4.0);
        let mut row = Strip::row(group(ui, &mut rows, "Units (Reusable Values)", 34.0), 10.0);
        for (v, size) in [("1,245 reqs", 14.0), ("75.4 %", 20.0), ("4.2 GB", 28.0)] {
            Label::new()
                .size(size)
                .weight(500)
                .show(ui, row.leading(200.0), v);
        }
    }

    fn table(&mut self, ui: &mut Ui, page: &mut Stack) {
        self.section(ui, page, "6. Table Component");
        let card = page.row(394.0);
        self.card(ui, card);
        let (options, area) = card.inset(20.0).split_top(28.0, 20.0);
        let mut options = Strip::row(options, 4.0);
        let flags = [
            ("Striped", &mut self.table.striped),
            ("Hoverable", &mut self.table.hoverable),
            ("Compact", &mut self.table.compact),
            ("Borderless", &mut self.table.borderless),
            ("Loading", &mut self.table.loading),
        ];
        for (i, (label, value)) in flags.into_iter().enumerate() {
            Checkbox::new().show(
                ui,
                &format!("table-option-{i}"),
                options.leading(156.0),
                label,
                value,
            );
        }
        let columns = [
            TableColumn::new("ID", 0.08),
            TableColumn::new("Name", 0.24),
            TableColumn::new("Email", 0.32),
            TableColumn::new("Role", 0.18),
            TableColumn::new("Status", 0.18),
        ];
        let rows: Vec<Vec<TableCell>> = [
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
        ]
        .iter()
        .map(|[id, name, email, role, status]| {
            vec![
                TableCell::Strong(id.to_string()),
                TableCell::text(*name),
                TableCell::Muted(email.to_string()),
                TableCell::Badge {
                    text: role.to_string(),
                    color: None,
                    variant: StatusVariant::Info,
                    large: true,
                },
                TableCell::badge(
                    *status,
                    if *status == "Active" {
                        StatusVariant::Success
                    } else {
                        StatusVariant::Default
                    },
                ),
            ]
        })
        .collect();
        Table::new(&columns, &rows).options(self.table).show(
            ui,
            "table",
            area,
            &mut self.sort,
            self.time,
        );
    }

    fn planning(&mut self, ui: &mut Ui, page: &mut Stack) {
        self.section(ui, page, "7. Planning Components");
        Label::muted().show(ui, page.row(34.0), "Reusable workflow and scheduling views. Drag pipeline cards between columns; use Shift-click for multi-selection.");
        let columns = [
            PipelineColumn::new("backlog", "Backlog"),
            PipelineColumn::new("planned", "Planned"),
            PipelineColumn::new("progress", "In progress"),
            PipelineColumn::new("review", "Review"),
            PipelineColumn::new("done", "Done"),
        ];
        let board = PipelineBoard::new(&columns);
        let card = page.row(20.0 + 26.0 + 20.0 + board.height(&self.pipeline) + 20.0);
        self.card(ui, card);
        let (title, area) = card.inset(20.0).split_top(26.0, 20.0);
        Label::heading(18.0).show(ui, title, "PipelineBoard");
        let response = board.show(
            ui,
            "pipeline",
            area,
            &mut self.pipeline,
            &mut self.pipeline_state,
            self.shift,
        );
        if let Some(card) = response.pressed {
            self.selected = self.pipeline[card].title.clone();
        }

        page.space(8.0);
        const DAYS: [&str; 6] = ["Aug 11", "Aug 12", "Aug 13", "Aug 14", "Aug 15", "Aug 16"];
        let timeline = Timeline::new(&DAYS).zoom(self.timeline_zoom);
        let card = page.row(66.0 + timeline.height(&self.timeline) + 32.0);
        self.card(ui, card);
        let mut header = Strip::row(
            card.inset_by(Insets::new(20.0, 16.0, 20.0, 0.0))
                .split_top(32.0, 0.0)
                .0,
            12.0,
        );
        if Button::new("Zoom")
            .icon(Icon::Plus)
            .variant(ButtonVariant::Secondary)
            .show(ui, "zoom-in", header.trailing(98.0))
            .clicked
        {
            self.timeline_zoom = (self.timeline_zoom * 1.16).min(1.6);
        }
        if Button::new("Zoom")
            .icon(Icon::Minus)
            .variant(ButtonVariant::Secondary)
            .show(ui, "zoom-out", header.trailing(96.0))
            .clicked
        {
            self.timeline_zoom = (self.timeline_zoom / 1.16).max(0.6);
        }
        Label::heading(18.0).show(ui, header.rest(), "Timeline");
        timeline.show(
            ui,
            "timeline",
            card.inset_by(Insets::new(20.0, 66.0, 20.0, 32.0)),
            &mut self.timeline,
            &mut self.timeline_state,
        );
    }

    fn bills(&mut self, ui: &mut Ui, page: &mut Stack) {
        self.section(ui, page, "8. Bill Component");
        Label::muted().show(ui, page.row(36.0), "Streamlined, display-only bill component displaying item breakdown, quantity, unit cost, and aligned total sum.");
        let card = page.row(550.0);
        self.card(ui, card);
        let active = [
            BillItem::new(
                "Cloud Server Cluster (Monthly - 8 Nodes)",
                "8 x $125.00",
                "$1,000.00",
            ),
            BillItem::new(
                "UI Kit Enterprise License & Support",
                "1 x $500.00, less $50.00",
                "$450.00",
            ),
            BillItem::new(
                "Promotional Refund / Service Credit",
                "1 x -$150.00",
                "-$150.00",
            ),
            BillItem::new("Included Tier Support", "1 x $0.00", "$0.00"),
        ];
        let secondary = [
            BillItem::new(
                "Design System Audit & Review",
                "1 x 0.004251 SOL",
                "0.004251 SOL",
            ),
            BillItem::new(
                "Custom Theme Token Preset",
                "2 x 0.000125 SOL",
                "0.000250 SOL",
            ),
        ];
        let bills = [
            Bill {
                kicker: "ACTIVE BILL WIDGET",
                number: "INV-2026-8801",
                status: ("Pending", StatusVariant::Warning),
                items: &active,
                total: "$1,420.00",
                note: "Payment is due within 15 days of invoice date.",
            },
            Bill {
                kicker: "SECONDARY INVOICE SUMMARY",
                number: "INV-2026-9042",
                status: ("Pending", StatusVariant::Warning),
                items: &secondary,
                total: "0.004501 SOL",
                note: "Approximately 0.342976 USDC",
            },
        ];
        // Two invoices side by side, each at most 380 wide.
        let halves = card
            .inset_by(Insets::new(20.0, 20.0, 20.0, 30.0))
            .columns(2, 0.0);
        for (bill, half) in bills.iter().zip(halves) {
            let area = half
                .inset_by(Insets::new(0.0, 0.0, 12.0, 0.0))
                .split_left(380.0, 0.0)
                .0;
            bill.show(ui, area);
        }
    }

    fn graphs(&mut self, ui: &mut Ui, page: &mut Stack) {
        let time = self.time;
        self.section(ui, page, "9. Graph Components");
        Label::muted().show(ui, page.row(44.0), "Interactive graph visualization components utilizing custom CSS grids, animated SVGs, and level-based auto-layout. Click nodes to select/focus.");

        let muted = ui.theme.palette.muted;
        let card = page.row(410.0);
        let area = self.graph_card(ui, card, "Flow Graph (CI/CD Pipeline)", None);
        let nodes = [
            FlowNode::new(
                "checkout",
                "1. Checkout",
                "Source Code",
                Point::new(90.0, 160.0),
                SUCCESS,
            ),
            FlowNode::new(
                "lint",
                "2a. Lint & Format",
                "Cargo clippy",
                Point::new(320.0, 80.0),
                SUCCESS,
            ),
            FlowNode::new(
                "test",
                "2b. Unit Tests",
                "Cargo test",
                Point::new(320.0, 240.0),
                SUCCESS,
            ),
            FlowNode::new(
                "build",
                "3. Build Artifact",
                "Cargo build --release",
                Point::new(550.0, 160.0),
                INFO,
            ),
            FlowNode::new(
                "deploy",
                "4. Deploy",
                "Deploy to AWS",
                Point::new(780.0, 160.0),
                muted,
            ),
        ];
        let edges = [
            GraphEdge::new(0, 1, SUCCESS).animated(),
            GraphEdge::new(0, 2, SUCCESS).animated(),
            GraphEdge::new(1, 3, INFO).label("Passed"),
            GraphEdge::new(2, 3, INFO).label("Passed"),
            GraphEdge::new(3, 4, INFO),
        ];
        self.flow_camera.controls(ui, "flow", controls(card));
        FlowGraph::new(&nodes, &edges, 900.0, 320.0).show(
            ui,
            "flow",
            area,
            &mut self.flow_camera,
            &mut self.flow_selected,
            time,
        );
        page.space(GRAPH_SPACE);

        let card = page.row(460.0);
        self.card(ui, card);
        let mut column = Strip::column(card.inset_by(Insets::new(20.0, 18.0, 20.0, 24.0)), 18.0);
        let mut header = Strip::row(column.leading(30.0), 40.0);
        Checkbox::new().show(
            ui,
            "tree-edit",
            header.trailing(110.0),
            "Editable",
            &mut self.tree_edit,
        );
        Label::heading(18.0).show(
            ui,
            header.rest(),
            "Hierarchy Graph (Mind Map / Org Structure)",
        );
        HierarchyGraph::new().editable(self.tree_edit).show(
            ui,
            "tree",
            column.rest(),
            &mut self.tree,
        );
        page.space(GRAPH_SPACE);

        let card = page.row(528.0);
        let area = self.graph_card(
            ui,
            card,
            "Network Graph (Mesh Topology)",
            Some("Hub-and-spoke automatic circular projection with direct straight mesh links."),
        );
        let nodes = [
            NetworkGraphNode::new("gateway", "Gateway", INFO),
            NetworkGraphNode::new("auth", "Auth Svc", SUCCESS),
            NetworkGraphNode::new("payment", "Billing", DANGER),
            NetworkGraphNode::new("db", "Shared DB", WARNING),
            NetworkGraphNode::new("cache", "Redis", INFO),
        ];
        let edges = [
            GraphEdge::new(0, 1, INFO).animated().label("Active"),
            GraphEdge::new(0, 2, DANGER),
            GraphEdge::new(1, 3, SUCCESS),
            GraphEdge::new(2, 3, WARNING),
            GraphEdge::new(3, 4, INFO).animated().label("Sync"),
            GraphEdge::new(0, 4, muted),
        ];
        self.network_camera.controls(ui, "network", controls(card));
        NetworkGraph::new(&nodes, &edges).show(
            ui,
            "network",
            area,
            &mut self.network_camera,
            &mut self.network_selected,
            time,
        );
        page.space(GRAPH_SPACE);

        let card = page.row(370.0);
        self.card(ui, card);
        Label::heading(18.0).show(
            ui,
            card.inset_by(Insets::new(20.0, 18.0, 20.0, 0.0))
                .split_top(30.0, 0.0)
                .0,
            "Node & Edge Shapes Playground",
        );
        // Nodes sit at fractions of the stage, by their top-left corner.
        let stage = card
            .inset_by(Insets::new(28.0, 70.0, 142.0, 0.0))
            .split_top(220.0, 0.0)
            .0;
        let shapes: Vec<(Rect, &str, NodeShape)> = [
            (0.04, 0.35, "Box", NodeShape::Box),
            (0.28, 0.65, "Circle", NodeShape::Circle),
            (0.53, 0.35, "Pill", NodeShape::Pill),
            (0.78, 0.65, "Diamond", NodeShape::Diamond),
        ]
        .into_iter()
        .map(|(fx, fy, label, shape)| {
            (
                Rect::new(
                    stage.x + fx * stage.width,
                    stage.y + fy * stage.height,
                    124.0,
                    44.0,
                ),
                label,
                shape,
            )
        })
        .collect();
        for pair in shapes.windows(2) {
            let (from, to) = (pair[0].0, pair[1].0);
            kit::elbow(
                ui,
                Point::new(from.right(), from.y + from.height * 0.5),
                Point::new(to.x, to.y + to.height * 0.5),
                2.0,
                INFO,
            );
        }
        for (i, (rect, label, shape)) in shapes.iter().enumerate() {
            if GraphNode::new(*shape)
                .selected(self.selected == *label)
                .show(ui, &format!("graph-3-{i}"), *rect, label)
            {
                self.selected = label.to_string();
            }
        }
        page.space(GRAPH_SPACE);

        let card = page.row(400.0);
        self.card(ui, card);
        let slices = [
            PieSlice::new("Direct", 0.45, INFO),
            PieSlice::new("Referral", 0.33, SUCCESS),
            PieSlice::new("Social", 0.22, WARNING),
        ];
        for (i, half) in card
            .inset_by(Insets::new(20.0, 20.0, 20.0, 42.0))
            .columns(2, 0.0)
            .into_iter()
            .enumerate()
        {
            let mut column = Strip::column(half.split_left(400.0, 0.0).0, 0.0);
            Label::heading(18.0).show(
                ui,
                column.leading(28.0),
                ["Pie Chart", "Donut Chart (Center Info)"][i],
            );
            let legend = column.trailing(30.0);
            column.skip_leading(16.0);
            let chart = column
                .rest()
                .split_left(320.0, 0.0)
                .0
                .centered(216.0, 216.0);
            let pie = PieChart::new(&slices);
            let pie = if i == 1 {
                pie.donut().center_label("75%")
            } else {
                pie
            };
            pie.show(ui, chart);
            pie.legend(ui, legend);
        }
        page.space(GRAPH_SPACE);

        let card = page.row(390.0);
        self.card(ui, card);
        let mut rows = Strip::column(card.inset(20.0), 18.0);
        Label::heading(18.0).show(ui, rows.leading(28.0), "Stacked Bar Charts");
        Label::subheading().show(
            ui,
            rows.leading(20.0),
            "Horizontal Stacked Bar Chart (Normalized 100%)",
        );
        let quarters = [
            BarGroup::new("Q1 2026", vec![400.0, 300.0, 200.0]),
            BarGroup::new("Q2 2026", vec![550.0, 250.0, 350.0]),
        ];
        let horizontal = StackedBarChart::new(&quarters);
        horizontal.show(
            ui,
            rows.leading(horizontal.extent())
                .inset_by(Insets::new(0.0, 0.0, 16.0, 0.0)),
        );
        // The columns stand on the bottom of the card, their heading just above the tallest.
        let years = [
            BarGroup::new("2024", vec![18.0; 3]),
            BarGroup::new("2025", vec![28.0; 3]),
            BarGroup::new("2026", vec![38.0; 3]),
        ];
        let colors = [INFO, rgb(0x06b6d4), WARNING];
        let vertical = StackedBarChart::new(&years)
            .colors(&colors)
            .vertical()
            .bars(44.0, 136.0);
        let chart = rows.trailing(vertical.extent());
        Label::subheading().show(
            ui,
            rows.trailing(20.0),
            "Vertical Stacked Bar Chart (Absolute Values)",
        );
        vertical.show(ui, chart.inset_by(Insets::new(44.0, 0.0, 0.0, 0.0)));
    }

    /// A graph card with its title and optional subtitle; returns the area
    /// under them for the graph.
    fn graph_card(&self, ui: &mut Ui, card: Rect, title: &str, subtitle: Option<&str>) -> Rect {
        self.card(ui, card);
        let mut column = Strip::column(card.inset_by(Insets::new(20.0, 18.0, 20.0, 20.0)), 4.0);
        // The title keeps clear of the zoom controls.
        Label::heading(18.0).show(
            ui,
            column
                .leading(30.0)
                .inset_by(Insets::new(0.0, 0.0, 160.0, 0.0)),
            title,
        );
        match subtitle {
            Some(subtitle) => {
                Label::muted()
                    .size(13.0)
                    .show(ui, column.leading(24.0), subtitle);
                column.skip_leading(8.0);
            }
            None => column.skip_leading(16.0),
        }
        column.rest()
    }

    fn dialog(&mut self, ui: &mut Ui, window: Rect) {
        let Some(dialog) = self.dialog else {
            return;
        };
        let modal = Modal::new(540.0, 380.0).glass(self.glass);
        let questions = std::mem::take(&mut self.survey_questions);
        let survey = &mut self.survey;
        let (closed, finished) = modal.show(ui, "dialog", window, |ui, rect| match dialog {
            Dialog::Modal => {
                let mut column =
                    Strip::column(rect.inset_by(Insets::new(24.0, 20.0, 24.0, 28.0)), 28.0);
                Label::heading(20.0).show(
                    ui,
                    column.leading(32.0).split_left(440.0, 0.0).0,
                    "Confirm Action",
                );
                Label::new().size(16.0).show(ui, column.leading(68.0), "Are you sure you want to perform this action? This operation will affect the active configuration.");
                Notification::new(StatusVariant::Warning).show(
                    ui,
                    column.leading(68.0),
                    "Warning",
                    "This operation cannot be undone.",
                );
                let mut buttons = Strip::row(column.trailing(38.0), 12.0);
                let confirm = buttons.trailing(118.0);
                let cancel = buttons.trailing(110.0);
                let cancelled = Button::new("Cancel")
                    .variant(ButtonVariant::Outline)
                    .show(ui, "cancel", cancel)
                    .clicked;
                let confirmed = Button::new("Confirm").show(ui, "confirm", confirm).clicked;
                cancelled || confirmed
            }
            Dialog::Survey => {
                FormFlow::new(&questions).show(ui, "survey", rect, survey)
                    == Some(FormFlowEvent::Submitted)
            }
        });
        self.survey_questions = questions;
        if finished && dialog == Dialog::Survey {
            self.survey_result = Some(self.survey_result());
        }
        if closed || finished {
            self.dialog = None;
        }
    }

    fn survey_result(&self) -> SurveyResult {
        let role = match self.survey.answers.get(1) {
            Some(Answer::Single(role)) => *role,
            _ => 0,
        };
        let tech_stack = match self.survey.answers.get(2) {
            Some(Answer::Multiple(chosen)) if role == 0 => STACKS
                .iter()
                .zip(chosen)
                .filter(|(_, on)| **on)
                .map(|(s, _)| s.to_string())
                .collect(),
            _ => Vec::new(),
        };
        let feedback = match self.survey.answers.get(3) {
            Some(Answer::Text(text)) => text.clone(),
            _ => String::new(),
        };
        SurveyResult {
            role: ROLES[role].into(),
            tech_stack,
            feedback,
        }
    }
}

/// Where a graph card's zoom controls go: its top-right corner.
fn controls(card: Rect) -> Rect {
    card.inset_by(Insets::new(0.0, 14.0, 14.0, 0.0))
        .split_top(30.0, 0.0)
        .0
}
