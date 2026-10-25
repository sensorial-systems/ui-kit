//! Pipelines compiled once and kept between launches. On Android the GPU
//! drivers keep no cache of their own, and a pipeline can take seconds to
//! compile on a phone's GPU: every launch paid for all of them again, on the
//! event loop's thread. Desktop drivers keep their own, so the cache is made
//! only on Android, and only where the device can keep one (Vulkan).
//!
//! It lives in the app's files, named for the GPU (wgpu's
//! `pipeline_cache_key`); wgpu checks the driver too, and starts empty from
//! data another driver made.
use crate::gpu::Gpu;
use std::path::PathBuf;

pub(crate) struct PipelineCache {
    cache: wgpu::PipelineCache,
    path: PathBuf,
    /// How much of it is on disk, to write it only once it has grown.
    saved: usize,
}

impl PipelineCache {
    /// The cache kept for `gpu`'s device, or `None` where there is none to
    /// keep.
    pub fn open(gpu: &Gpu) -> Option<Self> {
        if !cfg!(target_os = "android") || !gpu.device.features().contains(wgpu::Features::PIPELINE_CACHE) {
            return None;
        }
        let key = wgpu::util::pipeline_cache_key(&gpu.adapter.get_info())?;
        let path = files_dir()?.join(key);
        let data = std::fs::read(&path).ok();
        // Safety: the data is only ever what `get_data` gave for this key,
        // written whole (see `save`); with `fallback`, data the driver can't
        // use leaves the cache empty instead.
        let cache = unsafe {
            gpu.device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
                label: Some("pipelines"),
                data: data.as_deref(),
                fallback: true,
            })
        };
        Some(Self {
            cache,
            path,
            saved: data.map_or(0, |d| d.len()),
        })
    }

    pub fn cache(&self) -> &wgpu::PipelineCache {
        &self.cache
    }

    /// Writes the cache out if pipelines were added to it since it was read
    /// or last written: through a temporary file, so a launch never reads
    /// half of one.
    pub fn save(&mut self) {
        let Some(data) = self.cache.get_data() else {
            return;
        };
        if data.len() == self.saved {
            return;
        }
        let temporary = self.path.with_extension("partial");
        match std::fs::write(&temporary, &data).and_then(|()| std::fs::rename(&temporary, &self.path)) {
            Ok(()) => self.saved = data.len(),
            Err(e) => eprintln!("ui-kit-app: the pipeline cache {}: {e}", self.path.display()),
        }
    }
}

#[cfg(target_os = "android")]
fn files_dir() -> Option<PathBuf> {
    crate::android::files_dir()
}

#[cfg(not(target_os = "android"))]
fn files_dir() -> Option<PathBuf> {
    None
}
