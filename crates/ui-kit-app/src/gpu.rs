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
