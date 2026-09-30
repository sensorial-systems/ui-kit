// Glass: what is behind a panel, blurred as through frosted glass and bent
// at its rounded edges as through a thick lens, each colour bent a little
// differently -- the split of light a prism makes -- with a rim of light
// round the edge and a highlight where the pointer is.
//
// Drawn in two steps. What is behind is copied out and halved again and
// again (`fs_down`), each level blurring more; then the panel is drawn from
// the copy and one of the levels (`fs_glass`).

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

// ------------------------------------------------------------ the blur

struct Down {
    // 1 / the source's size, then 1 / the destination's.
    texel: vec4<f32>,
    // Where the source holds what is behind the panel, in its uv.
    region: vec4<f32>,
};

@group(0) @binding(0) var down_source: texture_2d<f32>;
@group(0) @binding(1) var down_sampler: sampler;
@group(0) @binding(2) var<uniform> down: Down;

fn tap(uv: vec2<f32>) -> vec3<f32> {
    let at = clamp(uv, down.region.xy, down.region.zw);
    return textureSampleLevel(down_source, down_sampler, at, 0.0).rgb;
}

// Thirteen taps weighted as Jimenez's bloom does: halving without shimmer.
@fragment
fn fs_down(in: Out) -> @location(0) vec4<f32> {
    let uv = in.clip.xy * down.texel.zw;
    let t = down.texel.xy;
    let a = tap(uv + t * vec2<f32>(-2.0, -2.0));
    let b = tap(uv + t * vec2<f32>(0.0, -2.0));
    let c = tap(uv + t * vec2<f32>(2.0, -2.0));
    let d = tap(uv + t * vec2<f32>(-2.0, 0.0));
    let e = tap(uv);
    let f = tap(uv + t * vec2<f32>(2.0, 0.0));
    let g = tap(uv + t * vec2<f32>(-2.0, 2.0));
    let h = tap(uv + t * vec2<f32>(0.0, 2.0));
    let i = tap(uv + t * vec2<f32>(2.0, 2.0));
    let j = tap(uv + t * vec2<f32>(-1.0, -1.0));
    let k = tap(uv + t * vec2<f32>(1.0, -1.0));
    let l = tap(uv + t * vec2<f32>(-1.0, 1.0));
    let m = tap(uv + t * vec2<f32>(1.0, 1.0));
    let colour = e * 0.125 + (a + c + g + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
    return vec4<f32>(colour, 1.0);
}

// ------------------------------------------------------------ the glass

struct Glass {
    // The panel: left, top, width, height, in pixels.
    rect: vec4<f32>,
    // Corner radius, how wide the bent rim is, how far it bends light, how
    // much more red bends than blue.
    shape: vec4<f32>,
    // How frosted (0 clear, 1 all blur), how much the tint covers, how
    // bright the rim is, the panel's opacity.
    look: vec4<f32>,
    // Where the highlight is, in pixels, how bright, how wide.
    light: vec4<f32>,
    // Where the copy holds what is behind, in pixels: left, top, right, bottom.
    region: vec4<f32>,
    // Target width and height, 1 / the blur level's size.
    screen: vec4<f32>,
    // Tint colour, and how much of the rim is split into a spectrum.
    tint: vec4<f32>,
    // How much the whole panel magnifies, the time, whether the target
    // stores linear light, how bright the panel is over what is behind.
    extra: vec4<f32>,
};

@group(0) @binding(0) var sharp: texture_2d<f32>;
@group(0) @binding(1) var blurred: texture_2d<f32>;
@group(0) @binding(2) var glass_sampler: sampler;
@group(0) @binding(3) var<uniform> g: Glass;

fn spectrum(x: f32) -> vec3<f32> {
    let h = clamp(x, 0.0, 1.0) * 0.8;
    let rgb = clamp(abs(fract(h + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
    return rgb * rgb * (3.0 - 2.0 * rgb);
}

fn rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// What is behind at `px`, as frosted as the panel is: the sharp copy mixed
// with the blur level, read with a tent so its texels do not show.
fn behind(px: vec2<f32>) -> vec3<f32> {
    let at = clamp(px, g.region.xy + vec2<f32>(1.0), g.region.zw - vec2<f32>(1.0));
    let uv = at / g.screen.xy;
    let clear = textureSampleLevel(sharp, glass_sampler, uv, 0.0).rgb;
    let t = g.screen.zw * 0.5;
    let frosted = (textureSampleLevel(blurred, glass_sampler, uv + vec2<f32>(-t.x, -t.y), 0.0).rgb
        + textureSampleLevel(blurred, glass_sampler, uv + vec2<f32>(t.x, -t.y), 0.0).rgb
        + textureSampleLevel(blurred, glass_sampler, uv + vec2<f32>(-t.x, t.y), 0.0).rgb
        + textureSampleLevel(blurred, glass_sampler, uv + vec2<f32>(t.x, t.y), 0.0).rgb) * 0.25;
    return mix(clear, frosted, g.look.x);
}

fn to_linear(x: vec3<f32>) -> vec3<f32> {
    return select(pow((x + 0.055) / 1.055, vec3<f32>(2.4)), x / 12.92, x <= vec3<f32>(0.04045));
}

fn to_srgb(x: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(x, vec3<f32>(1.0 / 2.4)) - 0.055, x * 12.92, x <= vec3<f32>(0.0031308));
}

@fragment
fn fs_glass(in: Out) -> @location(0) vec4<f32> {
    let px = in.clip.xy;
    let half_size = g.rect.zw * 0.5;
    let centre = g.rect.xy + half_size;
    let p = px - centre;
    let radius = min(g.shape.x, min(half_size.x, half_size.y));
    let d = rounded_box(p, half_size, radius);
    let coverage = clamp(0.5 - d, 0.0, 1.0);
    if (coverage <= 0.0) {
        discard;
    }

    // Which way the edge faces, from how the distance changes.
    let e = 0.5;
    let n = normalize(vec2<f32>(
        rounded_box(p + vec2<f32>(e, 0.0), half_size, radius) - rounded_box(p - vec2<f32>(e, 0.0), half_size, radius),
        rounded_box(p + vec2<f32>(0.0, e), half_size, radius) - rounded_box(p - vec2<f32>(0.0, e), half_size, radius),
    ) + vec2<f32>(1e-6));
    let inside = -d;
    let bevel = max(g.shape.y, 1.0);
    // 1 at the edge, falling to 0 where the rim meets the flat face: the
    // steepness of a rounded edge.
    let slope = pow(1.0 - clamp(inside / bevel, 0.0, 1.0), 2.2);

    // The rim bends light in from beyond the edge; the face magnifies a
    // little. Each colour bends by its own amount.
    let bend = n * g.shape.z * slope - p * g.extra.x;
    let split = g.shape.w * (0.35 + slope);
    var colour = vec3<f32>(
        behind(px - bend * (1.0 + split)).r,
        behind(px - bend).g,
        behind(px - bend * (1.0 - split)).b,
    );

    let srgb = g.extra.z > 0.5;
    if (srgb) {
        colour = to_srgb(colour);
    }

    // Tint and a lift of brightness, as frosted glass scatters light.
    colour = mix(colour, g.tint.rgb, g.look.y) * g.extra.w;

    // A rim of light: bright where the edge faces the light, up and left,
    // and again, fainter, on the far side; split into colours round it.
    let facing = dot(n, normalize(vec2<f32>(-0.55, -1.0)));
    let rim_band = exp(-inside / 1.35);
    let rim = rim_band * (0.35 + 0.65 * max(facing, 0.0) + 0.3 * max(-facing, 0.0));
    // The colour runs round the rim smoothly, joining up with itself all
    // the way round: an angle wrapped into 0..1 would jump where the edge
    // faces straight left.
    let angle = atan2(n.y, n.x);
    let hue = 0.5 + 0.5 * sin(angle * 2.0 + 0.6 + g.extra.y * 0.2);
    let fringe = spectrum(hue) * exp(-inside / 3.5) * slope;
    colour += vec3<f32>(rim * g.look.z) + fringe * g.tint.w;

    // The highlight, where the pointer is.
    let to_light = (px - g.light.xy) / max(g.light.w, 1.0);
    colour += vec3<f32>(exp(-dot(to_light, to_light)) * g.light.z * (0.4 + 0.6 * (1.0 - clamp(inside / (bevel * 4.0), 0.0, 1.0))));

    colour = clamp(colour, vec3<f32>(0.0), vec3<f32>(1.0));
    if (srgb) {
        colour = to_linear(colour);
    }
    return vec4<f32>(colour, coverage * g.look.w);
}
