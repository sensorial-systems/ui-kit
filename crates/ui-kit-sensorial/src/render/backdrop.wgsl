// The world behind the page: black, as the brand's intro is, lit by light
// split as through a prism -- a fan of faint spectral rays from above, a
// haze where the subject of the screen hangs, and dust drifting at three
// depths that streaks and splits into its colours when the page moves fast.
// A layer of light the application renders -- a 3D logo, say -- is added
// over it.

struct Backdrop {
    // Target width and height in pixels, pixels per CSS pixel, whether the
    // target stores linear light.
    screen: vec4<f32>,
    // Seconds, scroll in CSS pixels, scroll speed in CSS pixels a second,
    // how much of the world is lit (0 during the intro, which is all black).
    motion: vec4<f32>,
    // Pointer in 0..1, the light layer's opacity, how far its colours split.
    pointer: vec4<f32>,
    // The brand's hue in the spectrum, a flash of light, where the haze
    // sits (0..1 down the screen), how wide it spreads.
    tone: vec4<f32>,
};

@group(0) @binding(0) var<uniform> b: Backdrop;
@group(0) @binding(1) var light: texture_2d<f32>;
@group(0) @binding(2) var clamped: sampler;

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> Out {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: Out;
    out.clip = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

// 0 red through 1 violet, as the intro colours its light.
fn spectrum(x: f32) -> vec3<f32> {
    let h = clamp(x, 0.0, 1.0) * 0.8;
    let rgb = clamp(abs(fract(h + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
    return rgb * rgb * (3.0 - 2.0 * rgb);
}

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(123.34, 456.21));
    q += dot(q, q + 45.32);
    return fract(q.x * q.y);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let n = hash21(p);
    return vec2<f32>(n, hash21(p + n + 17.17));
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let c = hash21(i + vec2<f32>(1.0, 0.0));
    let d = hash21(i + vec2<f32>(0.0, 1.0));
    let e = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, c, u.x), mix(d, e, u.x), u.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var q = p;
    for (var i = 0; i < 4; i++) {
        sum += noise(q) * amplitude;
        q = q * 2.03 + vec2<f32>(3.1, 1.7);
        amplitude *= 0.5;
    }
    return sum;
}

// The rays: light falling from a point above the top of the screen, fanned
// out and split, brightest along a few drifting streaks.
fn rays(q: vec2<f32>, time: f32, hue: f32) -> vec3<f32> {
    let source = vec2<f32>(0.0, -0.55);
    let d = q - source;
    let angle = atan2(d.x, d.y);
    let distance = length(d);
    let streaks = fbm(vec2<f32>(angle * 7.0 + time * 0.035, time * 0.02)) ;
    let fan = exp(-angle * angle * 3.2);
    let fall = exp(-distance * 1.35);
    let intensity = pow(streaks, 2.6) * fan * fall;
    let colour = mix(vec3<f32>(1.0), spectrum(0.5 + angle * 0.9 + (hue - 0.5) * 0.4), 0.75);
    return colour * intensity;
}

// Pools of coloured light far behind the page, drifting slowly, rising
// past as the page scrolls -- at a distance, slower than the page. What the
// glass panels bend and blur.
fn pools(q: vec2<f32>, aspect: f32, time: f32, drift: f32, hue: f32) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    for (var i = 0; i < 5; i++) {
        let fi = f32(i);
        let side = select(-1.0, 1.0, (i % 2) == 0);
        let centre = vec2<f32>(
            side * aspect * (0.22 + 0.1 * sin(time * 0.043 + fi * 1.7)) + 0.12 * sin(time * 0.031 + fi * 4.1),
            0.95 + fi * 0.62 + 0.1 * sin(time * 0.057 + fi * 2.3) - drift * 0.72,
        );
        let radius = 0.36 + 0.07 * sin(time * 0.083 + fi * 1.3);
        let d = (q - centre) / radius;
        let weight = exp(-dot(d, d) * 1.4);
        // Near the brand's own colour, a little either side of it.
        let offsets = array<f32, 5>(0.0, 0.2, -0.12, 0.28, 0.08);
        let tint = spectrum(clamp(hue + offsets[i], 0.02, 0.98));
        sum += mix(tint, vec3<f32>(1.0), 0.18) * weight;
    }
    return sum;
}

// Dust at one depth: a speck in some cells of a grid, drifting up with the
// page and towards the pointer, stretched along the way the page moves.
fn dust(px: vec2<f32>, depth: f32, cell: f32, stretch: f32, time: f32) -> f32 {
    let p = px / cell;
    let id = floor(p);
    let f = fract(p) - 0.5;
    let h = hash22(id + depth * 31.0);
    if (h.x < 0.62) {
        return 0.0;
    }
    let wander = vec2<f32>(sin(time * 0.21 + h.y * 6.28), cos(time * 0.17 + h.x * 6.28)) * 0.08;
    var d = (f - (h - 0.5) * 0.6 - wander) * cell;
    d.y /= stretch;
    let r = length(d);
    let twinkle = 0.55 + 0.45 * sin(time * (0.6 + h.y * 1.7) + h.x * 40.0);
    let size = 0.55 + depth * 0.9;
    return smoothstep(size + 0.9, 0.0, r) * twinkle * (0.25 + depth * 0.75);
}

fn dust_field(px: vec2<f32>, scroll: f32, velocity: f32, pointer: vec2<f32>, time: f32) -> f32 {
    var sum = 0.0;
    for (var i = 0; i < 3; i++) {
        let depth = 0.3 + f32(i) * 0.35;
        let cell = 70.0 + f32(i) * 55.0;
        let offset = vec2<f32>(0.0, scroll * depth * 0.45 + time * (2.0 + depth * 5.0)) + pointer * depth * 22.0;
        let stretch = 1.0 + abs(velocity) * depth * 0.0045;
        sum += dust(px + offset, depth, cell, stretch, time);
    }
    return sum;
}

fn to_linear(x: vec3<f32>) -> vec3<f32> {
    return select(pow((x + 0.055) / 1.055, vec3<f32>(2.4)), x / 12.92, x <= vec3<f32>(0.04045));
}

@fragment
fn fs_backdrop(in: Out) -> @location(0) vec4<f32> {
    let size = b.screen.xy;
    let dpr = b.screen.z;
    let uv = in.uv;
    let aspect = size.x / size.y;
    let time = b.motion.x;
    let scroll = b.motion.y;
    let velocity = b.motion.z;
    let lit = b.motion.w;
    let pointer = b.pointer.xy - vec2<f32>(0.5);
    let hue = b.tone.x;

    // CSS pixels, for sizes that should not change with the display.
    let css = in.clip.xy / dpr;
    let height_css = size.y / dpr;
    // The world scrolls by slower than the page: it is further away.
    let drift = scroll / height_css;

    var colour = vec3<f32>(0.0);
    if (lit > 0.0) {
        let q = vec2<f32>((uv.x - 0.5) * aspect, uv.y + drift * 0.18) + pointer * vec2<f32>(0.05, 0.03);
        colour += rays(q, time, hue) * 0.1;
        colour += pools(vec2<f32>((uv.x - 0.5) * aspect, uv.y) + pointer * vec2<f32>(0.04, 0.02), aspect, time, drift, hue) * 0.13;

        // A haze where the subject hangs, as if it lit the air round it.
        let haze_at = vec2<f32>(0.0, b.tone.z - drift * 0.55);
        let h = (vec2<f32>((uv.x - 0.5) * aspect, uv.y) - haze_at) * vec2<f32>(1.0 / b.tone.w, 3.2);
        let haze = exp(-dot(h, h)) * (0.035 + b.tone.y * 0.12);
        colour += haze * mix(vec3<f32>(1.0), spectrum(hue), 0.35);

        // Deep colour pooling low on the screen, far away.
        let pool = fbm(vec2<f32>(uv.x * aspect * 1.4 + time * 0.012, uv.y * 1.2 - drift * 0.25 - time * 0.008));
        let low = smoothstep(0.35, 1.25, uv.y + drift * 0.1 + pool * 0.35);
        colour += spectrum(fract(hue + pool * 0.35 - 0.1)) * low * pool * pool * 0.05;

        // Dust, its colours pulled apart by speed.
        let split = clamp(velocity * 0.004, -6.0, 6.0);
        let dust_rgb = vec3<f32>(
            dust_field(css + vec2<f32>(0.0, split), scroll, velocity, pointer, time),
            dust_field(css, scroll, velocity, pointer, time),
            dust_field(css - vec2<f32>(0.0, split), scroll, velocity, pointer, time),
        );
        colour += dust_rgb * 0.33;

        colour *= lit;
    }

    // The light layer, added: black adds nothing, so the world shows
    // through everywhere it is not.
    if (b.pointer.z > 0.0) {
        let offset = (uv - vec2<f32>(0.5)) * b.pointer.w;
        let light_rgb = vec3<f32>(
            textureSampleLevel(light, clamped, uv + offset, 0.0).r,
            textureSampleLevel(light, clamped, uv, 0.0).g,
            textureSampleLevel(light, clamped, uv - offset, 0.0).b,
        );
        colour += light_rgb * b.pointer.z;
    }

    // A vignette, and grain so gradients do not band.
    let v = (uv - vec2<f32>(0.5)) * vec2<f32>(aspect, 1.0);
    colour *= 1.0 - 0.28 * lit * dot(v, v);
    let grain = hash21(in.clip.xy + fract(time * 13.7) * 91.0) - 0.5;
    colour += grain * 0.012 * lit;
    colour = clamp(colour, vec3<f32>(0.0), vec3<f32>(1.0));

    if (b.screen.w > 0.5) {
        colour = to_linear(colour);
    }
    return vec4<f32>(colour, 1.0);
}
