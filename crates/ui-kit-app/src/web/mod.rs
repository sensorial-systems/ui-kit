//! Running in a browser: the page is the window.
//!
//! The application draws into a `<canvas>` -- one it names in
//! [`AppConfig::canvas`], or one filling the page -- on WebGPU where the
//! browser has it and WebGL where it does not. Frames follow the display's
//! refresh through `requestAnimationFrame`, input comes from the DOM's own
//! events (see [`input`]), and tasks are the page's promises.

mod input;

use crate::app::{App, WindowCommand};
use crate::config::AppConfig;
use crate::context::FrameContext;
use crate::gpu;
use crate::damage::Region;
use crate::present::Presenter;
use crate::scale::scale_commands;
use anyhow::{anyhow, Result};
use input::DomInput;
use std::cell::RefCell;
use std::rc::Rc;
use ui_kit_core::Rect;
use ui_kit_wgpu::{CommandRenderer, GpuContext};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlCanvasElement;

struct WebRunner<A, C> {
    app: A,
    config: AppConfig,
    canvas: HtmlCanvasElement,
    input: DomInput,
    presenter: Presenter<C>,
    start: f64,
    stopped: bool,
}

/// Start `app` on the page. Returns once it is started; the page's event
/// loop runs it from then on, and a failure to start is reported on the
/// console and on the page.
pub(crate) fn run<A, C, F>(app: A, config: AppConfig, make_custom: F) -> Result<()>
where
    A: App + 'static,
    C: CommandRenderer + 'static,
    F: FnOnce(&GpuContext, wgpu::TextureFormat) -> C + 'static,
{
    let canvas = canvas(&config).map_err(js_error)?;
    let input = DomInput::attach(&canvas).map_err(js_error)?;
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(error) = start(app, config, make_custom, canvas.clone(), input).await {
            let message = format!("{error:#}");
            web_sys::console::error_1(&message.clone().into());
            show_error(&canvas, &message);
        }
    });
    Ok(())
}

async fn start<A, C, F>(
    mut app: A,
    config: AppConfig,
    make_custom: F,
    canvas: HtmlCanvasElement,
    input: DomInput,
) -> Result<()>
where
    A: App + 'static,
    C: CommandRenderer + 'static,
    F: FnOnce(&GpuContext, wgpu::TextureFormat) -> C + 'static,
{
    let (width, height) = fit(&canvas);
    let instance = gpu::instance().await;
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| anyhow!("the canvas cannot be drawn to: {e}"))?;
    let gpu = gpu::request(instance, &surface, &app).await?;
    app.gpu_ready(&gpu);
    let presenter = Presenter::new(&gpu, surface, width, height, config.transparent, make_custom)?;
    app.fonts_ready(&presenter.renderer);
    app.text_measure_ready(presenter.renderer.text_measure());
    app.on_resume();

    let runner = Rc::new(RefCell::new(WebRunner {
        app,
        config,
        canvas,
        input,
        presenter,
        start: now(),
        stopped: false,
    }));
    animate(move || {
        let mut runner = runner.borrow_mut();
        if let Err(error) = runner.frame() {
            let message = format!("{error:#}");
            web_sys::console::error_1(&message.clone().into());
            show_error(&runner.canvas, &message);
            runner.stopped = true;
        }
        !runner.stopped
    });
    Ok(())
}

impl<A: App, C: CommandRenderer> WebRunner<A, C> {
    fn frame(&mut self) -> Result<()> {
        self.app.update();
        for command in self.app.window_commands() {
            // A page cannot hide or raise the browser's window.
            if command == WindowCommand::Exit {
                self.stopped = true;
            }
        }
        if self.stopped || self.app.should_exit() {
            self.stopped = true;
            self.app.on_suspend();
            return Ok(());
        }

        let (width, height) = fit(&self.canvas);
        if width == 0 || height == 0 {
            return Ok(());
        }
        let scale = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio()) as f32;
        let (logical_width, logical_height) = (width as f32 / scale, height as f32 / scale);

        // Pictures first, so the frame can draw what was just uploaded.
        self.app.prepare(&mut self.presenter.images());

        let time = (now() - self.start) / 1000.0;
        let viewport = Rect::new(0.0, 0.0, logical_width, logical_height);
        // The UI steps through the input in the order it came -- each
        // stretch between button changes a frame of its own -- and the last
        // step is the one drawn.
        let mut commands = Vec::new();
        for input in self.input.take(viewport, time) {
            if input.scroll.y != 0.0 {
                self.app.on_scroll(input.scroll.y);
            }
            let ctx = FrameContext::new(
                input,
                logical_width,
                logical_height,
                time as f32,
                scale,
                self.config.dark,
            );
            commands = self.app.frame(&ctx);
            if let Some(output) = self.app.output() {
                self.input.apply(&output);
            }
        }
        scale_commands(&mut commands, scale);
        // A frame that changes nothing leaves the canvas as it is.
        let region = self.presenter.region(&commands, width, height);
        if region == Region::Nothing {
            return Ok(());
        }
        let Some(surface_texture) = self.presenter.acquire(width, height)? else {
            return Ok(());
        };
        self.presenter.present(&surface_texture, commands, self.config.background, region)?;
        self.presenter.show(surface_texture);
        Ok(())
    }
}

/// The canvas to draw into: the one the configuration names, or a new one
/// filling the page.
fn canvas(config: &AppConfig) -> Result<HtmlCanvasElement, JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    document.set_title(&config.title);
    if let Some(id) = &config.canvas {
        return document
            .get_element_by_id(id)
            .ok_or_else(|| JsValue::from_str(&format!("no element with id `{id}`")))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str(&format!("`{id}` is not a canvas")));
    }
    let body = document.body().ok_or("no body")?;
    let [r, g, b, _] = config.background.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
    body.set_attribute(
        "style",
        &format!("margin: 0; overflow: hidden; overscroll-behavior: none; background: rgb({r}, {g}, {b});"),
    )?;
    let canvas: HtmlCanvasElement = document.create_element("canvas")?.dyn_into()?;
    canvas.set_attribute(
        "style",
        "position: fixed; left: 0; top: 0; width: 100vw; height: 100vh; display: block; \
         outline: none; touch-action: none; user-select: none; -webkit-user-select: none;",
    )?;
    body.append_child(&canvas)?;
    Ok(canvas)
}

/// Size the canvas's drawing buffer to the device pixels it covers on the
/// page, and return that size.
fn fit(canvas: &HtmlCanvasElement) -> (u32, u32) {
    let scale = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
    let width = (canvas.client_width() as f64 * scale).round().max(0.0) as u32;
    let height = (canvas.client_height() as f64 * scale).round().max(0.0) as u32;
    if canvas.width() != width {
        canvas.set_width(width);
    }
    if canvas.height() != height {
        canvas.set_height(height);
    }
    (width, height)
}

/// Call `frame` on every animation frame while it returns true.
fn animate(mut frame: impl FnMut() -> bool + 'static) {
    let next: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let first = next.clone();
    *first.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if frame() {
            request_frame(next.borrow().as_ref().unwrap());
        } else {
            // Break the closure's cycle with itself.
            next.borrow_mut().take();
        }
    }) as Box<dyn FnMut()>));
    request_frame(first.borrow().as_ref().unwrap());
}

fn request_frame(closure: &Closure<dyn FnMut()>) {
    if let Some(window) = web_sys::window() {
        let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
    }
}

/// Milliseconds since the page loaded.
fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or(0.0, |p| p.now())
}

/// Put `message` on the page in place of the canvas, so a failure to start
/// (no WebGPU or WebGL) says what happened instead of leaving it blank.
fn show_error(canvas: &HtmlCanvasElement, message: &str) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(element) = document.create_element("pre") else {
        return;
    };
    let _ = element.set_attribute(
        "style",
        "position: fixed; left: 24px; top: 24px; right: 24px; margin: 0; padding: 16px; \
         color: #eee; background: #2a1215; border: 1px solid #a33; border-radius: 8px; \
         font: 14px/1.4 ui-monospace, monospace; white-space: pre-wrap;",
    );
    element.set_text_content(Some(message));
    if let Some(parent) = canvas.parent_node() {
        let _ = parent.append_child(&element);
    }
}

fn js_error(value: JsValue) -> anyhow::Error {
    anyhow!("{}", value.as_string().unwrap_or_else(|| format!("{value:?}")))
}
