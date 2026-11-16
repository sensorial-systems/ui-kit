//! Running on Android: the activity hands `android_main` an [`AndroidApp`],
//! which becomes the event loop.
//!
//! ```ignore
//! #[unsafe(no_mangle)]
//! fn android_main(android: ui_kit_app::android::AndroidApp) {
//!     use ui_kit_app::android;
//!     ui_kit_app::set_log_tag("my-app");
//!     android::abort_on_panic();
//!     android::choose_backend("debug.my_app.backend");
//!     let result = android::event_loop(android)
//!         .and_then(|event_loop| ui_kit_app::run_app_on(event_loop, MyApp::default(), AppConfig::new("My app")));
//!     android::finish(result)
//! }
//! ```
//!
//! [`abort_on_panic`] and [`choose_backend`] are each a fix for what Google
//! Play reported of an app on many phones: a panic that froze it, and
//! Vulkan drivers that crash. An abort as it closed is fixed by the fork
//! below; [`exit`] stands in for it in an app built without it.
//!
//! The library is loaded by a `NativeActivity` (cargo-apk, xbuild), so the
//! application crate builds as a `cdylib`.
//!
//! The application's workspace also takes android-activity from
//! sensorial-systems' fork, whose lifecycle callbacks wait at most 2 s for
//! `android_main` (Android reported a busy one as not responding) and which
//! lets go of an input queue Android destroys once `android_main` has
//! returned (detached later, it aborted). A patch applies only from the
//! workspace being built, so each application adds it:
//!
//! ```toml
//! [patch.crates-io]
//! android-activity = { git = "https://github.com/sensorial-systems/android-activity", rev = "20eb77339b725d29c3f726c4e9daf924c810652f" }
//! ```
use jni::objects::{JObject, JValue};
use jni::{jni_sig, jni_str, Env, JavaVM};
use std::sync::Mutex;
use winit::event_loop::{EventLoop, EventLoopBuilder};
pub use winit::platform::android::activity::{AndroidApp, WindowManagerFlags};
pub use winit::platform::android::EventLoopBuilderExtAndroid;

/// The activity the event loop was built for, for calls into its window.
/// Android may make another activity in the same process (after the user
/// backs out and returns, say), and calls `android_main` again for it.
static ACTIVITY: Mutex<Option<AndroidApp>> = Mutex::new(None);

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
    *ACTIVITY.lock().unwrap_or_else(|p| p.into_inner()) = Some(app.clone());
    let mut builder = EventLoop::builder();
    builder.with_android_app(app);
    configure(&mut builder);
    Ok(builder.build()?)
}

/// A line in logcat at error priority, under `tag` (stderr goes nowhere on
/// Android).
pub fn log(tag: &str, message: &str) {
    #[link(name = "log")]
    unsafe extern "C" {
        fn __android_log_write(priority: i32, tag: *const std::ffi::c_char, text: *const std::ffi::c_char) -> i32;
    }
    const ERROR: i32 = 6;
    let tag = std::ffi::CString::new(tag.replace('\0', " ")).unwrap_or_default();
    let text = std::ffi::CString::new(message.replace('\0', " ")).unwrap_or_default();
    unsafe { __android_log_write(ERROR, tag.as_ptr(), text.as_ptr()) };
}

/// A panic on this thread (`android_main`'s, which runs the event loop)
/// ends the process, its message the crash's in Google Play. Call it first
/// thing in `android_main`.
///
/// android-activity catches the panic otherwise and finishes the activity,
/// but the process lives on with its last frame up and the input queue left
/// to Android's teardown ([`exit`] is skipped): the app froze, and Google
/// Play heard of it as not responding at best, never with the message.
/// Other threads' panics are left as they were.
pub fn abort_on_panic() {
    unsafe extern "C" {
        fn android_set_abort_message(message: *const std::ffi::c_char);
    }
    let main = std::thread::current().id();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        if std::thread::current().id() == main {
            let message = format!("{}: {info}", crate::log_tag());
            log(crate::log_tag(), &message);
            let text = std::ffi::CString::new(message.replace('\0', " ")).unwrap_or_default();
            unsafe { android_set_abort_message(text.as_ptr()) };
            std::process::abort();
        }
    }));
}

/// Says in logcat how the event loop ended: the last call of
/// `android_main`, which then returns. The activity finishes, and a launch
/// after that starts `android_main` again in the same process.
pub fn finish(result: anyhow::Result<()>) {
    log_end(&result);
}

/// Ends the process once the event loop has, saying in logcat how it
/// ended: the last call of `android_main`, which never returns. For an app
/// built without the android-activity fork in this module's docs;
/// [`finish`] otherwise.
///
/// Once `android_main` returns, android-activity 0.6 stops following the
/// activity's input queue. Android then destroys that queue while the
/// activity finishes, and the glue's last reference, dropped in onStop or
/// onDestroy, detaches the dead queue from its looper: an abort
/// ("pthread_mutex_lock called on a destroyed mutex"), among Google Play's
/// commonest crashes of a Rust app. Ended here, the process is gone before
/// that; the next launch starts it afresh.
pub fn exit(result: anyhow::Result<()>) -> ! {
    log_end(&result);
    unsafe extern "C" {
        fn _exit(status: i32) -> !;
    }
    unsafe { _exit(if result.is_ok() { 0 } else { 1 }) }
}

fn log_end(result: &anyhow::Result<()>) {
    match result {
        Ok(()) => log(crate::log_tag(), "the event loop ended"),
        Err(e) => log(crate::log_tag(), &format!("the event loop ended: {e}")),
    }
}

/// Draws with OpenGL ES on the phones whose Vulkan drivers crash
/// ([`vulkan_crashes`](crate::gpu::vulkan_crashes)), Vulkan elsewhere, and
/// says which in logcat. Call it in `android_main` before anything makes a
/// wgpu instance: the choice is put in `WGPU_BACKEND`, which the instance
/// reads.
///
/// For testing, the system property `property` chooses on any phone:
///
/// ```sh
/// adb shell setprop debug.my_app.backend gl      # or vulkan; "" for the chip's
/// ```
pub fn choose_backend(property: &str) {
    let forced = system_property(property).filter(|b| !b.is_empty());
    let chip = vulkan_adapter();
    let gl = match forced.as_deref() {
        Some(backend) => backend.eq_ignore_ascii_case("gl"),
        None => chip.as_deref().is_some_and(crate::gpu::vulkan_crashes),
    };
    let chip = chip.as_deref().unwrap_or("no Vulkan adapter");
    let why = if forced.is_some() { format!("as {property} says") } else { "for this chip".into() };
    log(crate::log_tag(), &format!("graphics: {} on {chip}, {why}", if gl { "OpenGL ES" } else { "Vulkan" }));
    if gl {
        // SAFETY: called before the event loop and the threads that read the
        // environment; the Java threads alive now do not touch it.
        unsafe { std::env::set_var("WGPU_BACKEND", "gl") };
    }
}

/// The name of the first Vulkan adapter, if there is one. Enumerating is
/// safe on the drivers that crash: they crash only compiling pipelines.
fn vulkan_adapter() -> Option<String> {
    let descriptor = wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    };
    let instance = wgpu::Instance::new(descriptor);
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN));
    adapters.first().map(|a| a.get_info().name)
}

/// An Android system property (`getprop`), if it is set.
pub fn system_property(name: &str) -> Option<String> {
    unsafe extern "C" {
        fn __system_property_get(name: *const std::ffi::c_char, value: *mut std::ffi::c_char) -> i32;
    }
    const PROP_VALUE_MAX: usize = 92;
    let name = std::ffi::CString::new(name).ok()?;
    let mut value = [0 as std::ffi::c_char; PROP_VALUE_MAX];
    let len = unsafe { __system_property_get(name.as_ptr(), value.as_mut_ptr()) };
    (len > 0).then(|| unsafe { std::ffi::CStr::from_ptr(value.as_ptr()) }.to_string_lossy().into_owned())
}

/// The activity's own files folder (`Context.getFilesDir`).
pub(crate) fn files_dir() -> Option<std::path::PathBuf> {
    ACTIVITY.lock().unwrap_or_else(|p| p.into_inner()).as_ref()?.internal_data_path()
}

/// Hides the status and navigation bars, the way a game does: they come back
/// for a moment with a swipe from the edge of the screen, and the content
/// also takes the area beside a camera cutout. Android shows the bars again
/// after the app leaves the screen, so the runner calls this again on every
/// resume and focus when [`AppConfig::fullscreen`](crate::AppConfig) is set.
pub fn hide_system_bars() {
    let Some(app) = ACTIVITY.lock().unwrap_or_else(|p| p.into_inner()).clone() else {
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
