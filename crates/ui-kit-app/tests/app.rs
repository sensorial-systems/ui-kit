use ui_kit_app::{scale_commands, App, AppConfig, ClosureApp, FrameContext};
use ui_kit_core::{Rect, Style};
use ui_kit_immediate::{DrawCommand, Input, Layer};

struct TestApp {
    frames: usize,
}

impl App for TestApp {
    fn frame(&mut self, ctx: &FrameContext) -> Vec<DrawCommand> {
        self.frames += 1;
        assert_eq!(ctx.width, 800.0);
        assert_eq!(ctx.height, 600.0);
        vec![DrawCommand {
            layer: Layer::BASE,
            rect: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
            },
            clip: None,
            style: Style::default(),
            kind: "panel".into(),
            text: String::new(),
            rich_text: None,
            value: 0.0,
            hovered: false,
            active: false,
            focused: false,
        }]
    }
}

#[test]
fn app_trait_and_closure_adapter_work() {
    let mut app = TestApp { frames: 0 };
    let ctx = FrameContext::new(Input::default(), 800.0, 600.0, 0.0, 1.0, true);
    let commands = app.frame(&ctx);
    assert_eq!(app.frames, 1);
    assert_eq!(commands.len(), 1);

    let mut closure_app = ClosureApp::new(|ctx: &FrameContext| {
        assert_eq!(ctx.width, 800.0);
        vec![]
    });
    let empty = closure_app.frame(&ctx);
    assert!(empty.is_empty());
}

#[test]
fn app_config_builder_customizes_properties() {
    let config = AppConfig::new("Demo")
        .with_size(1024.0, 768.0)
        .with_min_size(400.0, 300.0)
        .with_dark(false)
        .with_fps(120)
        .with_transparent(true);

    assert_eq!(config.title, "Demo");
    assert_eq!(config.width, 1024.0);
    assert_eq!(config.height, 768.0);
    assert_eq!(config.min_width, Some(400.0));
    assert_eq!(config.min_height, Some(300.0));
    assert!(!config.dark);
    assert_eq!(config.fps, 120);
    assert!(config.transparent);
}

#[test]
fn scale_commands_scales_geometry_and_styles() {
    let mut commands = vec![DrawCommand {
        layer: Layer::BASE,
        rect: Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        },
        clip: Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        }),
        style: Style {
            radius: 8.0,
            font_size: 14.0,
            text_padding: 6.0,
            stroke_width: 1.0,
            ..Default::default()
        },
        kind: "shadow".into(),
        text: String::new(),
        rich_text: None,
        value: 12.0,
        hovered: false,
        active: false,
        focused: false,
    }];

    scale_commands(&mut commands, 2.0);

    assert_eq!(commands[0].rect.x, 20.0);
    assert_eq!(commands[0].rect.y, 40.0);
    assert_eq!(commands[0].rect.width, 200.0);
    assert_eq!(commands[0].rect.height, 100.0);
    assert_eq!(commands[0].clip.unwrap().width, 400.0);
    assert_eq!(commands[0].style.radius, 16.0);
    assert_eq!(commands[0].style.font_size, 28.0);
    assert_eq!(commands[0].style.text_padding, 12.0);
    assert_eq!(commands[0].style.stroke_width, 2.0);
    assert_eq!(commands[0].value, 24.0);
}
