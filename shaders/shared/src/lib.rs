#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]

#[cfg(feature = "wgpu")]
pub mod wgpu;

use bytemuck::{Pod, Zeroable};
use spirv_std::glam::{vec4, Mat4, Vec4};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_pos: Vec4,
    pub view_proj: Mat4,
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_pos: Vec4::ZERO,
            view_proj: Mat4::IDENTITY,
        }
    }

    pub fn update(&mut self, pos: [f32; 4], proj: &[[f32; 4]; 4]) {
        self.view_pos = pos.into();
        self.view_proj = Mat4::from_cols_array_2d(proj);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LightUniform {
    pub position: Vec4,
    pub color: Vec4,
    pub radius: Vec4,
}

impl LightUniform {
    pub fn new(position: [f32; 3], color: [f32; 3], radius: f32) -> Self {
        Self {
            position: vec4(position[0], position[1], position[2], 0.),
            color: vec4(color[0], color[1], color[2], 0.),
            radius: vec4(radius, 0., 0., 0.),
        }
    }

    pub fn empty() -> Self {
        Self::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.0)
    }
}


const MAX_LIGHTS: usize = 16;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LightsUniform {
    pub data: [LightUniform; MAX_LIGHTS],
    pub size: usize,
    _padding1: usize,
    _padding2: usize,
    _padding3: usize,
}

impl LightsUniform {
    pub const MAX_LIGHTS: usize = MAX_LIGHTS;

    pub fn new(lights: &[LightUniform]) -> Self {
        let mut data = [LightUniform::empty(); MAX_LIGHTS];
        for i in 0..lights.len().min(MAX_LIGHTS) {
            data[i] = lights[i];
        }

        Self {
            data,
            size: lights.len(),
            _padding1: 0,
            _padding2: 0,
            _padding3: 0,
        }
    }
}