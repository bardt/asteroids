struct Backdrop {
    color: vec4<f32>;
};

[[group(0), binding(0)]]
var<uniform> backdrop: Backdrop;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn main(
    model: VertexInput,
) -> [[builtin(position)]] vec4<f32> {
    return vec4<f32>(model.position, 1.0);
}

[[stage(fragment)]]
fn main_fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return backdrop.color;
}