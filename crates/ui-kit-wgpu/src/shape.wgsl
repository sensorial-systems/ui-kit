struct Params {
    rect: vec4<f32>,
    fill: vec4<f32>,
    stroke: vec4<f32>,
    viewport_radius_blur: vec4<f32>,
    effect: vec4<f32>,
    gradient_end: vec4<f32>,
}
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var backdrop: texture_2d<f32>;
@group(0) @binding(2) var original_backdrop: texture_2d<f32>;
fn gaussian(pixel:vec2<i32>,axis:vec2<i32>)->vec4<f32>{
    let dimensions=vec2<i32>(textureDimensions(backdrop));
    let sigma=clamp(params.viewport_radius_blur.w,0.,32.);
    if sigma<0.5{return textureLoad(backdrop,clamp(pixel,vec2(0),dimensions-1),0);}
    let radius=i32(ceil(sigma*2.));
    var sum=vec4(0.);var weight_sum=0.;
    for(var offset=-radius;offset<=radius;offset++){
        let weight=exp(-f32(offset*offset)/(2.*sigma*sigma));
        sum+=textureLoad(backdrop,clamp(pixel+axis*offset,vec2(0),dimensions-1),0)*weight;
        weight_sum+=weight;
    }
    return sum/weight_sum;
}
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) local: vec2<f32> }
@vertex fn vs(@builtin(vertex_index) index: u32) -> Vertex {
    let corners = array<vec2<f32>, 6>(vec2(0.,0.), vec2(1.,0.), vec2(0.,1.), vec2(0.,1.), vec2(1.,0.), vec2(1.,1.));
    let local = corners[index] * params.rect.zw;
    let pixel = params.rect.xy + local;
    var result: Vertex;
    result.position = vec4(pixel.x / params.viewport_radius_blur.x * 2. - 1., 1. - pixel.y / params.viewport_radius_blur.y * 2., 0., 1.);
    result.local = local;
    return result;
}
@fragment fn fs(vertex: Vertex) -> @location(0) vec4<f32> {
    if params.effect.w>0.5{return gaussian(vec2<i32>(vertex.position.xy),vec2(1,0));}
    let half_size = params.rect.zw * .5;
    let radius = clamp(params.viewport_radius_blur.z, 0., min(half_size.x, half_size.y));
    let q = abs(vertex.local-half_size) - half_size + radius;
    let distance = length(max(q, vec2(0.))) + min(max(q.x,q.y),0.) - radius;
    let aa = max(fwidth(distance), .5);
    let coverage = 1. - smoothstep(-aa, 0., distance);
    let border = smoothstep(-params.effect.y-aa, -params.effect.y+aa, distance);
    let t = (vertex.local.x + vertex.local.y) / (params.rect.z + params.rect.w);
    let fill = mix(params.fill, params.gradient_end, t * params.effect.z);
    var color = mix(fill, params.stroke, border * select(0., 1., params.effect.y > 0.));
    if params.effect.x > .5 {
        let dimensions = vec2<i32>(textureDimensions(backdrop));
        let pixel = vec2<i32>(vertex.position.xy);
        let blurred=gaussian(pixel,vec2(0,1));
        let alpha = color.a + blurred.a * (1.-color.a);
        // Glass replaces the sampled backdrop, avoiding blending it a second time.
        let composed = vec4(color.rgb*color.a + blurred.rgb*(1.-color.a), alpha);
        let original = textureLoad(original_backdrop, clamp(pixel, vec2(0), dimensions-1), 0);
        return mix(original, composed, coverage);
    }
    return vec4(color.rgb, color.a*coverage);
}
