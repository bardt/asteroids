use crate::texture;
use anyhow::*;
use cgmath::{InnerSpace, Vector2, Vector3};
use rayon::prelude::*;
use std::{ops::Range, path::Path};
use tobj::LoadOptions;
use wgpu::{util::DeviceExt, BindGroup};

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
    tangent: [f32; 3],
    bitangent: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
    pub bind_group: BindGroup,
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        diffuse_texture: texture::Texture,
        normal_texture: texture::Texture,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
            ],
            label: None,
        });

        Self {
            name: String::from(name),
            diffuse_texture,
            normal_texture,
            bind_group,
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path: P,
    ) -> Result<Self> {
        let (obj_models, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        let obj_materials = obj_materials?;

        // We're assuming that the texture files are stored with the obj file
        let containing_folder = path.as_ref().parent().context("Directory has no parent")?;

        let materials = obj_materials
            .par_iter()
            .map(|mat| {
                let diffuse_texture = texture::Texture::load(
                    device,
                    queue,
                    containing_folder.join(&mat.diffuse_texture),
                    false,
                )?;

                let normal_texture = if &mat.normal_texture == "" {
                    // If no normal texture is set, use a default one, matching diffuse texture in size
                    let mut raw_img = image::RgbImage::new(
                        diffuse_texture.size.width,
                        diffuse_texture.size.height,
                    );

                    for x in 0..diffuse_texture.size.width {
                        for y in 0..diffuse_texture.size.height {
                            raw_img.put_pixel(x, y, image::Rgb([128, 128, 255]));
                        }
                    }

                    let img = image::DynamicImage::ImageRgb8(raw_img);
                    texture::Texture::from_image(device, queue, &img, None, true)?
                } else {
                    texture::Texture::load(
                        device,
                        queue,
                        containing_folder.join(&mat.normal_texture),
                        true,
                    )?
                };

                Ok(Material::new(
                    device,
                    &mat.name,
                    diffuse_texture,
                    normal_texture,
                    layout,
                ))
            })
            .collect::<Result<Vec<Material>>>()?;

        let meshes = obj_models
            .par_iter()
            .map(|m| {
                let mut vertices = (0..m.mesh.positions.len() / 3)
                    .into_par_iter()
                    .map(|i| ModelVertex {
                        position: [
                            m.mesh.positions[i * 3],
                            m.mesh.positions[i * 3 + 1],
                            m.mesh.positions[i * 3 + 2],
                        ],
                        tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                        normal: [
                            m.mesh.normals[i * 3],
                            m.mesh.normals[i * 3 + 1],
                            m.mesh.normals[i * 3 + 2],
                        ],
                        tangent: [0.0; 3],
                        bitangent: [0.0; 3],
                    })
                    .collect::<Vec<_>>();

                let indices = &m.mesh.indices;
                let mut triangles_included = (0..vertices.len()).collect::<Vec<_>>();

                // Calculate tangents and bitangets. We're going to
                // use the triangles, so we need to loop through the
                // indices in chunks of 3
                for c in indices.chunks(3) {
                    let v0 = vertices[c[0] as usize];
                    let v1 = vertices[c[1] as usize];
                    let v2 = vertices[c[2] as usize];

                    let pos0: Vector3<_> = v0.position.into();
                    let pos1: Vector3<_> = v1.position.into();
                    let pos2: Vector3<_> = v2.position.into();

                    let uv0: Vector2<_> = v0.tex_coords.into();
                    let uv1: Vector2<_> = v1.tex_coords.into();
                    let uv2: Vector2<_> = v2.tex_coords.into();

                    // Calculate the edges of the triangle
                    let delta_pos1 = pos1 - pos0;
                    let delta_pos2 = pos2 - pos0;

                    // This will give us a direction to calculate the
                    // tangent and bitangent
                    // Note: in some cases all vertices have the same UV texture coordinates;
                    // In this case we fallback to a defailt delta vector
                    let delta_uv1 = if uv1 == uv0 {
                        Vector2::new(1.0, 0.0)
                    } else {
                        uv1 - uv0
                    };
                    let delta_uv2 = if uv2 == uv0 {
                        Vector2::new(0.0, 1.0)
                    } else {
                        uv2 - uv0
                    };

                    // Solving the following system of equations will
                    // give us the tangent and bitangent.
                    //     delta_pos1 = delta_uv1.x * T + delta_u.y * B
                    //     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
                    // Luckily, the place I found this equation provided
                    // the solution!
                    let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                    let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                    let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

                    // We'll use the same tangent/bitangent for each vertex in the triangle
                    vertices[c[0] as usize].tangent =
                        (tangent + cgmath::Vector3::from(vertices[c[0] as usize].tangent)).into();
                    vertices[c[1] as usize].tangent =
                        (tangent + cgmath::Vector3::from(vertices[c[1] as usize].tangent)).into();
                    vertices[c[2] as usize].tangent =
                        (tangent + cgmath::Vector3::from(vertices[c[2] as usize].tangent)).into();
                    vertices[c[0] as usize].bitangent = (bitangent
                        + cgmath::Vector3::from(vertices[c[0] as usize].bitangent))
                    .into();
                    vertices[c[1] as usize].bitangent = (bitangent
                        + cgmath::Vector3::from(vertices[c[1] as usize].bitangent))
                    .into();
                    vertices[c[2] as usize].bitangent = (bitangent
                        + cgmath::Vector3::from(vertices[c[2] as usize].bitangent))
                    .into();

                    // Used to average the tangents/bitangents
                    triangles_included[c[0] as usize] += 1;
                    triangles_included[c[1] as usize] += 1;
                    triangles_included[c[2] as usize] += 1;
                }

                // Average the tangents/bitangents
                for (i, n) in triangles_included.into_iter().enumerate() {
                    let denom = 1.0 / n as f32;
                    let mut v = &mut vertices[i];
                    v.tangent = (cgmath::Vector3::from(v.tangent) * denom)
                        .normalize()
                        .into();
                    v.bitangent = (cgmath::Vector3::from(v.bitangent) * denom)
                        .normalize()
                        .into();
                }

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", m.name)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", m.name)),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                Ok(Mesh {
                    name: m.name.clone(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: m.mesh.indices.len() as u32,
                    material: m.mesh.material_id.unwrap_or(0),
                })
            })
            .collect::<Result<Vec<Mesh>>>()?;

        Ok(Self { meshes, materials })
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced_with_material(
        &mut self,
        model: &'a Model,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }

    fn draw_model_instanced_with_material(
        &mut self,
        model: &'b Model,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }
}