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
struct HudVertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
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
@vertex fn vs_hud(input: HudVertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = vec4<f32>(input.position.xy, 0.0, 1.0);
    out.uv = input.uv;
    out.vertex_color = vec3<f32>(1.0);
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
@fragment fn fs_crack(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(atlas, atlas_sampler, input.uv);
    return vec4<f32>(color.rgb * input.vertex_color, color.a);
}
@fragment fn fs_hud(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(atlas, atlas_sampler, input.uv);
    if color.a <= 0.0 { discard; }
    if OUTPUT_SRGB != 0u {
        return vec4<f32>(srgb_to_linear(color.rgb), color.a);
    }
    return color;
}
@fragment fn fs_hud_shadow(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(atlas, atlas_sampler, input.uv);
    if color.a <= 0.0 { discard; }
    return vec4<f32>(0.2470588, 0.2470588, 0.2470588, color.a);
}

struct SolidHudIn {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
};
struct SolidHudOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};
@vertex fn vs_solid_hud(input: SolidHudIn) -> SolidHudOut {
    var out: SolidHudOut;
    out.clip = vec4<f32>(input.position, 0.0, 1.0);
    out.color = input.color;
    return out;
}
@fragment fn fs_solid_hud(input: SolidHudOut) -> @location(0) vec4<f32> {
    if OUTPUT_SRGB != 0u { return vec4<f32>(srgb_to_linear(input.color), 1.0); }
    return vec4<f32>(input.color, 1.0);
}
