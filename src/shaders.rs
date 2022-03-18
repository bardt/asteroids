use crate::instance::InstanceRaw;
use crate::model::{self, Vertex};
use crate::texture::TextureVertex;
use texture_shader;
use wgpu;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShaderName {
    Model,
    Texture,
}

pub struct Shader {
    pub pipeline: wgpu::RenderPipeline,
}

pub struct Shaders {
    pub texture: Shader,
    pub model: Shader,
}

impl Shaders {
    pub fn init(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let texture = {
            let module = wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::SpirV(wgpu::util::make_spirv_raw(include_bytes!(
                    env!("texture_shader.spv")
                ))),
            };

            // @TODO: mode vertext layouts closer to the shader code
            let vertex_layouts = &[TextureVertex::desc()];

            let pipeline = create_render_pipeline(
                device,
                "Texture Render Pipeline",
                &texture_shader::pipeline::layout(device),
                color_format,
                depth_format,
                vertex_layouts,
                wgpu::CompareFunction::Always,
                module,
            );

            Shader { pipeline }
        };

        let model = {
            let module = wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::SpirV(wgpu::util::make_spirv_raw(include_bytes!(
                    env!("model_shader.spv")
                ))),
            };

            let vertex_layouts = &[model::ModelVertex::desc(), InstanceRaw::desc()];

            let pipeline = create_render_pipeline(
                device,
                "Model Render Pipeline",
                &model_shader::pipeline::layout(device),
                color_format,
                depth_format,
                vertex_layouts,
                wgpu::CompareFunction::Less,
                module,
            );

            Shader { pipeline }
        };

        Self { texture, model }
    }

    pub fn by_name(&self, name: ShaderName) -> &Shader {
        match name {
            ShaderName::Model => &self.model,
            ShaderName::Texture => &self.texture,
        }
    }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    depth_compare: wgpu::CompareFunction,
    shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(&shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "main_vs",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "main_fs",
            targets: &[wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
            conservative: false,
            unclipped_depth: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            // Has to do with anti-aliasing
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}
