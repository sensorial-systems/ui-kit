//! Running on Android: the activity hands `android_main` an [`AndroidApp`],
//! which becomes the event loop.
//!
//! ```ignore
//! #[unsafe(no_mangle)]
//! fn android_main(android: ui_kit_app::android::AndroidApp) {
//!     let event_loop = ui_kit_app::android::event_loop(android).unwrap();
//!     ui_kit_app::run_app_on(event_loop, MyApp::default(), AppConfig::new("My app")).unwrap();
//! }
//! ```
//!
//! The library is loaded by a `NativeActivity` (cargo-apk, xbuild), so the
//! application crate builds as a `cdylib`.
use jni::objects::{JObject, JValue};
use jni::{jni_sig, jni_str, Env, JavaVM};
use std::sync::OnceLock;
use winit::event_loop::{EventLoop, EventLoopBuilder};
pub use winit::platform::android::activity::{AndroidApp, WindowManagerFlags};
pub use winit::platform::android::EventLoopBuilderExtAndroid;

/// The activity the event loop was built for, for calls into its window.
static ACTIVITY: OnceLock<AndroidApp> = OnceLock::new();

/// The event loop for an Android activity.
pub fn event_loop(app: AndroidApp) -> anyhow::Result<EventLoop<()>> {
    event_loop_with(app, |_| {})
}

/// The event loop for an Android activity, with further Android options set
/// on its builder by `configure` (an input filter, volume key handling).
pub fn event_loop_with(
    app: AndroidApp,
    configure: impl FnOnce(&mut EventLoopBuilder<()>),
) -> anyhow::Result<EventLoop<()>> {
    let _ = ACTIVITY.set(app.clone());
    let mut builder = EventLoop::builder();
    builder.with_android_app(app);
    configure(&mut builder);
    Ok(builder.build()?)
}

/// Hides the status and navigation bars, the way a game does: they come back
/// for a moment with a swipe from the edge of the screen, and the content
/// also takes the area beside a camera cutout. Android shows the bars again
/// after the app leaves the screen, so the runner calls this again on every
/// resume and focus when [`AppConfig::fullscreen`](crate::AppConfig) is set.
pub fn hide_system_bars() {
    let Some(app) = ACTIVITY.get().cloned() else {
        return;
    };
    // Views may only be touched from the Java UI thread.
    let activity = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { JavaVM::from_raw(activity.vm_as_ptr() as _) };
        let result = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let result = immersive(env, activity.activity_as_ptr() as _);
            if env.exception_check() {
                env.exception_clear();
            }
            result
        });
        if let Err(e) = result {
            eprintln!("ui-kit-app: could not hide the system bars: {e}");
        }
    }));
}

fn immersive(env: &mut Env, activity: jni::sys::jobject) -> jni::errors::Result<()> {
    let activity = unsafe { JObject::from_raw(env, activity) };
    let window = env
        .call_method(&activity, jni_str!("getWindow"), jni_sig!("()Landroid/view/Window;"), &[])?
        .l()?;
    // Draw beside the cutout on the short edges (API 28+).
    const LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES: i32 = 1;
    let attributes = env
        .call_method(
            &window,
            jni_str!("getAttributes"),
            jni_sig!("()Landroid/view/WindowManager$LayoutParams;"),
            &[],
        )?
        .l()?;
    env.set_field(
        &attributes,
        jni_str!("layoutInDisplayCutoutMode"),
        jni_sig!("I"),
        JValue::Int(LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES),
    )?;
    env.call_method(
        &window,
        jni_str!("setAttributes"),
        jni_sig!("(Landroid/view/WindowManager$LayoutParams;)V"),
        &[JValue::Object(&attributes)],
    )?;
    if AndroidApp::sdk_version() >= 30 {
        // WindowInsetsController (API 30+).
        const BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE: i32 = 2;
        env.call_method(&window, jni_str!("setDecorFitsSystemWindows"), jni_sig!("(Z)V"), &[JValue::Bool(false)])?;
        let controller = env
            .call_method(
                &window,
                jni_str!("getInsetsController"),
                jni_sig!("()Landroid/view/WindowInsetsController;"),
                &[],
            )?
            .l()?;
        let bars = env
            .call_static_method(jni_str!("android/view/WindowInsets$Type"), jni_str!("systemBars"), jni_sig!("()I"), &[])?
            .i()?;
        env.call_method(&controller, jni_str!("hide"), jni_sig!("(I)V"), &[JValue::Int(bars)])?;
        env.call_method(
            &controller,
            jni_str!("setSystemBarsBehavior"),
            jni_sig!("(I)V"),
            &[JValue::Int(BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE)],
        )?;
    } else {
        // View.SYSTEM_UI_FLAG_*: immersive sticky, fullscreen, hide navigation,
        // and lay out as if they were hidden.
        const FLAGS: i32 = 0x1000 | 0x4 | 0x2 | 0x400 | 0x200 | 0x100;
        let decor = env
            .call_method(&window, jni_str!("getDecorView"), jni_sig!("()Landroid/view/View;"), &[])?
            .l()?;
        env.call_method(&decor, jni_str!("setSystemUiVisibility"), jni_sig!("(I)V"), &[JValue::Int(FLAGS)])?;
    }
    Ok(())
}
