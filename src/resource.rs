use crate::{
    model::{Material, Mesh, Model},
    shaders::Shaders,
    texture,
};
use anyhow::*;

pub struct Resources {
    pub shaders: Shaders,
    pub obj_model: Model,
}

impl Resources {
    pub fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Result<Self> {
        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");

        let texture_bind_group_layout = device.create_bind_group_layout(&texture::Texture::desc());

        let obj_model = Model::load(
            device,
            queue,
            &texture_bind_group_layout,
            res_dir.join("assets.obj"),
        )?;

        let shaders = Shaders::init(device, color_format, depth_format);

        Ok(Self { shaders, obj_model })
    }

    pub fn get_mesh(&self, name: &str) -> Option<&Mesh> {
        self.obj_model.meshes.iter().find(|mesh| mesh.name == name)
    }

    pub fn get_mesh_material(&self, mesh: &Mesh) -> &Material {
        &self.obj_model.materials[mesh.material]
    }
}
