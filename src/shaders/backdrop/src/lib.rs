#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
#![deny(warnings)]

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

use spirv_std::glam::Vec4;

#[spirv(vertex)]
pub fn main_vs(pos: Vec4, #[spirv(position)] builtin_pos: &mut Vec4) {
    *builtin_pos = pos;
}

pub struct Backdrop {
    color: Vec4,
}

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] backdrop: &Backdrop,
    output: &mut Vec4,
) {
    *output = backdrop.color;
}
