use wgpu::util::DeviceExt;

use crate::{
    camera,
    model::{self, Model, Vertex},
    texture,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding: u32,
    pub color: [f32; 3],
    pub radius: f32,
}

impl LightUniform {
    pub fn new(position: [f32; 3], color: [f32; 3], radius: f32) -> Self {
        Self {
            position,
            _padding: 0,
            color,
            radius,
        }
    }
}

pub struct LightRenderer {
    pub uniform: LightUniform,
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl LightRenderer {
    pub fn init(device: &wgpu::Device) -> Self {
        let uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 0.7, 0.3],
            radius: 10.0,
        };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            layout: &bind_group_layout,
        });

        Self {
            uniform,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn update_buffer(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn pipeline(
        &self,
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        camera_renderer: &camera::CameraRenderer,
    ) -> wgpu::RenderPipeline {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Light Render Pipeline Layout"),
            bind_group_layouts: &[&camera_renderer.bind_group_layout, &self.bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Light Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("light.wgsl").into()),
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Light Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[model::ModelVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLAMPING
                clamp_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                // Has to do with anti-aliasing
                alpha_to_coverage_enabled: false,
            },
        })
    }

    // @TODO: specify instance data; now we rely on set_vertex_buffer in slot 1 outside of this function
    pub fn draw_named_mesh<'a, 'b>(
        &'b self,
        name: &str,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) where
        'b: 'a,
    {
        if let Some(mesh) = model.meshes.iter().find(|mesh| mesh.name == name) {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.bind_group, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }
}
