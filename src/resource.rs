use anyhow::*;

use crate::{
    model::{Material, Mesh, Model},
    texture,
};

pub struct Resources {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Resources {
    pub const ZERO: Self = Resources {
        meshes: vec![],
        materials: vec![],
    };

    pub fn load(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");

        let texture_bind_group_layout = device.create_bind_group_layout(&texture::Texture::desc());

        let model = Model::load(
            device,
            queue,
            &texture_bind_group_layout,
            res_dir.join("assets.obj"),
        )?;

        let meshes = model.meshes;
        let materials = model.materials;

        Ok(Self { meshes, materials })
    }

    pub fn get_mesh_by_name(&self, name: &str) -> Option<(usize, &Mesh)> {
        self.meshes.iter().enumerate().find_map(|(id, mesh)| {
            if mesh.name == name {
                Some((id, mesh))
            } else {
                None
            }
        })
    }
}
