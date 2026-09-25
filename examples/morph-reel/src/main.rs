//! Render the morph reel without a window: single frames, one frame per beat
//! as a contact sheet, the sound cues, and the full motion-blurred video.
//!
//! ```text
//! morph-reel frame  <seconds> <out.png>
//! morph-reel beats  <dir>                      one PNG per beat + sheet.png
//! morph-reel cues   <out.json>                 where every UI sound belongs
//! morph-reel render <out.mp4> [--audio mix.wav] [--laps 2] [--fps 60] [--subframes 4]
//!
//! options: --bpm 120  --accent 3366ff  --size 1440  --at 0.4 (beats, for `beats`)
//! ```
//!
//! `render` draws `subframes` moments inside every output frame and pipes
//! them to ffmpeg, whose `tmix` averages each group into one frame: motion
//! blur from real in-between frames rather than a filter's guess.
use anyhow::{anyhow, bail, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use ui_kit_core::morph::{Reel, REEL_BEATS};
use ui_kit_core::motion::BeatGrid;
use ui_kit_core::Rect;
use ui_kit_immediate::components::paint_scene;
use ui_kit_immediate::composition::UiRenderer;
use ui_kit_immediate::{Input, Ui};
use ui_kit_wgpu::{GpuContext, TextureTarget, VectorRenderer, WgpuFrame, WgpuRenderer};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let option = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let bpm: f32 = option("--bpm").map_or(Ok(120.0), |v| v.parse())?;
    let accent = option("--accent").map(|hex| parse_hex(&hex)).transpose()?;
    let size: u32 = option("--size").map_or(Ok(1440), |v| v.parse())?;
    let reel = Reel::new(BeatGrid::new(bpm), accent);
    let positional: Vec<&String> = {
        let mut out = vec![];
        let mut skip = false;
        for a in &args {
            if skip {
                skip = false;
            } else if a.starts_with("--") {
                skip = true;
            } else {
                out.push(a);
            }
        }
        out
    };
    let usage = || anyhow!("usage: morph-reel frame|beats|cues|render ... (see the source header)");
    match positional.first().map(|s| s.as_str()) {
        Some("frame") => {
            let time: f32 = positional.get(1).ok_or_else(usage)?.parse()?;
            let out = positional.get(2).ok_or_else(usage)?;
            let mut canvas = Canvas::new(size)?;
            let pixels = canvas.draw(&reel, time)?;
            write_png(out, size, size, &pixels)?;
            println!("Wrote {out}");
        }
        Some("beats") => {
            let dir = std::path::PathBuf::from(positional.get(1).ok_or_else(usage)?);
            std::fs::create_dir_all(&dir)?;
            let at: f32 = option("--at").map_or(Ok(0.4), |v| v.parse())?;
            let mut canvas = Canvas::new(size)?;
            // A contact sheet: one row per bar, one column per beat.
            let cell = size / 4;
            let (columns, rows) = (4u32, (REEL_BEATS as u32).div_ceil(4));
            let mut sheet = vec![0u8; (cell * columns * cell * rows * 4) as usize];
            for beat in 0..REEL_BEATS as u32 {
                let time = (beat as f32 + at) * reel.grid().beat_length();
                let pixels = canvas.draw(&reel, time)?;
                let path = dir.join(format!("beat-{beat:02}.png"));
                write_png(path.to_str().unwrap(), size, size, &pixels)?;
                let small = shrink(&pixels, size, 4);
                let (cx, cy) = (beat % columns * cell, beat / columns * cell);
                for y in 0..cell {
                    let from = (y * cell * 4) as usize;
                    let to = (((cy + y) * cell * columns + cx) * 4) as usize;
                    sheet[to..to + cell as usize * 4]
                        .copy_from_slice(&small[from..from + cell as usize * 4]);
                }
                println!(
                    "beat {beat:>2}  {:<16} {}",
                    reel.state_at(time).name(),
                    Reel::beat_caption(beat as usize)
                );
            }
            let path = dir.join("sheet.png");
            write_png(path.to_str().unwrap(), cell * columns, cell * rows, &sheet)?;
            println!("Wrote {}", dir.display());
        }
        Some("cues") => {
            let out = positional.get(1).ok_or_else(usage)?;
            let json = serde_json::json!({
                "bpm": bpm,
                "beats": REEL_BEATS,
                "duration": reel.duration(),
                "cues": reel.cues().iter().map(|c| serde_json::json!({
                    "time": c.time,
                    "beat": c.beat,
                    "sound": c.sound.name(),
                })).collect::<Vec<_>>(),
            });
            std::fs::write(out, serde_json::to_string_pretty(&json)?)?;
            println!("Wrote {out}");
        }
        Some("render") => {
            let out = positional.get(1).ok_or_else(usage)?;
            let laps: u32 = option("--laps").map_or(Ok(2), |v| v.parse())?;
            let fps: u32 = option("--fps").map_or(Ok(60), |v| v.parse())?;
            let subframes: u32 = option("--subframes").map_or(Ok(4), |v| v.parse())?;
            render(&reel, out, size, laps, fps, subframes, option("--audio"))?;
        }
        _ => return Err(usage()),
    }
    Ok(())
}

fn render(
    reel: &Reel,
    out: &str,
    size: u32,
    laps: u32,
    fps: u32,
    subframes: u32,
    audio: Option<String>,
) -> Result<()> {
    let frames = (reel.duration() * laps as f32 * fps as f32).round() as u32;
    let rate = fps * subframes;
    let mut ffmpeg = Command::new("ffmpeg");
    ffmpeg.args(["-y", "-loglevel", "error", "-stats"]);
    ffmpeg.args(["-f", "rawvideo", "-pix_fmt", "rgba"]);
    ffmpeg.args([
        "-s",
        &format!("{size}x{size}"),
        "-r",
        &rate.to_string(),
        "-i",
        "-",
    ]);
    if let Some(audio) = &audio {
        ffmpeg.args(["-i", audio]);
    }
    // Average each group of subframes, keep the last frame of each group
    // (the one the average ends on), and time the result at `fps`.
    let weights = vec!["1"; subframes as usize].join(" ");
    let filter = format!(
        "tmix=frames={subframes}:weights='{weights}',select='not(mod(n+1\\,{subframes}))',setpts=N/({fps}*TB),format=yuv420p"
    );
    ffmpeg.args(["-vf", &filter, "-r", &fps.to_string()]);
    ffmpeg.args(["-c:v", "libx264", "-preset", "slow", "-crf", "14"]);
    if audio.is_some() {
        ffmpeg.args(["-c:a", "aac", "-b:a", "256k", "-shortest"]);
    }
    ffmpeg.args(["-movflags", "+faststart", out]);
    let mut child = ffmpeg
        .stdin(Stdio::piped())
        .spawn()
        .context("ffmpeg must be on PATH")?;
    let mut stdin = child.stdin.take().unwrap();
    let mut canvas = Canvas::new(size)?;
    let started = std::time::Instant::now();
    for frame in 0..frames {
        for sub in 0..subframes {
            // Subframes spread over the frame's interval, centred on it.
            let offset = (sub as f32 + 0.5) / subframes as f32 - 0.5;
            let time = (frame as f32 + offset) / fps as f32;
            stdin.write_all(&canvas.draw(reel, time)?)?;
        }
        if frame % fps == 0 {
            eprint!("\rframe {frame}/{frames}");
        }
    }
    drop(stdin);
    let status = child.wait()?;
    if !status.success() {
        bail!("ffmpeg failed: {status}");
    }
    eprintln!(
        "\rWrote {out}: {frames} frames, {subframes} subframes each, in {:.0}s",
        started.elapsed().as_secs_f32()
    );
    Ok(())
}

/// A headless renderer and the texture it draws into.
struct Canvas {
    gpu: GpuContext,
    renderer: WgpuRenderer<VectorRenderer>,
    target: ui_kit_wgpu::Texture2d,
    readback: wgpu::Buffer,
    size: u32,
    pitch: u32,
    ui: Ui,
}

impl Canvas {
    fn new(size: u32) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
        let gpu = GpuContext::new(Arc::new(device), Arc::new(queue));
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let renderer = WgpuRenderer::with_custom(&gpu, format, VectorRenderer::new(&gpu, format))?;
        let target = renderer.create_texture(size, size)?;
        let pitch = (size * 4).div_ceil(256) * 256;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("morph reel readback"),
            size: (pitch * size) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut ui = Ui::default();
        ui.set_shared_text_measure(renderer.text_measure());
        Ok(Self {
            gpu,
            renderer,
            target,
            readback,
            size,
            pitch,
            ui,
        })
    }

    /// The reel at `time`, as tightly packed RGBA rows.
    fn draw(&mut self, reel: &Reel, time: f32) -> Result<Vec<u8>> {
        let size = self.size as f32;
        let scene = reel.seek(time).placed(Rect::new(0.0, 0.0, size, size));
        self.ui.begin(Input::default());
        paint_scene(&mut self.ui, &scene);
        let commands = self.ui.end().to_vec();
        let [r, g, b, _] = scene.background;
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        self.renderer.render(
            &mut WgpuFrame {
                encoder: &mut encoder,
                target: TextureTarget::from_texture2d(&self.target)?,
                clear: Some(wgpu::Color {
                    r: r as f64,
                    g: g as f64,
                    b: b as f64,
                    a: 1.0,
                }),
            },
            &commands,
        )?;
        encoder.copy_texture_to_buffer(
            self.target.texture.as_ref().unwrap().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.pitch),
                    rows_per_image: Some(self.size),
                },
            },
            wgpu::Extent3d {
                width: self.size,
                height: self.size,
                depth_or_array_layers: 1,
            },
        );
        self.gpu.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        self.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        self.gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let row = self.size as usize * 4;
        let pixels = {
            let mapped = self.readback.slice(..).get_mapped_range()?;
            mapped
                .chunks(self.pitch as usize)
                .flat_map(|r| r[..row].iter().copied())
                .collect()
        };
        self.readback.unmap();
        Ok(pixels)
    }
}

/// Every `factor` × `factor` block of an RGBA image averaged into a pixel.
fn shrink(pixels: &[u8], size: u32, factor: u32) -> Vec<u8> {
    let small = size / factor;
    let mut out = vec![0u8; (small * small * 4) as usize];
    for y in 0..small {
        for x in 0..small {
            for c in 0..4 {
                let mut sum = 0u32;
                for dy in 0..factor {
                    for dx in 0..factor {
                        let i = (((y * factor + dy) * size + x * factor + dx) * 4 + c) as usize;
                        sum += pixels[i] as u32;
                    }
                }
                out[((y * small + x) * 4 + c) as usize] = (sum / (factor * factor)) as u8;
            }
        }
    }
    out
}

fn write_png(path: &str, width: u32, height: u32, pixels: &[u8]) -> Result<()> {
    let mut png = png::Encoder::new(std::fs::File::create(path)?, width, height);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()?.write_image_data(pixels)?;
    Ok(())
}

fn parse_hex(hex: &str) -> Result<[f32; 4]> {
    let value = u32::from_str_radix(hex.trim_start_matches('#'), 16)?;
    Ok(ui_kit_core::morph::hex(value))
}
