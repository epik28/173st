struct Camera { view_projection: mat4x4<f32>, };
override OUTPUT_SRGB: u32 = 0u;
@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;
@group(0) @binding(2) var<uniform> camera: Camera;

fn srgb_to_linear(encoded: vec3<f32>) -> vec3<f32> {
    let low = encoded / vec3<f32>(12.92);
    let high = pow((encoded + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    return select(low, high, encoded > vec3<f32>(0.04045));
}

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) vertex_color: vec3<f32>,
};
struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) vertex_color: vec3<f32>,
};
@vertex fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = camera.view_projection * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.vertex_color = input.vertex_color;
    return out;
}
@fragment fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(atlas, atlas_sampler, input.uv);
    if color.a <= 0.1 { discard; }
    // Native encoded RGB is packed per vertex before this single texture multiply.
    let native_rgb = color.rgb * input.vertex_color;
    if OUTPUT_SRGB != 0u {
        return vec4<f32>(srgb_to_linear(native_rgb), 1.0);
    }
    return vec4<f32>(native_rgb, 1.0);
}
@fragment fn fs_fluid(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(atlas, atlas_sampler, input.uv);
    let native_rgb = color.rgb * input.vertex_color;
    if OUTPUT_SRGB != 0u {
        return vec4<f32>(srgb_to_linear(native_rgb), color.a);
    }
    return vec4<f32>(native_rgb, color.a);
}
