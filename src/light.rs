use shared::{LightUniform, LightsUniform};
use wgpu::util::DeviceExt;

pub struct LightsBuffer {
    pub uniform: Vec<LightUniform>,
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl LightsBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform = vec![];

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[LightsUniform::new(&uniform)]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout =
            device.create_bind_group_layout(&shared::wgpu::light_bind_group_layout_desc());

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
            bind_group,
        }
    }

    pub fn update_buffer(&mut self, queue: &wgpu::Queue) {
        let buffer_uniform = &[LightsUniform::new(&self.uniform)];
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(buffer_uniform));
    }
}
