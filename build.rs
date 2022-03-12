use anyhow::*;
use fs_extra::{copy_items, dir::CopyOptions};
use spirv_builder::SpirvBuilder;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    // This tells cargo to rerun this script if something in /res/ changes.
    println!("cargo:rerun-if-changed=res/*");
    println!("cargo:rerun-if-changed=src/shaders/*");

    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push("res/");
    copy_items(&paths_to_copy, out_dir, &copy_options)?;

    // @TODO: find a way to build all shaders from directory
    build_shader("shaders/model", true)?;
    build_shader("shaders/texture", true)?;

    Ok(())
}

fn build_shader(path_to_crate: &str, codegen_names: bool) -> Result<()> {
    let builder_dir = &Path::new(env!("CARGO_MANIFEST_DIR"));
    let path_to_crate = builder_dir.join(path_to_crate);
    let result = SpirvBuilder::new(path_to_crate, "spirv-unknown-vulkan1.2").build()?;
    if codegen_names {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join("entry_points.rs");
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(&dest_path, result.codegen_entry_point_strings()).unwrap();
    }
    Ok(())
}
