use crate::backdrop::BackdropRenderer;
use crate::light::{self, LightRenderer};
use crate::{
    camera::{self, CameraRenderer},
    debug,
    gamestate::GameState,
    input::Input,
    instance::{Instance, InstanceRaw},
    model::{self, DrawModel, Model, Vertex},
    texture,
};

use cgmath::Rotation3;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::{event::WindowEvent, window::Window};

pub struct State {
    pub size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    camera_renderer: camera::CameraRenderer,
    instance_buffer: wgpu::Buffer,
    instance_buffer_size: usize,
    depth_texture: texture::Texture,
    obj_model: Model,
    light_renderer: light::LightRenderer,
    light_render_pipeline: wgpu::RenderPipeline,
    backdrop_renderer: BackdropRenderer,
    backdrop_render_pipeline: wgpu::RenderPipeline,
    gamestate: GameState,
    input: Input,
}

impl State {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };

        // Adapter is a handle to our graphics card
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        println!("Adapter info:\n{:#?}\n", adapter.get_info());
        println!("Adapter features:\n{:#?}", adapter.features());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::POLYGON_MODE_LINE,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        println!("Device info:\n{:#?}\n", device.limits());
        println!(
            "Surface preferred format:\n{:#?}\n",
            surface.get_preferred_format(&adapter)
        );

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let texture_bind_group_layout = device.create_bind_group_layout(&texture::Texture::desc());

        let aspect = config.width as f32 / config.height as f32;
        let gamestate = GameState::new_game(aspect);

        let mut camera_renderer = CameraRenderer::init(&device);
        camera_renderer
            .uniform
            .update_view_proj(&gamestate.world.camera);

        let light_renderer = LightRenderer::init(&device);

        let backdrop_renderer = BackdropRenderer::init(&device);

        // DEPTH

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "Depth Texture");

        // INSTANCES

        let instance_data = gamestate.instances_raw();
        // This buffer will be overridden in `update` to animate instances
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let instance_buffer_size = (bytemuck::cast_slice(&instance_data) as &[u8]).len();

        // PIPELINES

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_renderer.bind_group_layout,
                    &light_renderer.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            };
            State::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc(), InstanceRaw::desc()],
                shader,
            )
        };

        let light_render_pipeline = light_renderer.pipeline(&device, &config, &camera_renderer);
        let backdrop_render_pipeline = backdrop_renderer.pipeline(&device, &config);

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let obj_model = model::Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            res_dir.join("assets.obj"),
        )
        .unwrap();

        let input = Input::new();

        Self {
            surface,
            device,
            queue,
            config,
            size,
            gamestate,
            camera_renderer,
            obj_model,
            depth_texture,
            render_pipeline,
            light_renderer,
            light_render_pipeline,
            backdrop_renderer,
            backdrop_render_pipeline,
            instance_buffer,
            instance_buffer_size,
            input,
        }
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        shader: wgpu::ShaderModuleDescriptor,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(&shader);

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: color_format,
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
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
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

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "Depth Texture");
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.input.process_events(event)
    }

    pub fn update(&mut self) {
        self.gamestate
            .global_input_system(&self.input)
            .control_system(&self.input)
            .lifetime_system()
            .asteroids_spawn_system()
            .physics_system()
            .collision_system()
            .submit();

        let instance_data = self.gamestate.instances_raw();
        let buffer_contents = bytemuck::cast_slice(&instance_data) as &[u8];

        if buffer_contents.len() > self.instance_buffer_size {
            self.instance_buffer_size = buffer_contents.len();
            debug(&format!(
                "Reallocating buffer for size {:?}",
                self.instance_buffer_size
            ));
            self.instance_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: buffer_contents,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );
        } else {
            self.queue
                .write_buffer(&self.instance_buffer, 0, buffer_contents);
        }

        self.camera_renderer
            .update_buffer(&self.queue, &self.gamestate.world.camera);

        let old_position: cgmath::Vector3<_> = self.light_renderer.uniform.position.into();
        self.light_renderer.uniform.position =
            (cgmath::Quaternion::from_axis_angle((0.0, 0.0, 1.0).into(), cgmath::Deg(0.5))
                * old_position)
                .into();

        self.light_renderer.update_buffer(&self.queue);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // New SurfaceTexture we will render to
        let output = self.surface.get_current_texture()?;

        // TextureView we need to control how the render code interacts with the texture
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Encoder builds a command buffer we can send to the GPU
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        /*
        `encoder.begin_render_pass` borrows `encoder` mutably

        `{}` tells rust to drop variable within the block and this releasing the mutable borrow
        and allowing us to `encoder.finish()`
        */
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            {
                // Persist aspect ratio
                let (world_width, world_height) = self.gamestate.world.size;

                let world_aspect = world_width / world_height;
                let surface_aspect = self.config.width as f32 / self.config.height as f32;

                let (delta_surface_width, delta_surface_height) = if surface_aspect >= world_aspect
                {
                    let expected_surface_width = world_aspect * self.config.height as f32;
                    (self.config.width as f32 - expected_surface_width, 0.)
                } else {
                    let expected_surface_height = self.config.width as f32 / world_aspect;
                    (0., self.config.height as f32 - expected_surface_height)
                };

                render_pass.set_viewport(
                    delta_surface_width / 2.,
                    delta_surface_height / 2.,
                    self.size.width as f32 - delta_surface_width,
                    self.size.height as f32 - delta_surface_height,
                    0.,
                    1.,
                );
            }

            render_pass.set_pipeline(&self.backdrop_render_pipeline);
            self.backdrop_renderer.draw(&mut render_pass);

            // Render light
            render_pass.set_pipeline(&self.light_render_pipeline);
            self.light_renderer.draw_named_mesh(
                "Asteroid_S",
                &self.obj_model,
                &self.camera_renderer.bind_group,
                &mut render_pass,
            );

            // Render entities
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            let mut offset = 0_u32;
            for (name, instances) in self.gamestate.instances_grouped() {
                let size = instances.len() as u32;
                render_pass.draw_named_mesh_instanced(
                    name,
                    &self.obj_model,
                    offset..(offset + size),
                    &self.camera_renderer.bind_group,
                    &self.light_renderer.bind_group,
                );
                offset += size;
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
