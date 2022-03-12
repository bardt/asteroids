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
pub fn main_vs(
    pos: Vec4,
    uv: Vec2,
    color: Vec4,
    #[spirv(position)] builtin_pos: &mut Vec4,
    out_uv: &mut Vec2,
    out_color: &mut Vec4,
) {
    *builtin_pos = vec4(pos.x, pos.y, pos.z, 1.0);
    *out_uv = uv;
    *out_color = color;
}

#[spirv(fragment)]
pub fn main_fs(
    uv: Vec2,
    color: Vec4,
    #[spirv(descriptor_set = 0, binding = 0)] t_diffuse: &Image2d,
    #[spirv(descriptor_set = 0, binding = 1)] s_diffuse: &Sampler,
    output: &mut Vec4,
) {
    let texture_color: Vec4 = t_diffuse.sample(*s_diffuse, uv);
    *output = blend(color, texture_color);
}

fn blend(base: Vec4, added: Vec4) -> Vec4 {
    let alpha = 1. - (1. - added.w) * (1. - base.w); // alpha
    if alpha == 0. {
        base
    } else {
        Vec4::new(
            (added.x * added.w / alpha) + (base.x * base.w * (1. - added.w) / alpha),
            (added.y * added.w / alpha) + (base.y * base.w * (1. - added.w) / alpha),
            (added.z * added.w / alpha) + (base.z * base.w * (1. - added.w) / alpha),
            alpha,
        )
    }
}
