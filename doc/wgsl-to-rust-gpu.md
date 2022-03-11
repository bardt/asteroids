---
title: Notes on migrating from WGSL to rust-gpu shaders
published: true
description: Problems I faced and workarounds I found
tags: rust, rustgpu, wgsl
//cover_image: https://direct_url_to_image.jpg
--- 
## Preface

You probably ended up here through googling one of the weird error messages mentioned below. I also tried googling them with limited success, so I decided to post this to save time to anyone in the same situation.

I develop a small game on top of the project structure from [Learn WGPU guide](https://sotrh.github.io/learn-wgpu/). I found WGSL shaders tedious to write, though, since text editor support is non-existent, and it's hard to find examples and precise documentation. Once I learned about [rust-gpu](https://github.com/EmbarkStudios/rust-gpu) approach to writing shaders in plain Rust code, I instantly decided to try it out and to migrate my existing WGSL shaders. Below goes several obstacles I faced and learnings I took away.

`rust-gpu` is in active development, so most things here might change in future versions. For reference, here are the versions I'm using at the moment:

- rust edition `2021`
- toolchain `nightly-2022-01-13`
- spirv-std `0.4.0-alpha.12`

### Implicit locations

In WGSL, you explicitly specify locations for position, texture coordinates etc. This is how input parameters in my WGSL shader for displaying textured models looked like:

```wgsl
struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
    [[location(3)]] tangent: vec3<f32>;
    [[location(4)]] bitangent: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] tangent_position: vec3<f32>;
    [[location(2)]] tangent_view_position: vec3<f32>;
};

...

[[stage(vertex)]]
fn main(model: VertexInput) -> VertexOutput {
    ...
}
```

It took me time to figure out how to represent a similar set of input/output parameters in `rust-gpu`. It turns out the order of arguments in the shader function maps to the location index. It is more evident with input parameters, though. Output parameters are all `&mut` arguments; as far as I understand, their locations are resolved in order of occurrence except for the ones marked explicitly to built-in meanings (like `#[spirv(position, invariant)]` in the code below).

```rust
#[spirv(vertex)]
pub fn main_vs(
    position: Vec3, // implicit input Location 0
    uv: Vec2,       // implicit input Location 1
    normal: Vec3,   // implicit input Location 2 etc.
    tangent: Vec3,
    bitangent: Vec3,
    ...
    #[spirv(position, invariant)] out_position: &mut Vec4, // builtin position
    out_uv: &mut Vec2,                      // implicit output Location 0
    out_tangent_position: &mut Vec3,        // implicit output Location 1
    out_tangent_view_position: &mut Vec3    // implicit output Location 2
    )
```

### From indexes to Glam methods

In the WGSL shader, all vector arguments had built-in type `vec3<f32>`, and you could access elements by index. Naively, I copied the code into the `rust-gpu` shader, using `glam::Vec3` as an input type. 

```rust
use spirv_std::glam::Vec3;

#[spirv(vertex)]
pub fn main_vs(
    position: Vec3,
    ...
) {
    let x = position[0];
}
```

This code causes a confusing compiler error:


```
error: Using pointers with OpPhi requires capability VariablePointers or VariablePointersStorageBuffer
           %184 = OpPhi %_ptr_Function_float %181 %172 %182 %173 %183 %174
    |
    = note: module `/Users/bardt/Projects/rust/asteroids/target/spirv-builder/spirv-unknown-vulkan1.2/release/deps/model.spv.dir/module`
```

The remedy is to be careful while copy-pasting the code between shaders and consider the new data structures used. In Glam, you access elements via named properties:

```rust
use spirv_std::glam::Vec3;

#[spirv(vertex)]
pub fn main_vs(
    position: Vec3,
    ...
) {
    let x = position.x; // use property instead
}
```

### Matrix parameters

One thing I couldn't make work in WGSL and hoped to get in `rust-gpu` is passing matrix parameters. I hoped this to work:

```rust
#[spirv(vertex)]
pub fn main_vs(
    position: Vec3,
    uv: Vec2,
    normal: Vec3,
    tangent: Vec3,
    bitangent: Vec3,
    model_matrix: Mat4, // matrix parameter
    ...
) { 
    ...
}
```

Unfortunately, magic didn't happen. Instead, I got a very confusing error. I don't fully understand why the location is 6 while I expected it to be 5. Now I know that this roughly translates to "there is something wrong with your arguments".

```
error: Entry-point has conflicting input location assignment at location 6, component 0
    OpEntryPoint Vertex %2 "main_vs" %position %uv %normal %tangent %bitangent %model_matrix ...
```

I had to switch back to passing each matrix column as a separate vector and then combining them in the shader body. The same applies to passing arguments between vertex and fragment shader.

```rust
use spirv_std::glam::mat4;

#[spirv(vertex)]
pub fn main_vs(
    position: Vec3,
    uv: Vec2,
    normal: Vec3,
    tangent: Vec3,
    bitangent: Vec3,
    model_matrix_0: Vec4,
    model_matrix_1: Vec4,
    model_matrix_2: Vec4,
    model_matrix_3: Vec4,
    ...
) {
    let model_matrix = mat4(
        model_matrix_0,
        model_matrix_1,
        model_matrix_2,
        model_matrix_3,
    );
    ...
}
```

### for loops

Another weird thing I noticed is `for` loops are not working. The shader compiles fine and passes validation, but pixels are not drawn on the screen, so I assume the code doesn't reach the final line.

```rust
for i in 0..lights_number {
    // do stuff
}

*output = result_color;
```

After refactoring to a `while` loop, everything works just fine.

```rust
let mut i = 0_usize;    
while i < lights_number {
    // do stuff
}
```

### min/max on ints

One more surprise: the `min` and `max` methods do not work on integers while working fine on floats. 

```rust
let lights_number: usize = lights.size.min(MAX_LIGHTS);
```

The error message gives a slight hint on the roots of the problem but doesn't help much in solving it:

```
error: u8 without OpCapability Int8
     --> ~/.rustup/toolchains/nightly-2022-01-13-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/cmp.rs:850:5
      |
  850 |     fn partial_cmp(&self, other: &Ordering) -> Option<Ordering> {
      |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      |
      = note: Stack:
              core::cmp::min_by::<usize, <usize as core::cmp::Ord>::cmp>
              <usize as core::cmp::Ord>::min
              model::main_fs
              main_fs
```

I decided not to spend too much time on finding the root causes (here is a [related issue](https://github.com/EmbarkStudios/rust-gpu/issues/758)), and rewrote that comparison by hand.

```rust
fn min_usize(a: usize, b: usize) -> usize {
    if a <= b {
        a
    } else {
        b
    }
}

let lights_number: usize = min_usize(lights.size, MAX_LIGHTS);

```

## Conclusion

`rust-gpu` concept and vision has the potential to flip the game in writing testable, maintainable, reusable shader code. Still, it is in alpha, has a lot of minor issues and inconveniences, and you should seriously evaluate if you want to spend time on those in a project with a deadline and requirements for performance and stability. However, all of this is not the case for my pet project game, so I keep living on the edge and look forward to a bright future. 