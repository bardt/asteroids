struct Camera {
    view_pos: vec4<f32>;
    view_proj: mat4x4<f32>;
};

// We skip material, which is usually set in group(0)
[[group(0), binding(0)]]
var<uniform> camera: Camera;

struct Light {
    position: vec4<f32>;
    color: vec4<f32>;
    radius: vec4<f32>;
};
let max_lights: u32 = 16u;
struct LightBuffer {
    data: array<Light, 16>;
    size: u32; 
};
[[group(1), binding(0)]]
var<uniform> lights: LightBuffer;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec3<f32>;
};

[[stage(vertex)]]
fn main(
    model: VertexInput,
) -> VertexOutput {
    let light = lights.data[0];
    let scale = 0.25;
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position * scale + light.position.xyz, 1.0);
    out.color = light.color.xyz;
    return out;
}

[[stage(fragment)]]
fn main_fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}