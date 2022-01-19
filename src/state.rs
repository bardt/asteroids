use crate::{
    camera,
    gamestate::GameState,
    input::Input,
    instance::{Instance, InstanceRaw},
    model::{self, DrawModel, Model, Vertex},
    texture,
};

use cgmath::Rotation3;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::time::Instant;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::{event::WindowEvent, window::Window};

const MINIMUM_FRAME_DURATION_IN_MILLIS: u128 = 16;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding: u32,
    color: [f32; 3],
    radius: f32,
}

pub struct State {
    pub size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    obj_model: Model,
    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,
    last_update: Instant,
    last_render: Instant,
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

        println!(
            "Surface preferred format:\n{:#?}\n",
            surface.get_preferred_format(&adapter)
        );

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

        // CAMERA

        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&gamestate.world.camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&camera::Camera::desc());

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Camera Bind Group"),
        });

        // LIGHT

        let light_uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 0.7, 0.3],
            radius: 10.0,
        };

        // We'll want to update our lights position, so we use COPY_DST
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: None,
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });

        // DEPTH

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "Depth Texture");

        // INSTANCES

        let instance_data = gamestate
            .instances()
            .iter()
            .map(|(_name, instance)| {
                gamestate
                    .world
                    .add_ghost_instances(instance)
                    .par_iter()
                    .map(|instance| Instance::to_raw(instance))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .concat();
        // This buffer will be overridden in `update` to animate instances
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // PIPELINES

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
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

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout, // We don't use it in shader, but specify for uniformity
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("light.wgsl").into()),
            };
            State::create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc()],
                shader,
            )
        };

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let obj_model = model::Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            res_dir.join("assets.obj"),
        )
        .unwrap();

        let now = std::time::Instant::now();

        let input = Input::new();

        Self {
            surface,
            device,
            queue,
            config,
            size,
            gamestate,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            obj_model,
            depth_texture,
            render_pipeline,
            light_uniform,
            light_buffer,
            light_bind_group,
            light_render_pipeline,
            last_update: now,
            last_render: now,
            instance_buffer,
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
            // @TODO: leave black space around world map on resize
            // self.world.resize(&self.config);
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "Depth Texture");
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        /*  Returns a bool to indicate whether an event has been fully processed.
            If the method returns true, the main loop won't process the event any further.
        */
        self.input.process_events(event)
    }

    pub fn update(&mut self) {
        // Time elapsed since last update
        let delta_time = self.last_update.elapsed();
        if delta_time.as_millis() >= MINIMUM_FRAME_DURATION_IN_MILLIS {
            self.last_update = Instant::now();

            self.gamestate
                .control_system(&self.input, &delta_time)
                .lifetime_system(&delta_time)
                .physics_system(&delta_time)
                .collision_system();

            let instance_data = self
                .gamestate
                .instances()
                .par_iter()
                .map(|(_, instance)| {
                    self.gamestate
                        .world
                        .add_ghost_instances(instance)
                        .par_iter()
                        .map(|instance| Instance::to_raw(instance))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
                .concat();

            // @TODO override buffer only when the number of instances changes
            self.instance_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );

            self.camera_uniform
                .update_view_proj(&self.gamestate.world.camera);
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::cast_slice(&[self.camera_uniform]),
            );

            let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
            self.light_uniform.position =
                (cgmath::Quaternion::from_axis_angle((0.0, 0.0, 1.0).into(), cgmath::Deg(0.5))
                    * old_position)
                    .into();
            self.queue.write_buffer(
                &self.light_buffer,
                0,
                bytemuck::cast_slice(&[self.light_uniform]),
            );
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let delta_time = self.last_render.elapsed();
        if delta_time.as_millis() >= MINIMUM_FRAME_DURATION_IN_MILLIS {
            self.last_render = Instant::now();
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
                                g: 0.01,
                                b: 0.01,
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

                render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

                render_pass.set_pipeline(&self.light_render_pipeline);

                render_pass.draw_named_mesh(
                    "Asteroid",
                    &self.obj_model,
                    &self.camera_bind_group,
                    &self.light_bind_group,
                );

                // Render entities
                render_pass.set_pipeline(&self.render_pipeline);

                let mut offset = 0;
                let mut size = 0;
                let mut entity_name = "";
                let instances_per_entity = 9; // because of ghost instances to make world looping

                for entity in &self.gamestate.instances() {
                    if entity_name == "" {
                        entity_name = entity.0;
                    }

                    if entity_name == entity.0 {
                        size += instances_per_entity;
                    } else {
                        render_pass.draw_named_mesh_instanced(
                            entity_name,
                            &self.obj_model,
                            offset..(offset + size),
                            &self.camera_bind_group,
                            &self.light_bind_group,
                        );

                        entity_name = entity.0;
                        offset = size;
                        size = instances_per_entity;
                    }
                }

                render_pass.draw_named_mesh_instanced(
                    entity_name,
                    &self.obj_model,
                    offset..(offset + size),
                    &self.camera_bind_group,
                    &self.light_bind_group,
                );
            }

            self.queue.submit(std::iter::once(encoder.finish()));
            output.present();
        }
        Ok(())
    }
}
