[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
};


[[stage(vertex)]]
fn main(
    vertex: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(vertex.position, 1.0);
    out.tex_coords = vertex.tex_coords;
    return out;
}

[[stage(fragment)]]
fn main_fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return object_color;
}
