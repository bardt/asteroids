In wgpu code:

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

In shader crate:

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



In rust-gpu shader:

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

results in error:

```
cannot transmute between types of different sizes, or dependently-sized types
source type: `LightsUniform` (6272 bits)
target type: `_::{closure#0}::TypeWithoutPadding` (6208 bits)
```


Trying to add padding:


```rust
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LightsUniform {
    data: [LightUniform; MAX_LIGHTS],
    size: usize,
    _padding: [usize; 3],              
}
```

Results in a different error

```
error: Structure id 60 decorated as Block for variable in Uniform storage class must follow relaxed uniform buffer layout rules: member 2 at offset 772 is not aligned to 16
           %LightsUniform = OpTypeStruct %_arr_LightUniform_uint_16 %uint %_arr_uint_uint_3
```

"member 2" here is `%_arr_uint_uint_3`, standing for our `_padding`

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