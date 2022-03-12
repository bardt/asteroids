use std::time::Instant;

use crate::{
    backdrop::BackdropRenderer,
    camera::{self, CameraBuffer},
    debug,
    gamestate::GameState,
    input::Input,
    light::{self, LightsBuffer},
    model::{self, DrawModel, Model},
    shaders::Shaders,
    texture,
    ui::UI,
};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::{
    event::{KeyboardInput, VirtualKeyCode, WindowEvent},
    window::Window,
};

pub struct State {
    pub size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    shaders: Shaders,
    instance_buffer: wgpu::Buffer,
    instance_buffer_size: usize,
    camera_buffer: camera::CameraBuffer,
    lights_buffer: light::LightsBuffer,
    depth_texture: texture::Texture,
    obj_model: Model,
    backdrop_renderer: BackdropRenderer,
    gamestate: GameState,
    input: Input,
    last_renders: [Instant; 2],
    ui: UI,
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
        let mut gamestate = GameState::new_game(aspect);

        let mut camera_buffer = CameraBuffer::new(&device);
        camera_buffer.update_buffer(&queue, &mut gamestate.world.camera);

        let lights_buffer = LightsBuffer::new(&device);
        let backdrop_renderer = BackdropRenderer::init(&device, &queue);
        

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

        let shaders = Shaders::init(&device, config.format, Some(texture::Texture::DEPTH_FORMAT));

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let obj_model = model::Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            res_dir.join("assets.obj"),
        )
        .unwrap();

        let input = Input::new();
        let last_renders = [Instant::now(), Instant::now()];
        let ui = UI::new(&device);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            gamestate,
            camera_buffer,
            obj_model,
            depth_texture,
            lights_buffer,
            backdrop_renderer,
            instance_buffer,
            instance_buffer_size,
            shaders,
            last_renders,
            input,
            ui,
        }
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
            || match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            ..
                        },
                    ..
                } => match keycode {
                    VirtualKeyCode::N => {
                        let aspect = self.config.width as f32 / self.config.height as f32;
                        self.gamestate = GameState::new_game(aspect);
                        true
                    }

                    _ => false,
                },
                _ => false,
            }
    }

    pub fn update(&mut self) {
        self.gamestate
            .control_system(&self.input)
            .lifetime_system()
            .asteroids_spawn_system()
            .physics_system()
            .collision_system()
            .submit();

        self.ui
            .update(&self.gamestate, self.fps(), &self.device, &self.queue);

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

        self.camera_buffer
            .update_buffer(&self.queue, &mut self.gamestate.world.camera);

        self.lights_buffer.uniform = self.gamestate.light_uniforms();
        self.lights_buffer.update_buffer(&self.queue);
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

            render_pass.set_pipeline(&self.shaders.texture.pipeline);
            self.backdrop_renderer.render(&self.shaders, &mut render_pass);

            // Render entities
            render_pass.set_pipeline(&self.shaders.model.pipeline);
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            let mut offset = 0_u32;
            for (name, instances) in self.gamestate.instances_grouped() {
                let size = instances.len() as u32;
                render_pass.draw_named_mesh_instanced(
                    name,
                    &self.obj_model,
                    offset..(offset + size),
                    &self.camera_buffer,
                    &self.lights_buffer,
                );
                offset += size;
            }

            self.ui.render(&self.shaders, &mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.last_renders[1] = self.last_renders[0];
        self.last_renders[0] = Instant::now();

        Ok(())
    }

    fn fps(&self) -> u128 {
        let [last, previous] = self.last_renders;
        if last > previous {
            let delta_time = (last - previous).as_millis();

            if delta_time > 0 {
                1000 / delta_time
            } else {
                0
            }
        } else {
            0
        }
    }
}
