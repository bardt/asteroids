use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    position: [f32; 4],
    color: [f32; 4],
    radius: [f32; 4],
}

impl LightUniform {
    pub fn new(position: [f32; 3], color: [f32; 3], radius: f32) -> Self {
        Self {
            position: [position[0], position[1], position[2], 0.],
            color: [color[0], color[1], color[2], 0.],
            radius: [radius, 0., 0., 0.],
        }
    }

    fn empty() -> Self {
        Self::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.0)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightBuffer {
    data: [LightUniform; 16],
    size: u32,
    _padding: [u32; 3],
}

impl LightBuffer {
    fn new(lights: &[LightUniform]) -> Self {
        let mut data = [LightUniform::empty(); 16];
        for i in 0..lights.len().min(16) {
            data[i] = lights[i];
        }

        Self {
            data,
            size: lights.len() as u32,
            _padding: [0, 0, 0],
        }
    }
}

pub struct LightRenderer {
    pub uniform: Vec<LightUniform>,
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl LightRenderer {
    pub fn init(device: &wgpu::Device) -> Self {
        let uniform = vec![];

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[LightBuffer::new(&uniform)]),
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
        let buffer_uniform = &[LightBuffer::new(&self.uniform)];
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(buffer_uniform));
    }
}
