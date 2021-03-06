use wgpu::PipelineLayout;

pub fn layout(device: &wgpu::Device) -> PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Model Shader Pipeline Layout"),
        bind_group_layouts: &[
            &device.create_bind_group_layout(&shared::wgpu::texture_bind_group_layout_desc()),
            &device.create_bind_group_layout(&shared::wgpu::camera_bind_group_layout_desc()),
            &device.create_bind_group_layout(&shared::wgpu::light_bind_group_layout_desc()),
        ],
        push_constant_ranges: &[],
    })
}
