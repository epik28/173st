struct Camera { view_projection: mat4x4<f32>, };
@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;
@group(0) @binding(2) var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) shade: f32,
    @location(3) saved_light: f32,
};
struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) shade: f32,
    @location(2) saved_light: f32,
};
@vertex fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = camera.view_projection * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.shade = input.shade;
    out.saved_light = input.saved_light;
    return out;
}
@fragment fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(atlas, atlas_sampler, input.uv);
    // This bounded approximation uses saved nibble light. It does not reproduce
    // native packed uu.d brightness conversion or per-corner ambient occlusion.
    return vec4<f32>(color.rgb * input.shade * input.saved_light, color.a);
}
