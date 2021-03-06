#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
#![deny(warnings)]

#[cfg(feature = "wgpu")]
pub mod pipeline;



use shared::{CameraUniform, LightUniform, LightsUniform};
use spirv_std::glam::Vec4Swizzles;
use spirv_std::glam::{mat3, mat4, vec3, Mat3, Vec2, Vec3, Vec4};
use spirv_std::num_traits::Float;
use spirv_std::Image;
use spirv_std::Sampler;

type Image2d = Image!(2D, type=f32, sampled);

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;


#[spirv(vertex)]
pub fn main_vs(
    // Vertex data
    position: Vec3,
    uv: Vec2,
    _color: Vec4,
    normal: Vec3,
    tangent: Vec3,
    bitangent: Vec3,
    // Instance data
    model_matrix_0: Vec4,
    model_matrix_1: Vec4,
    model_matrix_2: Vec4,
    model_matrix_3: Vec4,
    normal_matrix_0: Vec3,
    normal_matrix_1: Vec3,
    normal_matrix_2: Vec3,
    #[spirv(descriptor_set = 1, binding = 0, uniform)] camera: &CameraUniform,
    #[spirv(position, invariant)] clip_position: &mut Vec4,
    out_uv: &mut Vec2,
    out_tangent_position: &mut Vec3,
    out_tangent_view_position: &mut Vec3,
    out_world_normal: &mut Vec3,
    out_world_tangent: &mut Vec3,
    out_world_bitangent: &mut Vec3,
) {
    let model_matrix = mat4(
        model_matrix_0,
        model_matrix_1,
        model_matrix_2,
        model_matrix_3,
    );
    let normal_matrix = mat3(normal_matrix_0, normal_matrix_1, normal_matrix_2);
    let world_normal = (normal_matrix * normal).normalize();
    let world_tangent = (normal_matrix * tangent).normalize();
    let world_bitangent = (normal_matrix * bitangent).normalize();
    let tangent_matrix = Mat3::transpose(&mat3(world_tangent, world_bitangent, world_normal));
    let world_position = model_matrix * Vec3::extend(position, 1.0);

    *clip_position = camera.view_proj * world_position;
    *out_uv = uv;

    *out_tangent_position = tangent_matrix * world_position.xyz();
    *out_tangent_view_position = tangent_matrix * camera.view_pos.xyz();
    *out_world_normal = world_normal;
    *out_world_tangent = world_tangent;
    *out_world_bitangent = world_bitangent;
}

#[spirv(fragment)]
pub fn main_fs(
    uv: Vec2,
    tangent_position: Vec3,
    tangent_view_position: Vec3,
    world_normal: Vec3,
    world_tangent: Vec3,
    world_bitangent: Vec3,
    #[spirv(descriptor_set = 0, binding = 0)] t_diffuse: &Image2d,
    #[spirv(descriptor_set = 0, binding = 1)] s_diffuse: &Sampler,
    #[spirv(descriptor_set = 0, binding = 2)] t_normal: &Image2d,
    #[spirv(descriptor_set = 0, binding = 3)] s_normal: &Sampler,
    #[spirv(descriptor_set = 2, binding = 0, uniform)] lights: &LightsUniform,
    output: &mut Vec4,
) {
    let object_color: Vec4 = t_diffuse.sample(*s_diffuse, uv);
    let object_normal: Vec4 = t_normal.sample(*s_normal, uv);

    let tangent_matrix = Mat3::transpose(&mat3(world_tangent, world_bitangent, world_normal));
    let tangent_normal = object_normal.xyz() * 2.0 - 1.0;

    let ambient_strength = 0.05;
    let mut total_lighting_color: Vec3 = vec3(1.0, 1.0, 1.0) * ambient_strength;

    let mut i = 0_usize;

    while i < min_usize(lights.size as usize, LightsUniform::MAX_LIGHTS) {
        let light: &LightUniform = &lights.data[i];

        let tangent_light_position = tangent_matrix * light.position.xyz();

        let light_dir = (tangent_light_position - tangent_position).normalize();
        let light_distance = (tangent_light_position - tangent_position).length();
        let light_intencity = smoothstep(light.radius.x, 0.0, light_distance);
        let view_dir = (tangent_view_position - tangent_position).normalize();
        let half_dir = (view_dir + light_dir).normalize();

        let diffuse_strength = (tangent_normal.dot(light_dir) * light_intencity).max(0.0);
        let specular_strength = Float::powf(
            (tangent_normal.dot(half_dir) * light_intencity).max(0.0),
            32.0,
        );

        total_lighting_color =
            total_lighting_color + (diffuse_strength + specular_strength) * light.color.xyz();

        i += 1;
    }

    let result_color = total_lighting_color * object_color.xyz();
    *output = result_color.extend(object_color.w);
}

pub fn saturate(x: f32) -> f32 {
    x.max(0.0).min(1.0)
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    // Scale, bias and saturate x to 0..1 range
    let x = saturate((x - edge0) / (edge1 - edge0));
    // Evaluate polynomial
    x * x * (3.0 - 2.0 * x)
}

fn min_usize(a: usize, b: usize) -> usize {
    if a <= b {
        a
    } else {
        b
    }
}
