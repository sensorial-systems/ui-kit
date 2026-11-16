use crate::app::App;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use ui_kit_wgpu::GpuContext;

/// The device the renderer draws with, and what it was made from.
///
/// An application that renders content of its own -- a 3D view, a
/// simulation, a video -- wants it on this same device, so that what it
/// renders can be registered with the renderer and drawn by the UI without a
/// copy between devices. [`App::gpu_ready`] hands it over once it exists, and
/// [`App::device_descriptor`] says what the device has to support.
#[derive(Clone)]
pub struct Gpu {
    pub instance: Arc<wgpu::Instance>,
    pub adapter: Arc<wgpu::Adapter>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl Gpu {
    /// The device and queue, as the renderer takes them.
    pub fn context(&self) -> GpuContext {
        GpuContext::new(self.device.clone(), self.queue.clone())
    }
}

/// The instance surfaces and adapters come from: Prism's, which in a browser
/// is WebGPU where the browser has it and WebGL2 where it does not.
pub(crate) async fn instance() -> wgpu::Instance {
    prism_gpu::globals::WgpuContext::instance().await
}

/// An adapter that can present to `surface`, and the device the application
/// asks for on it.
pub(crate) async fn request<A: App>(
    instance: wgpu::Instance,
    surface: &wgpu::Surface<'_>,
    app: &A,
) -> Result<Gpu> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(surface),
            ..Default::default()
        })
        .await
        .map_err(|e| anyhow!("no graphics adapter can draw to this window: {e}"))?;
    let descriptor = app.device_descriptor(&adapter);
    let (device, queue) = adapter
        .request_device(&descriptor)
        .await
        .map_err(|e| anyhow!("the graphics device could not be created: {e}"))?;
    Ok(Gpu {
        instance: Arc::new(instance),
        adapter: Arc::new(adapter),
        device: Arc::new(device),
        queue: Arc::new(queue),
    })
}

/// Whether the Vulkan driver of the GPU named `name` (as its adapter calls
/// itself) crashes compiling ordinary pipelines, so that it is drawn with
/// OpenGL ES instead (`android::choose_backend`).
///
/// PowerVR's Rogue GE8xxx (GE8320 in MediaTek's Helio P35, GE8322 in
/// Unisoc's SC9863A: the cheapest Redmi, POCO, Galaxy A0x, Itel and ZTE
/// phones) segfault in their own `vulkan.*.so` inside
/// `vkCreateGraphicsPipelines`. Their OpenGL ES 3.2 drivers compile the
/// same shaders, by way of GLSL.
pub fn vulkan_crashes(name: &str) -> bool {
    name.contains("PowerVR Rogue GE8")
}

/// Set by [`survive_out_of_memory`]'s handler, read by [`Pace`].
static OUT_OF_MEMORY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A frame the driver has no memory for is lost, not the app. wgpu's own
/// handler panics on any error, and a phone's driver (a Mali's, say) can run
/// out of memory for a heavy frame's commands and have it again for the
/// next, lighter one. The commands it could not record are then refused as
/// they are submitted: a second or two of errors that follow from it, which
/// are logged too. Any other error is a bug, and panics as before. [`Pace`]
/// counts the lost frame as the GPU being behind.
pub fn survive_out_of_memory(device: &wgpu::Device) {
    device.on_uncaptured_error(Arc::new(survive));
}

fn survive(error: wgpu::Error) {
    use std::sync::Mutex;
    use web_time::{Duration, Instant};
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let following = last.is_some_and(|t| t.elapsed() < Duration::from_secs(2));
    match error {
        wgpu::Error::OutOfMemory { .. } => {
            *last = Some(Instant::now());
            OUT_OF_MEMORY.store(true, std::sync::atomic::Ordering::Relaxed);
            if !following {
                crate::log_error("graphics: the GPU ran out of memory for a frame, which is dropped");
            }
        }
        error if following => crate::log_error(&format!("graphics: after running out of memory: {error}")),
        error => panic!("wgpu error: {error}"),
    }
}

/// Keeping a frame's commands from piling up on a slow GPU, and noticing
/// when it stays behind.
///
/// A frame's commands hold the driver's memory until the GPU has drawn
/// them. Made while the GPU still draws the last one, two frames of a heavy
/// scene can be more than a phone's driver gives. [`Pace::wait`] waits for
/// the GPU after each frame, but no longer than a limit: on a phone whose
/// GPU took seconds a frame, the event loop's thread was held as long, and
/// Android reported the app as not responding. A GPU that keeps going over
/// the limits is behind, and the app draws fewer pixels, say.
#[cfg(not(target_arch = "wasm32"))]
pub struct Pace {
    /// The longest the GPU is waited for after a frame.
    pub wait: std::time::Duration,
    /// A frame the GPU took this long over is slow.
    pub slow: std::time::Duration,
    /// This many slow frames in a row, and the GPU is behind.
    pub frames: u32,
    slow_frames: u32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for Pace {
    fn default() -> Self {
        use std::time::Duration;
        Self { wait: Duration::from_millis(500), slow: Duration::from_millis(250), frames: 3, slow_frames: 0 }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Pace {
    /// Waits, at most [`Pace::wait`], for the GPU to draw what was
    /// submitted, and says whether it is behind: [`Pace::frames`] slow frames
    /// in a row, the count then starting again. A frame lost for want of
    /// memory ([`survive_out_of_memory`]) counts as that many slow frames:
    /// the GPU is behind at once.
    pub fn wait(&mut self, device: &wgpu::Device) -> bool {
        let started = std::time::Instant::now();
        let waited = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(self.wait) });
        let starved = OUT_OF_MEMORY.swap(false, std::sync::atomic::Ordering::Relaxed);
        if starved {
            self.slow_frames = self.slow_frames.max(self.frames.saturating_sub(1));
        }
        let slow = starved || waited.is_err() || started.elapsed() >= self.slow;
        self.slow_frames = if slow { self.slow_frames + 1 } else { 0 };
        let behind = self.slow_frames >= self.frames;
        if behind {
            self.slow_frames = 0;
        }
        behind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_rogue_ge8_chips_are_kept_off_vulkan() {
        assert!(vulkan_crashes("PowerVR Rogue GE8320"));
        assert!(vulkan_crashes("PowerVR Rogue GE8322"));
        assert!(!vulkan_crashes("PowerVR B-Series BXM-8-256"));
        assert!(!vulkan_crashes("Mali-G615 MC2"));
        assert!(!vulkan_crashes("Adreno (TM) 610"));
    }
}
