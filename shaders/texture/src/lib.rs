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

use spirv_std::glam::{vec4, Vec2, Vec4};
use spirv_std::Image;
use spirv_std::Sampler;

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

type Image2d = Image!(2D, type=f32, sampled);

#[spirv(vertex)]
pub fn main_vs(pos: Vec4, uv: Vec2, #[spirv(position)] builtin_pos: &mut Vec4, out_uv: &mut Vec2) {
    *builtin_pos = vec4(pos.x, pos.y, pos.z, 1.0);
    *out_uv = uv;
}

#[spirv(fragment)]
pub fn main_fs(
    uv: Vec2,
    #[spirv(descriptor_set = 0, binding = 0)] t_diffuse: &Image2d,
    #[spirv(descriptor_set = 0, binding = 1)] s_diffuse: &Sampler,
    output: &mut Vec4,
) {
    *output = t_diffuse.sample(*s_diffuse, uv);
}
