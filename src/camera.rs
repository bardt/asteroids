use model_shader::CameraUniform;
use wgpu::util::DeviceExt;

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub near: f32,
    pub far: f32,
    pub uniform: CameraUniform,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 1. move to position and set rotation of the camera
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // 2. wrap the scene to give effect of depth
        let proj = cgmath::ortho(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        );

        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }

    fn update_uniform(&mut self) {
        self.uniform.update(
            self.eye.to_homogeneous().into(),
            &self.build_view_projection_matrix().into(),
        );
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct CameraBuffer {
    pub uniform: CameraUniform,
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl CameraBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform = CameraUniform::new();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout =
            device.create_bind_group_layout(&shared::camera_bind_group_layout_desc());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            layout: &bind_group_layout,
        });

        Self {
            // Data we want to put into buffer
            uniform,
            buffer,
            bind_group,
        }
    }

    pub fn update_buffer(&mut self, queue: &wgpu::Queue, world_camera: &mut Camera) {
        world_camera.update_uniform();
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[world_camera.uniform]),
        );
    }
}
