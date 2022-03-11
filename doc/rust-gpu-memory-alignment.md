---
title: Sharing types between WGPU code and rust-gpu shaders
published: true
description: Writing to buffers with bytemuck and fixing memory layout issues
tags: rust rustgpu wgpu spirv
//cover_image: https://direct_url_to_image.jpg
---

> If you find the code here confusing, you might want to start with [Learn WGPU](https://sotrh.github.io/learn-wgpu/) guide and then read my [previous post](https://dev.to/bardt/notes-on-migrating-from-wgsl-to-rust-gpu-shaders-56bg). 

Last time I finished the upgrade of my WGSL shaders to [rust-gpu]. One of its reasons is the ability to reuse types and methods between your GPU and CPU code. It would help keep uniform data structures in sync on both ends of the GPU buffers.

I picked a uniform describing my light sources for a first try. Here, I want to use the type imported from the shader crate to describe the data I write into the GPU buffer:

```rust
use my_shader::{LightUniform, LightsUniform};

let lights: LightsUniform;

// ... lights initialization ...

let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
        label: Some("Light Buffer"),
        contents: bytemuck::cast_slice(&[lights]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
```

In the shader crate, I have:
1.`spirv-std` for compiling shaders with enabled `glam` feature;
2.`glam` for the shader-compatible vectors and matrices with enabled `bytemuck` feature;
3.`bytemuck` for representing data structures as a slice of bytes, so we can write into buffers: 

```toml
[package]
name = "my-shader"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["lib", "dylib"]

[dependencies]
spirv-std = { git = "https://github.com/EmbarkStudios/rust-gpu", features = ["glam"] }
glam = { version = "0.20.2", default-features = false, features = ["libm", "bytemuck"] }
bytemuck = { version = "1.4", features = [ "derive" ] }
```

In the shader, I have uniform structures defining the light sources. I add Pod and Zeroable traits to the uniform data struct to represent them in a raw format suitable for the buffer via `bytemuck::cast_slice`. 

```rust
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LightUniform {
    position: Vec4,
    color: Vec4,
    radius: Vec4,
}

const MAX_LIGHTS: usize = 16;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LightsUniform {
    data: [LightUniform; MAX_LIGHTS],
    size: usize,
}
```

However, adding those traits is not enough to make it work. The code above results in a compiler error: 

```
cannot transmute between types of different sizes, or dependently-sized types
source type: `LightsUniform` (6272 bits)
target type: `_::{closure#0}::TypeWithoutPadding` (6208 bits)
```

As outlined in the chapter on [WGSL memory layout](https://sotrh.github.io/learn-wgpu/showcase/alignment/#memory-layout-in-wgsl), we need to explicitly add padding to make the memory size aligned to 16. 

`LightUniform` doesn't need any modifications, as it uses Vec4, which occupies the whole 16 bytes, and there is no tail left. So is the `data` field in the `LightsUniform` type. 

But then it follows with the `size` field. In my case, `usize` occupies 32 bits, equal to 4 bytes, so I need to reserve 12 more bytes in the data structure to add to 16. Usually, I would do it like that:

```rust
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LightsUniform {
    data: [LightUniform; MAX_LIGHTS],
    size: usize,
    _padding: [usize; 3],              
}
```

But the shader compiler doesn't like it for some reason. It looks at the `[usize; 3]` on its own and complains that it's not aligned with 16. 

```
error: Structure id 60 decorated as Block for variable in Uniform storage class must follow relaxed uniform buffer layout rules: member 2 at offset 772 is not aligned to 16
           %LightsUniform = OpTypeStruct %_arr_LightUniform_uint_16 %uint %_arr_uint_uint_3
```
> In the error above "member 2" is `%_arr_uint_uint_3`, which stands for our `_padding` field.

A weird but working solution is to split padding into independent fields, each sized after a power of 2 bytes.

```rust
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LightsUniform {
    data: [LightUniform; MAX_LIGHTS],
    size: usize,
    _padding1: usize,
    _padding2: usize,
    _padding3: usize,
}
```

With this final fix, both Rust and SPIR-V compilers are satisfied. I can now use the same type to write to the GPU buffer and to read from it.